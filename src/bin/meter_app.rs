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
use nrf52840_dk_template::cli::Terminal;

bind_interrupts!(struct Irqs {
    UARTE1 => embassy_nrf::uarte::InterruptHandler<embassy_nrf::peripherals::UARTE1>;
});

#[embassy_executor::task]
async fn softdevice_task(sd: &'static Softdevice) -> ! {
    sd.run().await
}

#[embassy_executor::task]
async fn clock_detection_task(
    mut clock_pin: Input<'static>,
    mut clock_led: Output<'static>,
    mut data_pin: Output<'static>,
    mut activity_led: Output<'static>,
) -> ! {
    info!("Clock detection task started");

    let mut pulse_count = 0u32;
    let mut last_pulse_time = embassy_time::Instant::now();
    let wake_up_threshold = 10; // Minimum pulses to consider wake-up sequence
    let pulse_timeout = embassy_time::Duration::from_millis(10); // Longer timeout for pulse sequences

    loop {
        // Wait for any clock signal (both edges like RPI project)
        clock_pin.wait_for_any_edge().await;

        let now = embassy_time::Instant::now();

        // Check if this pulse is part of a sequence (within timeout)
        if now.duration_since(last_pulse_time) > pulse_timeout {
            pulse_count = 0; // Reset count if too much time passed
        }

        pulse_count += 1;
        last_pulse_time = now;

        // Flash LED4 briefly for each clock edge
        clock_led.set_low(); // LED on
        embassy_time::Timer::after(embassy_time::Duration::from_millis(20)).await;
        clock_led.set_high(); // LED off

        info!("Clock edge detected! Pulse count: {}", pulse_count);

        // Check if we have a complete wake-up sequence
        if pulse_count >= wake_up_threshold {
            info!("Wake-up sequence detected! Sending meter response...");
            pulse_count = 0; // Reset for next sequence

            // Send automatic response with standard meter message
            let message =
                "V;RB00000123;IB12345678;A0000;Z1000;XT0732;MT0661;RR00000100;GX333333;GN000000\r";
            if (send_meter_response(message, &mut data_pin, &mut activity_led).await).is_err() {
                info!("Failed to send meter response");
            }

            // Wait a bit before detecting next wake-up sequence
            embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
        }
    }
}

// Send meter response via GPIO UART simulation
async fn send_meter_response(
    message: &str,
    data_pin: &mut Output<'_>,
    activity_led: &mut Output<'_>,
) -> Result<(), ()> {
    info!("Sending meter response: {}", message);

    // Flash activity LED during transmission
    activity_led.set_low(); // LED on

    // Send each character in the message using 7E1 framing (Sensus Standard)
    for ch in message.chars() {
        if send_uart_char(data_pin, ch as u8).await.is_err() {
            activity_led.set_high(); // LED off
            return Err(());
        }
    }

    activity_led.set_high(); // LED off
    info!("Meter response sent successfully");
    Ok(())
}

// Send a single character via GPIO UART at 9600 baud with 7E1 framing
async fn send_uart_char(data_pin: &mut Output<'_>, byte: u8) -> Result<(), ()> {
    let bit_duration = embassy_time::Duration::from_micros(104); // 9600 baud = ~104μs per bit

    // 7E1 framing: 1 start + 7 data + 1 even parity + 1 stop
    let data_7bit = byte & 0x7F; // Use only 7 bits
    let parity = (data_7bit.count_ones() % 2) as u8; // Even parity

    // Send start bit (low)
    data_pin.set_low();
    embassy_time::Timer::after(bit_duration).await;

    // Send 7 data bits (LSB first)
    for i in 0..7 {
        let bit = (data_7bit >> i) & 1;
        if bit == 1 {
            data_pin.set_high();
        } else {
            data_pin.set_low();
        }
        embassy_time::Timer::after(bit_duration).await;
    }

    // Send parity bit
    if parity == 1 {
        data_pin.set_high();
    } else {
        data_pin.set_low();
    }
    embassy_time::Timer::after(bit_duration).await;

    // Send stop bit (high)
    data_pin.set_high();
    embassy_time::Timer::after(bit_duration).await;

    Ok(())
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

    // Spawn background task for clock signal detection
    spawner
        .spawn(clock_detection_task(
            static_clock_pin,
            static_led4,
            static_data_pin,
            static_led3,
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
                    Ok(Some(_command_line)) => {
                        // Simple command handling - automatic response mode
                        let _ = terminal
                            .write_line(
                                "Automatic meter mode - responses handled by background task",
                            )
                            .await;
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
