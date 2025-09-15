#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pull};
use embassy_time::{Duration, Timer};
use nrf_softdevice::{raw, Softdevice};
use {defmt_rtt as _, panic_halt as _};

use core::mem::MaybeUninit;

mod ble_task;
mod gpio_tasks;

use ble_task::*;
use gpio_tasks::*;

static mut SERVER_STORAGE: MaybeUninit<BluetoothServer> = MaybeUninit::uninit();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("nRF52840-DK Embassy Template Starting!");

    let p = embassy_nrf::init(Default::default());

    // Configure SoftDevice
    let config = nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 16,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 1,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 1,
            central_role_count: 0,
            central_sec_count: 0,
            _bitfield_1: Default::default(),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"nRF52840-DK\0" as *const u8 as _,
            current_len: 11,
            max_len: 11,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    let sd = Softdevice::enable(&config);
    let server = unsafe {
        SERVER_STORAGE.write(unwrap!(BluetoothServer::new(sd)));
        SERVER_STORAGE.assume_init_ref()
    };

    info!("SoftDevice and BLE server initialized");

    // Spawn SoftDevice task (required for BLE operation)
    unwrap!(spawner.spawn(softdevice_task(sd)));

    // Configure GPIO pins for nRF52840-DK
    // LEDs: P0.13, P0.14, P0.15, P0.16 (active low)
    let led1 = Output::new(p.P0_13, Level::High, OutputDrive::Standard);
    let led2 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);

    // Buttons: P0.11, P0.12, P0.24, P0.25 (active low with internal pull-up)
    let btn1 = Input::new(p.P0_11, Pull::Up);
    let _btn2 = Input::new(p.P0_12, Pull::Up);
    let _btn3 = Input::new(p.P0_24, Pull::Up);
    let _btn4 = Input::new(p.P0_25, Pull::Up);

    // Spawn async tasks
    unwrap!(spawner.spawn(heartbeat_task(led1)));
    unwrap!(spawner.spawn(button_handler_task(btn1, led2)));
    unwrap!(spawner.spawn(led_pattern_task(led3, led4)));
    unwrap!(spawner.spawn(ble_task(sd, server)));

    info!("All tasks spawned successfully");

    // Main loop - can be used for other tasks or just sleep
    loop {
        info!("Main loop iteration - system running with BLE");
        Timer::after(Duration::from_secs(30)).await;
    }
}