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

// Defmt timestamp provider using embassy-time
defmt::timestamp!("{=u64:us}", { embassy_time::Instant::now().as_micros() });

// Import our CLI modules
use nrf52840_dk_template::cli::{CliError, CommandHandler, Terminal};

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
            p_value: b"nRF52840-DK CLI" as *const u8 as _,
            current_len: 15,
            max_len: 15,
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

    // Configure LED3 (P0.15) and LED4 (P0.16) for CLI commands
    let led3 = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);

    // Configure Buttons (P0.11, P0.12, P0.24, P0.25) for CLI commands
    // Buttons are active low, so we use internal pull-up resistors
    let button1 = Input::new(p.P0_11, Pull::Up);
    let button2 = Input::new(p.P0_12, Pull::Up);
    let button3 = Input::new(p.P0_24, Pull::Up);
    let button4 = Input::new(p.P0_25, Pull::Up);

    // Configure MTU pins (P0.02 for clock out, P0.03 for data in)
    let mtu_clock_pin = Output::new(p.P0_02, Level::Low, OutputDrive::Standard);
    let mtu_data_pin = Input::new(p.P0_03, Pull::None); // No pull resistor - meter drives the line

    // Configure UART for CLI
    let mut uart_config = uarte::Config::default();
    uart_config.parity = uarte::Parity::EXCLUDED;
    uart_config.baudrate = uarte::Baudrate::BAUD115200;

    let uarte = Uarte::new(p.UARTE1, Irqs, p.P1_14, p.P1_15, uart_config);
    info!("✅ Peripherals configured");

    // Configure MTU debug LEDs (use onboard LEDs)
    // LED1 (P0.13) is already used for UART RX, LED2 (P0.14) for UART TX
    // Use LED3 (P0.15) for clock debug, LED4 (P0.16) for data debug
    let led_clock = led3; // LED3 for clock activity
    let led_data = led4; // LED4 for data activity

    // Initialize CLI components with LEDs, buttons, MTU, and SoftDevice
    let mut terminal = Terminal::new(uarte).with_tx_led(led2);
    let mut command_handler = CommandHandler::new()
        .with_buttons(button1, button2, button3, button4)
        .with_mtu(mtu_clock_pin, mtu_data_pin)
        .with_mtu_debug_leds(led_clock, led_data)
        .with_softdevice(sd);

    // Send welcome message
    let _ = terminal.write_line("").await;
    let _ = terminal.write_line("Water Meter MTU Interface").await;
    let _ = terminal
        .write_line("Type 'help' for available commands")
        .await;
    let _ = terminal
        .write_line("Use TAB for command autocompletion")
        .await;
    let _ = terminal.write_line("MTU Clock: P0.02 | Data: P0.03").await;
    let _ = terminal
        .write_line("Debug LEDs: LED3 (clock) | LED4 (data)")
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
                            nrf52840_dk_template::cli::parser::CommandParser::parse_command(
                                &command_line,
                            );

                        // Clone command for later pattern matching
                        let command_clone = command.clone();

                        match command_handler.execute_command(command).await {
                            Ok(response) => {
                                // Only write response if it's not empty
                                if !response.is_empty() {
                                    let _ = terminal.write_line(&response).await;
                                }
                            }
                            Err(CliError::InvalidCommand) => {
                                let _ = terminal
                                    .write_line(
                                        "Invalid command. Type 'help' for available commands.",
                                    )
                                    .await;
                            }
                            Err(_) => {
                                let _ = terminal.write_line("Command execution error.").await;
                            }
                        }

                        // Handle special commands that need terminal interaction
                        match command_clone {
                            nrf52840_dk_template::cli::CliCommand::Help => {
                                let _ = terminal.show_help().await;
                            }
                            nrf52840_dk_template::cli::CliCommand::Clear => {
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
