use defmt::*;
use embassy_executor::task;
use embassy_time::{Duration, Timer};
use heapless::Vec;
use nrf_softdevice::ble::{gatt_server, peripheral, Connection};
use nrf_softdevice::{raw, Softdevice};

// Nordic UART Service (NUS) for phone connectivity
#[nrf_softdevice::gatt_service(uuid = "6e400001-b5a3-f393-e0a9-e50e24dcca9e")]
pub struct NordicUartService {
    #[characteristic(
        uuid = "6e400002-b5a3-f393-e0a9-e50e24dcca9e",
        write_without_response,
        write
    )]
    rx: Vec<u8, 64>,

    #[characteristic(uuid = "6e400003-b5a3-f393-e0a9-e50e24dcca9e", notify)]
    tx: Vec<u8, 64>,
}

// Custom sensor data service
#[nrf_softdevice::gatt_service(uuid = "12345678-1234-5678-9abc-123456789abc")]
pub struct SensorService {
    #[characteristic(uuid = "12345678-1234-5678-9abc-123456789abd", read, notify)]
    temperature: i16,

    #[characteristic(uuid = "12345678-1234-5678-9abc-123456789abe", read, notify)]
    button_state: u8,
}

#[nrf_softdevice::gatt_server]
pub struct BluetoothServer {
    nordic_uart: NordicUartService,
    sensor: SensorService,
}

#[embassy_executor::task]
pub async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[task]
pub async fn ble_task(sd: &'static Softdevice, server: &'static BluetoothServer) {
    info!("Starting BLE task with SoftDevice");

    loop {
        let config = peripheral::Config::default();

        // Advertisement data
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data: &[
                0x02,
                0x01,
                raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
                0x03,
                0x03,
                0x09,
                0x18, // 16-bit service UUID list
                0x0c,
                0x09,
                b'n',
                b'R',
                b'F',
                b'5',
                b'2',
                b'8',
                b'4',
                b'0',
                b'-',
                b'D',
                b'K', // Complete local name
            ],
            scan_data: &[
                0x03, 0x03, 0x09, 0x18, // Additional service UUIDs
            ],
        };

        info!("BLE advertising started - waiting for connection");

        match peripheral::advertise_connectable(sd, adv, &config).await {
            Ok(conn) => {
                info!("BLE device connected!");
                handle_connection(&conn, server).await;
                info!("BLE device disconnected");
            }
            Err(e) => {
                warn!("Failed to advertise: {:?}", e);
                Timer::after(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn handle_connection(conn: &Connection, server: &BluetoothServer) {
    let _disconnected = gatt_server::run(conn, server, |e| match e {
        BluetoothServerEvent::NordicUart(e) => match e {
            NordicUartServiceEvent::RxWrite(data) => {
                info!(
                    "UART RX received {} bytes: {:?}",
                    data.len(),
                    data.as_slice()
                );

                // Echo the data back (simple example)
                if let Ok(_echo_msg) = Vec::<u8, 64>::from_slice(data.as_slice()) {
                    // In a real app, you'd process the data and send meaningful responses
                    info!("Echoing data back to phone");
                }
            }
            NordicUartServiceEvent::TxCccdWrite { notifications } => {
                info!("UART TX notifications enabled: {}", notifications);
            }
        },
        BluetoothServerEvent::Sensor(e) => match e {
            SensorServiceEvent::TemperatureCccdWrite { notifications } => {
                info!("Temperature notifications enabled: {}", notifications);
            }
            SensorServiceEvent::ButtonStateCccdWrite { notifications } => {
                info!("Button state notifications enabled: {}", notifications);
            }
        },
    })
    .await;
}

#[allow(dead_code)]
pub async fn send_sensor_data(
    _conn: &Connection,
    _server: &BluetoothServer,
    temperature: i16,
    button_pressed: bool,
) {
    // This function can be called from GPIO tasks to update BLE characteristics
    info!(
        "Sensor data updated - Temp: {}°C, Button: {}",
        temperature,
        if button_pressed {
            "pressed"
        } else {
            "released"
        }
    );

    // In a real implementation, you would update the GATT characteristics here
    // server.sensor.temperature_set(temperature);
    // server.sensor.button_state_set(if button_pressed { 1 } else { 0 });
}
