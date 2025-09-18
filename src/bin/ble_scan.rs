#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
#[allow(clippy::single_component_path_imports)]
use embassy_nrf;
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::central;
use nrf_softdevice::{raw, Softdevice};
use {defmt_rtt as _, panic_halt as _};

// SoftDevice task
#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

// BLE scanning task (separate from main to allow proper timing)
#[embassy_executor::task]
async fn ble_scan_task(sd: &'static Softdevice) {
    info!("Starting BLE scan task...");

    Timer::after(Duration::from_secs(2)).await; // Give SoftDevice time to fully initialize

    info!("Beginning BLE scan...");
    let config = central::ScanConfig {
        timeout: 30, // 30 second timeout
        ..Default::default()
    };
    let res = central::scan(sd, &config, |params| {
        // Keep callback minimal to avoid blocking interrupts
        info!(
            "BLE Device: addr={:?} connectable={} data_len={}",
            params.peer_addr.addr,
            params.type_.connectable(),
            params.data.len
        );

        None::<()>
    })
    .await;

    match res {
        Ok(_) => info!("BLE scan completed successfully"),
        Err(e) => info!("BLE scan finished: {:?}", e),
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=== nRF52840-DK BLE Scanner Starting ===");

    // Initialize Embassy FIRST with SoftDevice-compatible settings
    info!("Initializing Embassy...");
    let mut embassy_config = embassy_nrf::config::Config::default();
    embassy_config.gpiote_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    embassy_config.time_interrupt_priority = embassy_nrf::interrupt::Priority::P2;
    let _p = embassy_nrf::init(embassy_config);
    info!("✅ Embassy initialized");

    // Configure SoftDevice with the same config as working BLE+GPIO app
    let config = nrf_softdevice::Config {
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
            p_value: b"nRF52840-DK" as *const u8 as _,
            current_len: 11,
            max_len: 11,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    info!("Enabling SoftDevice...");
    let sd = Softdevice::enable(&config);
    info!("✅ SoftDevice enabled successfully!");

    info!("Spawning SoftDevice task...");
    unwrap!(spawner.spawn(softdevice_task(sd)));
    info!("✅ SoftDevice task spawned");

    info!("Spawning BLE scan task...");
    unwrap!(spawner.spawn(ble_scan_task(sd)));
    info!("✅ BLE scan task spawned");

    info!("All systems operational - BLE scanner ready!");

    // Main loop - keep the app alive
    loop {
        info!("BLE Scanner running...");
        Timer::after(Duration::from_millis(10000)).await;
    }
}
