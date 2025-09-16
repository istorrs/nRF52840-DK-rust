#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use nrf_softdevice::ble::central;
use nrf_softdevice::{raw, Softdevice};
use {defmt_rtt as _, panic_halt as _};

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("=== nRF52840-DK BLE Scanner Starting ===");

    // Configure SoftDevice with the same config as before
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

    info!("Starting BLE scan...");

    let config = central::ScanConfig::default();
    let res = central::scan(sd, &config, |params| {
        info!(
            "Advertisement: addr={:?} addr_type={:?} connectable={} scan_rsp={}",
            params.peer_addr.addr,
            params.peer_addr.addr_type(),
            params.type_.connectable(),
            params.type_.scan_response(),
        );

        // Simple data dump without private util function
        info!("  Data length: {}", params.data.len);
        if params.data.len > 0 {
            let data_len = params.data.len as usize;
            let slice_len = data_len.min(8);
            unsafe {
                let data_slice = core::slice::from_raw_parts(params.data.p_data, slice_len);
                info!("  First bytes: {:02x}", data_slice);
            }
        }

        None
    })
    .await;
    unwrap!(res);
    info!("Scan complete");
}