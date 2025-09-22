#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Level, Output, OutputDrive, Pull},
    uarte::{self, Uarte},
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Instant, Timer};
use nrf_softdevice::{raw, Softdevice};
use {defmt_rtt as _, panic_halt as _};

// Defmt timestamp provider using embassy-time
defmt::timestamp!("{=u64:us}", { embassy_time::Instant::now().as_micros() });

// Import our CLI modules
use nrf52840_dk_template::cli::Terminal;
use nrf52840_dk_template::meter::{MeterCommandParser, MeterConfig, MeterHandler};

bind_interrupts!(struct Irqs {
    UARTE1 => embassy_nrf::uarte::InterruptHandler<embassy_nrf::peripherals::UARTE1>;
});

// Communication structure between fast clock response and LED/logging tasks
#[derive(Clone, Copy)]
struct ClockEvent {
    pulse_count: u32,
    bit_index: usize,
    bit_value: u8,
    transmitting: bool,
    time_delta_micros: u64,
}

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

// Fast clock response task - only handles critical timing and data pin
#[embassy_executor::task]
async fn fast_clock_response_task(
    mut clock_pin: Input<'static>,
    mut data_pin: Output<'static>,
    meter_handler: &'static MeterHandler<'static>,
    event_sender: embassy_sync::channel::Sender<'static, ThreadModeRawMutex, ClockEvent, 32>,
) -> ! {
    info!("Fast clock response task started");

    let mut pulse_count = 0u32;
    let wake_up_threshold = 10; // Pulses to consider start of transmission
    let pulse_timeout = Duration::from_millis(2000); // Support MTU rates as low as 1 bps
    let mut last_pulse_time = Instant::now();
    let mut last_log_time = Instant::now();

    // Pre-build response frame buffer to avoid async delays during transmission
    let mut response_bits = meter_handler.build_response_frames().await;
    let mut bit_index = 0;
    let mut transmitting = false;

    loop {
        // Wait for rising edge of clock - this is the critical timing
        clock_pin.wait_for_rising_edge().await;
        let now = Instant::now();
        let time_delta = now.duration_since(last_log_time);
        last_log_time = now;

        // Check timeout immediately
        let time_since_last = now.duration_since(last_pulse_time);
        if time_since_last > pulse_timeout && transmitting {
            warn!(
                "Meter: Transmission timeout after {}ms, stopping transmission at bit {}",
                time_since_last.as_millis(),
                bit_index
            );
            transmitting = false;
            bit_index = 0;
            pulse_count = 0;
        }

        pulse_count += 1;
        last_pulse_time = now;

        // Check if we should start transmitting (after wake-up sequence)
        if !transmitting && pulse_count >= wake_up_threshold {
            if response_bits.is_empty() {
                warn!("Meter: Response buffer empty, rebuilding response frames");
                response_bits = meter_handler.build_response_frames().await;
            }
            transmitting = true;
            bit_index = 0;

            // Pre-set the data pin for the first bit (start bit) immediately
            if !response_bits.is_empty() {
                let bit = response_bits[0];
                if bit == 1 {
                    data_pin.set_high();
                } else {
                    data_pin.set_low();
                }
                bit_index = 1; // We've already set bit 0

                // Send event for logging
                let event = ClockEvent {
                    pulse_count,
                    bit_index: 1,
                    bit_value: bit,
                    transmitting: true,
                    time_delta_micros: time_delta.as_micros(),
                };
                let _ = event_sender.try_send(event);
            }

            // Return early to avoid double-logging on the start transmission pulse
            continue;
        }

        // If transmitting, send the next bit on each clock pulse
        if transmitting && bit_index < response_bits.len() {
            let bit = response_bits[bit_index];

            // Set data pin immediately - this is the critical operation
            if bit == 1 {
                data_pin.set_high();
            } else {
                data_pin.set_low();
            }

            bit_index += 1;

            // Send event for logging
            let event = ClockEvent {
                pulse_count,
                bit_index,
                bit_value: bit,
                transmitting: true,
                time_delta_micros: time_delta.as_micros(),
            };
            let _ = event_sender.try_send(event);

            // If we've sent all bits, stop transmitting and reset for next cycle
            if bit_index >= response_bits.len() {
                transmitting = false;
                bit_index = 0;
                pulse_count = 0; // Reset pulse count to require new wake-up sequence
                data_pin.set_high(); // Return to idle state
            }
        } else if !transmitting {
            // Send event for idle pulses
            let event = ClockEvent {
                pulse_count,
                bit_index: 0,
                bit_value: 0,
                transmitting: false,
                time_delta_micros: time_delta.as_micros(),
            };
            let _ = event_sender.try_send(event);
        }
    }
}

