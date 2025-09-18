#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::central;
use nrf_softdevice::{raw, Softdevice};
use {defmt_rtt as _, panic_halt as _};

// GPIO tasks
#[embassy_executor::task]
async fn heartbeat_task(mut led: Output<'static>) {
    info!("Starting heartbeat task");
    loop {
        led.set_low();
        Timer::after(Duration::from_millis(100)).await;
        led.set_high();
        Timer::after(Duration::from_millis(900)).await;
    }
}

#[embassy_executor::task]
async fn button_handler_task(button: Input<'static>, mut led: Output<'static>) {
    info!("Starting button handler task");
    info!(
        "Button initial state: {}",
        if button.is_low() { "LOW" } else { "HIGH" }
    );

    let mut last_state = button.is_high(); // true when not pressed (pull-up)

    loop {
        let current_state = button.is_high();

        // Button pressed (high to low transition)
        if last_state && !current_state {
            info!("Button pressed!");
            led.set_low(); // Turn on LED (active low)
        }
        // Button released (low to high transition)
        else if !last_state && current_state {
            info!("Button released!");
            led.set_high(); // Turn off LED
        }

        last_state = current_state;

        // Poll every 10ms for responsive button handling
        Timer::after(Duration::from_millis(10)).await;
    }
}

#[embassy_executor::task]
async fn led_pattern_task(mut led3: Output<'static>, mut led4: Output<'static>) {
    info!("Starting LED pattern task");
    loop {
        // Pattern: LED3 and LED4 alternating
        led3.set_low();
        led4.set_high();
        Timer::after(Duration::from_millis(300)).await;

        led3.set_high();
        led4.set_low();
        Timer::after(Duration::from_millis(300)).await;
    }
}

// SoftDevice task
#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

// BLE scanning task
#[embassy_executor::task]
async fn ble_scan_task(sd: &'static Softdevice) {
    info!("Starting BLE scan task...");

    Timer::after(Duration::from_secs(2)).await; // Give other tasks time to start

    let config = central::ScanConfig::default();
    let res = central::scan(sd, &config, |params| {
        info!(
            "BLE Device: addr={:?} connectable={}",
            params.peer_addr.addr,
            params.type_.connectable(),
        );
        None
    })
    .await;
    unwrap!(res);
    info!("BLE scan complete");
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=== nRF52840-DK Combined BLE + GPIO App ===");

    // Initialize Embassy FIRST with SoftDevice-compatible settings
    info!("Initializing Embassy with SoftDevice-compatible settings...");
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    let p = embassy_nrf::init(config);
    info!("✅ Embassy initialized");

    // Configure SoftDevice AFTER Embassy initialization
    info!("Configuring SoftDevice...");
    let sd_config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 16,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 6,
            event_length: 6,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: raw::BLE_GATTS_ATTR_TAB_SIZE_DEFAULT,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 3,
            central_role_count: 3,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"nRF52840-DK-GPIO" as *const u8 as _,
            current_len: 16,
            max_len: 16,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    info!("Enabling SoftDevice...");
    let sd = Softdevice::enable(&sd_config);
    info!("✅ SoftDevice enabled");

    // Spawn SoftDevice task
    unwrap!(spawner.spawn(softdevice_task(sd)));
    info!("✅ SoftDevice task spawned");

    // Configure GPIO pins
    info!("Configuring GPIO pins...");
    let led1 = Output::new(p.P0_13, Level::High, OutputDrive::Standard);
    let led2 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);
    let btn1 = Input::new(p.P0_11, Pull::Up);
    info!("✅ GPIO pins configured");

    // Spawn GPIO tasks
    info!("Spawning GPIO tasks...");
    unwrap!(spawner.spawn(heartbeat_task(led1)));
    unwrap!(spawner.spawn(button_handler_task(btn1, led2)));
    unwrap!(spawner.spawn(led_pattern_task(led3, led4)));
    info!("✅ GPIO tasks spawned");

    // Spawn BLE scan task
    info!("Spawning BLE scan task...");
    unwrap!(spawner.spawn(ble_scan_task(sd)));
    info!("✅ BLE scan task spawned");

    info!("All systems operational - BLE + GPIO + RTT!");

    // Main loop
    loop {
        info!("Combined app running: BLE scanning + GPIO tasks + RTT working!");
        Timer::after(Duration::from_millis(10000)).await;
    }
}
