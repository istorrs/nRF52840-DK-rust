use defmt::*;
use embassy_executor::task;
use embassy_nrf::{peripherals, radio};
use embassy_time::{Duration, Timer};
use nrf_softdevice::ble::{gatt_server, peripheral};
use nrf_softdevice::{raw, Softdevice};
use heapless::Vec;

// Custom BLE service UUID for data collection
const DATA_SERVICE_UUID: u16 = 0x1234;
const DATA_CHARACTERISTIC_UUID: u16 = 0x5678;
const STATUS_CHARACTERISTIC_UUID: u16 = 0x9ABC;

#[nrf_softdevice::gatt_server]
pub struct DataServer {
    pub data_service: DataService,
}

#[nrf_softdevice::gatt_service(uuid = "6e400001-b5a3-f393-e0a9-e50e24dcca9e")]
pub struct DataService {
    #[characteristic(uuid = "6e400002-b5a3-f393-e0a9-e50e24dcca9e", read, write, notify)]
    pub data_rx: Vec<u8, 64>,
    
    #[characteristic(uuid = "6e400003-b5a3-f393-e0a9-e50e24dcca9e", read, write, notify)]
    pub data_tx: Vec<u8, 64>,
    
    #[characteristic(uuid = "6e400004-b5a3-f393-e0a9-e50e24dcca9e", read)]
    pub status: u8,
}

#[task]
pub async fn ble_task(
    _radio: peripherals::RADIO,
    _timer: peripherals::TIMER0,
) {
    info!("Starting BLE task");
    
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
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 128 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 1,
            central_role_count: 0,
            central_sec_count: 0,
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"nRF52840-DK\0" as *const u8 as _,
            current_len: 11,
            max_len: 11,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_align_1: [],
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    let sd = Softdevice::enable(&config);
    let server = DataServer::new(sd).unwrap();
    
    info!("BLE SoftDevice initialized");

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data: &[
                0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
                0x03, 0x03, 0x09, 0x18,
                0x0a, 0x09, b'n', b'R', b'F', b'5', b'2', b'8', b'4', b'0', b'-', b'D', b'K',
            ],
            scan_data: &[
                0x03, 0x03, 0x09, 0x18,
            ],
        };
        
        info!("Starting BLE advertising");
        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &config).await);
        info!("BLE device connected");

        let res = gatt_server::run(&conn, &server, |e| match e {
            DataServerEvent::DataService(e) => match e {
                DataServiceEvent::DataRxWrite(val) => {
                    info!("Received data: {:?}", val);
                    // Process received data here
                }
                DataServiceEvent::DataTxCccdWrite { notifications } => {
                    info!("Notifications enabled: {}", notifications);
                }
                DataServiceEvent::DataRxCccdWrite { notifications: _ } => {}
            }
        }).await;

        if let Err(e) = res {
            info!("GATT server error: {:?}", e);
        }
        
        info!("BLE connection lost, restarting advertising");
        Timer::after(Duration::from_secs(1)).await;
    }
}