// LED and logging task - handles non-critical operations
#[embassy_executor::task]
async fn led_logging_task(
    mut clock_led: Output<'static>,
    mut activity_led: Output<'static>,
    event_receiver: embassy_sync::channel::Receiver<'static, ThreadModeRawMutex, ClockEvent, 32>,
) -> ! {
    info!("LED and logging task started");

    loop {
        // Wait for events from the fast clock response task
        let event = event_receiver.receive().await;

        let time_delta_micros = event.time_delta_micros;

        // Flash LED4 briefly for each clock edge
        clock_led.set_low(); // LED on
        Timer::after(Duration::from_millis(5)).await;
        clock_led.set_high(); // LED off

        if event.transmitting {
            // Set activity LED on during transmission
            activity_led.set_low();

            // Calculate which character and bit position we're sending
            let char_index = (event.bit_index - 1) / 10; // 10 bits per character for 7E1
            let bit_in_char = (event.bit_index - 1) % 10 + 1;

            info!(
                "METER: CLK #{} TICK {} - TX bit #{} value {} [char #{}, bit #{}]",
                event.pulse_count,
                time_delta_micros,
                event.bit_index,
                event.bit_value,
                char_index + 1,
                bit_in_char
            );
        } else {
            // Set activity LED off when not transmitting
            activity_led.set_high();

            info!(
                "METER: CLK #{} TICK {} - pulse detected (transmitting: {})",
                event.pulse_count, time_delta_micros, event.transmitting
            );
        }
    }
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

    // Configure LED4 (P0.16) for clock detection
    let led4 = Output::new(p.P0_16, Level::High, OutputDrive::Standard);

    // Configure Meter pins - P0.02 for clock input (from MTU), P0.03 for data output (to MTU)
    let meter_clock_pin = Input::new(p.P0_02, Pull::None); // Clock input from MTU, no pull resistor like RPI project
    let meter_data_pin = Output::new(p.P0_03, Level::High, OutputDrive::Standard); // Data output to MTU

    // Configure UART for CLI
    let mut uart_config = uarte::Config::default();
    uart_config.parity = uarte::Parity::EXCLUDED;
    uart_config.baudrate = uarte::Baudrate::BAUD115200;

    let uarte = Uarte::new(p.UARTE1, Irqs, p.P1_14, p.P1_15, uart_config);
    info!("✅ Peripherals configured");

    // Initialize CLI components with meter functionality
    let mut terminal = Terminal::new(uarte).with_tx_led(led2);

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
    let _ = terminal
        .write_line("Debug LEDs: LED3 (data tx) | LED4 (clock detect)")
        .await;
    let _ = terminal.print_prompt().await;

    // Make static references for the background task
    let static_clock_pin =
        unsafe { core::mem::transmute::<Input<'_>, Input<'static>>(meter_clock_pin) };
    let static_led4 = unsafe { core::mem::transmute::<Output<'_>, Output<'static>>(led4) };
    let static_data_pin =
        unsafe { core::mem::transmute::<Output<'_>, Output<'static>>(meter_data_pin) };
    let static_led3 = unsafe { core::mem::transmute::<Output<'_>, Output<'static>>(led3) };

    // Create meter handler for CLI command processing
    // Note: We can't use the actual GPIO pins here since they're used by the background task
    // This handler will be used for configuration commands only
    let meter_config = MeterConfig::default();
    let dummy_clock_pin = Input::new(p.P0_04, Pull::None); // Unused pin for handler
    let dummy_data_pin = Output::new(p.P0_05, Level::High, OutputDrive::Standard); // Unused pin
    let dummy_activity_led = Output::new(p.P0_06, Level::High, OutputDrive::Standard); // Unused pin
    let mut meter_handler = MeterHandler::new(
        meter_config,
        dummy_clock_pin,
        dummy_data_pin,
        dummy_activity_led,
    );

    // Make meter handler static for the background task
    let static_meter_handler = unsafe {
        core::mem::transmute::<&mut MeterHandler<'_>, &'static MeterHandler<'static>>(
            &mut meter_handler,
        )
    };

    // Create channel for communication between tasks
    static EVENT_CHANNEL: Channel<ThreadModeRawMutex, ClockEvent, 32> = Channel::new();
    let event_sender = EVENT_CHANNEL.sender();
    let event_receiver = EVENT_CHANNEL.receiver();

    // Spawn fast clock response task (highest priority)
    spawner
        .spawn(fast_clock_response_task(
            static_clock_pin,
            static_data_pin,
            static_meter_handler,
            event_sender,
        ))
        .unwrap();

    // Spawn LED and logging task (lower priority)
    spawner
        .spawn(led_logging_task(
            static_led4, // Clock LED (brief flash on each pulse)
            static_led3, // Activity LED (on during transmission)
            event_receiver,
        ))
        .unwrap();

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
                        // Parse and execute meter commands
                        let command = MeterCommandParser::parse_command(&command_line);

                        match meter_handler.execute_command(command).await {
                            Ok(response) => {
                                // Only write response if it's not empty
                                if !response.is_empty() {
                                    let _ = terminal.write_line(&response).await;
                                }
                            }
                            Err(_) => {
                                error!("Meter: Command execution error");
                                let _ = terminal.write_line("Command execution error").await;
                            }
                        }

                        // Handle special commands that need terminal interaction
                        if command_line.trim() == "help" || command_line.trim() == "h" {
                            let _ = terminal.show_meter_help().await;
                        } else if command_line.trim() == "clear" || command_line.trim() == "cls" {
                            let _ = terminal.clear_screen().await;
                        }

                        let _ = terminal.print_prompt().await;
                    }
                    Ok(None) => {
                        // Character processed but no complete command yet
                    }
                    Err(_) => {
                        // Handle error
                        error!("Meter: Terminal input error");
                        let _ = terminal.write_line("Input error").await;
                        let _ = terminal.print_prompt().await;
                    }
                }
            }
            Err(_) => {
                // No LED activity on UART read error (no data received)
                // This is expected when no data is available, so we don't log it as an error
                // Continue on error
            }
        }
    }
}
