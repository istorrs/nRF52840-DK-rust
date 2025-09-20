#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    uarte::{self, Uarte},
};
use embassy_time::{Duration, Timer};
use nrf_softdevice::{raw, Softdevice};
use {defmt_rtt as _, panic_halt as _};

// Import our CLI modules
use nrf52840_dk_template::cli::{CliError, Terminal};
use nrf52840_dk_template::meter::{MeterHandler, MeterConfig, MeterType};

bind_interrupts!(struct Irqs {
    UARTE1 => embassy_nrf::uarte::InterruptHandler<embassy_nrf::peripherals::UARTE1>;
});

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
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
            p_value: b"nRF52840-DK Meter" as *const u8 as _,
            current_len: 17,
            max_len: 17,
            write_perm: unsafe { core::mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    };

    let sd = Softdevice::enable(&sd_config);
    let _ = spawner.spawn(softdevice_task(sd));
    info!("✅ SoftDevice enabled and task spawned");

    // Configure peripherals AFTER SoftDevice is enabled
    info!("Configuring peripherals...");

    // Configure LED1 (P0.13) for UART RX activity indication
    let mut led1 = Output::new(p.P0_13, Level::High, OutputDrive::Standard);

    // Configure LED2 (P0.14) for UART TX activity indication
    let led2 = Output::new(p.P0_14, Level::High, OutputDrive::Standard);

    // Configure LED3 (P0.15) for meter activity
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);

    // Configure Meter pins - P0.02 for clock input (from MTU), P0.03 for data output (to MTU)
    let meter_clock_pin = Input::new(p.P0_02, Pull::Up); // Clock input from MTU
    let meter_data_pin = Output::new(p.P0_03, Level::High, OutputDrive::Standard); // Data output to MTU

    // Configure UART for CLI
    let mut uart_config = uarte::Config::default();
    uart_config.parity = uarte::Parity::EXCLUDED;
    uart_config.baudrate = uarte::Baudrate::BAUD115200;

    let uarte = Uarte::new(p.UARTE1, Irqs, p.P1_14, p.P1_15, uart_config);
    info!("✅ Peripherals configured");

    // Initialize CLI components with meter functionality
    let mut terminal = Terminal::new(uarte).with_tx_led(led2);
    let mut meter_handler = MeterHandler::new(
        MeterConfig::default(),
        meter_clock_pin,
        meter_data_pin,
        led3,
    );

    // Send welcome message
    let _ = terminal.write_line("").await;
    let _ = terminal.write_line("Water Meter Simulator Interface").await;
    let _ = terminal
        .write_line("Type 'help' for available commands")
        .await;
    let _ = terminal
        .write_line("Use TAB for command autocompletion")
        .await;
    let _ = terminal
        .write_line("Meter Clock: P0.02 (in) | Data: P0.03 (out)")
        .await;
    let _ = terminal.print_prompt().await;

    // Main CLI loop
    loop {
        let mut single_byte = [0u8; 1];

        match terminal.uart.read(&mut single_byte).await {
            Ok(_) => {
                // Flash LED1 briefly on UART RX activity
                led1.set_low();
                let ch = single_byte[0];

                // Small delay to make flash visible, then turn off RX LED
                Timer::after(Duration::from_millis(10)).await;
                led1.set_high();

                // Handle character and check if we got a complete command
                match terminal.handle_char(ch).await {
                    Ok(Some(command_line)) => {
                        // Parse and execute the command
                        let command =
                            nrf52840_dk_template::meter::parser::MeterCommandParser::parse_command(
                                &command_line,
                            );

                        // Clone command for later pattern matching
                        let command_clone = command.clone();

                        match meter_handler.execute_command(command).await {
                            Ok(response) => {
                                // Only write response if it's not empty
                                if !response.is_empty() {
                                    let _ = terminal.write_line(&response).await;
                                }
                            }
                            Err(_) => {
                                let _ = terminal.write_line("Command execution error.").await;
                            }
                        }

                        // Handle special commands that need terminal interaction
                        match command_clone {
                            nrf52840_dk_template::meter::MeterCommand::Help => {
                                let _ = terminal.show_meter_help().await;
                            }
                            nrf52840_dk_template::meter::MeterCommand::Clear => {
                                let _ = terminal.clear_screen().await;
                            }
                            _ => {}
                        }

                        let _ = terminal.print_prompt().await;
                    }
                    Ok(None) => {
                        // Character processed but no complete command yet
                    }
                    Err(_) => {
                        // Handle error
                        let _ = terminal.write_line("Input error").await;
                        let _ = terminal.print_prompt().await;
                    }
                }
            }
            Err(_) => {
                // No LED activity on UART read error (no data received)
                // Continue on error
            }
        }
    }
}