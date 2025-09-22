use super::{CliCommand, CliError};
use core::fmt::Write;
use cortex_m::peripheral::SCB;
use defmt::info;
use embassy_nrf::gpio::{Input, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use heapless::String;
use nrf_softdevice::ble::central;
use nrf_softdevice::Softdevice;

pub struct CommandHandler<'d> {
    led_states: [bool; 4],
    start_time: Instant,
    led3: Option<Output<'d>>,
    led4: Option<Output<'d>>,
    button1: Option<Input<'d>>,
    button2: Option<Input<'d>>,
    button3: Option<Input<'d>>,
    button4: Option<Input<'d>>,
    softdevice: Option<&'d Softdevice>,
    mtu: Option<Mutex<ThreadModeRawMutex, crate::mtu::GpioMtu>>,
    mtu_clock_pin: Option<Output<'d>>,
    mtu_data_pin: Option<Input<'d>>,
    mtu_clock_led: Option<Output<'d>>,
    mtu_data_led: Option<Output<'d>>,
}

impl<'d> Default for CommandHandler<'d> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d> CommandHandler<'d> {
    pub fn new() -> Self {
        Self {
            led_states: [false; 4],
            start_time: Instant::now(),
            led3: None,
            led4: None,
            button1: None,
            button2: None,
            button3: None,
            button4: None,
            softdevice: None,
            mtu: None,
            mtu_clock_pin: None,
            mtu_data_pin: None,
            mtu_clock_led: None,
            mtu_data_led: None,
        }
    }

    pub fn with_leds(mut self, led3: Output<'d>, led4: Output<'d>) -> Self {
        self.led3 = Some(led3);
        self.led4 = Some(led4);
        self
    }

    pub fn with_buttons(
        mut self,
        button1: Input<'d>,
        button2: Input<'d>,
        button3: Input<'d>,
        button4: Input<'d>,
    ) -> Self {
        self.button1 = Some(button1);
        self.button2 = Some(button2);
        self.button3 = Some(button3);
        self.button4 = Some(button4);
        self
    }

    pub fn with_softdevice(mut self, softdevice: &'d Softdevice) -> Self {
        self.softdevice = Some(softdevice);
        self
    }

    pub fn with_mtu(mut self, mtu_clock_pin: Output<'d>, mtu_data_pin: Input<'d>) -> Self {
        let mtu = crate::mtu::GpioMtu::new(crate::mtu::MtuConfig::default());
        self.mtu = Some(Mutex::new(mtu));
        self.mtu_clock_pin = Some(mtu_clock_pin);
        self.mtu_data_pin = Some(mtu_data_pin);
        self
    }

    pub fn with_mtu_debug_leds(mut self, clock_led: Output<'d>, data_led: Output<'d>) -> Self {
        self.mtu_clock_led = Some(clock_led);
        self.mtu_data_led = Some(data_led);
        self
    }

    pub async fn execute_command(
        &mut self,
        command: CliCommand,
    ) -> Result<heapless::String<256>, CliError> {
        let mut response = heapless::String::new();

        match command {
            CliCommand::Empty => {
                // Empty command - just return empty response (no error)
                // This will result in just showing a new prompt
            }
            CliCommand::Help => {
                // Help is handled in terminal.rs
                let _ = response.push_str("Help displayed");
            }
            CliCommand::Version => {
                info!("CLI: Version requested");
                let _ = response.push_str("nRF52840-DK CLI v1.0.0");
            }
            CliCommand::Status => {
                info!("CLI: Status requested");
                let _ = response.push_str("System Status:\r\n");
                let _ = response.push_str("  Firmware: nRF52840-DK CLI v1.0.0\r\n");
                let _ = response.push_str("  UART: 115200 baud on pins P1.14/P1.15\r\n");
                let _ = response.push_str("  LEDs: ");
                let _ = response.push_str("3:");
                let _ = response.push_str(if self.led_states[2] { "on " } else { "off " });
                let _ = response.push_str("4:");
                let _ = response.push_str(if self.led_states[3] { "on" } else { "off" });
            }
            CliCommand::Uptime => {
                info!("CLI: Uptime requested");
                let uptime = Instant::now() - self.start_time;
                let uptime_secs = uptime.as_secs();
                let hours = uptime_secs / 3600;
                let minutes = (uptime_secs % 3600) / 60;
                let seconds = uptime_secs % 60;

                let _ = response.push_str("Uptime: ");
                if hours > 0 {
                    let _ = write_num(&mut response, hours);
                    let _ = response.push_str("h ");
                }
                if minutes > 0 || hours > 0 {
                    let _ = write_num(&mut response, minutes);
                    let _ = response.push_str("m ");
                }
                let _ = write_num(&mut response, seconds);
                let _ = response.push_str("s");
            }
            CliCommand::Clear => {
                // Clear is handled in terminal.rs
                let _ = response.push_str("Screen cleared");
            }
            CliCommand::Reset => {
                info!("CLI: Reset requested");
                let _ = response.push_str("Resetting system...");
                // Perform system reset using cortex-m
                SCB::sys_reset();
            }
            CliCommand::Echo(text) => {
                info!("CLI: Echo requested: {}", text.as_str());
                let _ = response.push_str(&text);
            }
            CliCommand::LedOn(led_num) => {
                info!("CLI: LED {} on requested", led_num);
                let idx = (led_num - 1) as usize;
                if idx < 4 {
                    self.led_states[idx] = true;
                    let _ = response.push_str("LED ");
                    let _ = response.push((led_num + b'0') as char);
                    let _ = response.push_str(" turned on");

                    // Actually control the LED hardware
                    match led_num {
                        3 => {
                            if let Some(ref mut led) = self.led3 {
                                led.set_low(); // LEDs are active low
                            }
                        }
                        4 => {
                            if let Some(ref mut led) = self.led4 {
                                led.set_low(); // LEDs are active low
                            }
                        }
                        _ => {}
                    }
                }
            }
            CliCommand::LedOff(led_num) => {
                info!("CLI: LED {} off requested", led_num);
                let idx = (led_num - 1) as usize;
                if idx < 4 {
                    self.led_states[idx] = false;
                    let _ = response.push_str("LED ");
                    let _ = response.push((led_num + b'0') as char);
                    let _ = response.push_str(" turned off");

                    // Actually control the LED hardware
                    match led_num {
                        3 => {
                            if let Some(ref mut led) = self.led3 {
                                led.set_high(); // LEDs are active low
                            }
                        }
                        4 => {
                            if let Some(ref mut led) = self.led4 {
                                led.set_high(); // LEDs are active low
                            }
                        }
                        _ => {}
                    }
                }
            }
            CliCommand::Button => {
                info!("CLI: Button state requested");
                let _ = response.push_str("Button States:\r\n");

                // Read button states (buttons are active low)
                if let (Some(ref btn1), Some(ref btn2), Some(ref btn3), Some(ref btn4)) =
                    (&self.button1, &self.button2, &self.button3, &self.button4)
                {
                    let btn1_pressed = btn1.is_low();
                    let btn2_pressed = btn2.is_low();
                    let btn3_pressed = btn3.is_low();
                    let btn4_pressed = btn4.is_low();

                    let _ = response.push_str("  Button 1: ");
                    let _ = response.push_str(if btn1_pressed { "pressed" } else { "released" });
                    let _ = response.push_str("\r\n");

                    let _ = response.push_str("  Button 2: ");
                    let _ = response.push_str(if btn2_pressed { "pressed" } else { "released" });
                    let _ = response.push_str("\r\n");

                    let _ = response.push_str("  Button 3: ");
                    let _ = response.push_str(if btn3_pressed { "pressed" } else { "released" });
                    let _ = response.push_str("\r\n");

                    let _ = response.push_str("  Button 4: ");
                    let _ = response.push_str(if btn4_pressed { "pressed" } else { "released" });
                } else {
                    let _ = response.push_str("  Buttons not configured");
                }
            }
            CliCommand::Temp => {
                info!("CLI: Temperature requested");
                // Use SoftDevice temperature reading
                match self.read_temperature() {
                    Ok(temp_celsius) => {
                        let _ = response.push_str("Temperature: ");
                        // Format temperature with one decimal place manually
                        let temp_int = temp_celsius as i32;
                        let temp_frac = ((temp_celsius - temp_int as f32) * 10.0) as i32;

                        // Write integer part (handle negative temperatures)
                        if temp_int < 0 {
                            let _ = response.push('-');
                            let _ = write_num(&mut response, (-temp_int) as u64);
                        } else {
                            let _ = write_num(&mut response, temp_int as u64);
                        }
                        let _ = response.push('.');
                        let _ = response.push((b'0' + temp_frac.unsigned_abs() as u8) as char);
                        let _ = response.push_str("°C");
                    }
                    Err(_) => {
                        let _ = response.push_str("Failed to read temperature sensor");
                    }
                }
            }
            CliCommand::BtScan(scan_time) => {
                let scan_duration = scan_time.unwrap_or(10); // Default 10 seconds
                info!("CLI: BLE scan requested for {} seconds", scan_duration);
                match self.perform_scan(scan_duration).await {
                    Ok(scan_results) => {
                        let _ = response.push_str("BLE scan completed (");
                        let _ = write_num(&mut response, scan_duration as u64);
                        let _ = response.push_str("s) - found ");
                        let _ = write_num(&mut response, scan_results.len() as u64);
                        let _ = response.push_str(" devices:\r\n");

                        // Display all devices that fit in the buffer
                        let mut displayed_count = 0;
                        for addr in scan_results.iter() {
                            // Calculate the exact space needed for this address line: "  aa:bb:cc:dd:ee:ff\r\n" = 21 chars
                            let line_length = 21;

                            // Check if this line would fit
                            if response.len() + line_length > response.capacity() {
                                break;
                            }

                            // Add the line since it fits
                            let _ = response.push_str("  ");
                            for (i, byte) in addr.iter().enumerate() {
                                if i > 0 {
                                    let _ = response.push(':');
                                }
                                let _ = write_hex_byte(&mut response, *byte);
                            }
                            let _ = response.push_str("\r\n");
                            displayed_count += 1;
                        }

                        let remaining = scan_results.len() - displayed_count;
                        if remaining > 0 {
                            let _ = response.push_str("  ... and ");
                            let _ = write_num(&mut response, remaining as u64);
                            let _ = response.push_str(" more\r\n");
                        }
                    }
                    Err(_) => {
                        let _ = response.push_str("BLE scan failed");
                    }
                }
            }
            CliCommand::MtuStart(duration) => {
                info!("CLI: MTU start requested");
                if let Some(ref mtu_mutex) = self.mtu {
                    let duration_secs = duration.unwrap_or(30);
                    let _ = response.push_str("Starting GPIO-based MTU operation for ");
                    let _ = write_num(&mut response, duration_secs as u64);
                    let _ = response.push_str(" seconds...\r\n");
                    let _ = response.push_str("MTU: Clock on P0.02, Data on P0.03\r\n");
                    let _ = response.push_str("Use 'mtu_status' to check for received messages");

                    // Start MTU operation
                    let mtu = mtu_mutex.lock().await;
                    if mtu.start().await.is_err() {
                        let _ = response.push_str("\r\nError: Failed to start MTU");
                    } else if let (Some(clock_pin), Some(data_pin)) =
                        (self.mtu_clock_pin.as_mut(), self.mtu_data_pin.as_ref())
                    {
                        // Start the actual MTU operation with LED debug indicators and stats tracking
                        let duration = embassy_time::Duration::from_secs(duration_secs as u64);
                        if let Err(e) = mtu
                            .run_mtu_operation_with_stats(
                                duration,
                                clock_pin,
                                data_pin,
                                self.mtu_clock_led.as_mut(),
                                self.mtu_data_led.as_mut(),
                            )
                            .await
                        {
                            let _ = response.push_str("\r\nError: MTU operation failed");
                            info!("MTU operation error: {:?}", e);
                        }
                    } else {
                        let _ = response.push_str("\r\nError: MTU GPIO pins not configured");
                    }
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuStop => {
                info!("CLI: MTU stop requested");
                if let Some(ref mtu_mutex) = self.mtu {
                    let mtu = mtu_mutex.lock().await;
                    mtu.stop();
                    let _ = response.push_str("MTU operation stopped");
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuStatus => {
                info!("CLI: MTU status requested");
                if let Some(ref mtu_mutex) = self.mtu {
                    let mtu = mtu_mutex.lock().await;
                    let baud_rate = mtu.get_baud_rate().await;
                    let expected_message = mtu.get_expected_message().await;
                    let (successful, corrupted) = mtu.get_stats().await;
                    let total_reads = successful + corrupted;

                    let _ = response.push_str("MTU Status:\r\n");
                    let _ = response.push_str("  State: ");
                    let _ = response.push_str(if mtu.is_running() {
                        "Running"
                    } else {
                        "Stopped"
                    });
                    let _ = response.push_str("\r\n");
                    let _ = response.push_str("  Baud rate: ");
                    let mut baud_str = heapless::String::<16>::new();
                    let _ = write!(baud_str, "{}", baud_rate);
                    let _ = response.push_str(baud_str.as_str());
                    let _ = response.push_str(" bps\r\n");
                    let _ = response.push_str("  Pins: P0.02 (clock), P0.03 (data)\r\n");

                    // Show expected message for testing
                    let _ = response.push_str("  Expected Message: ");
                    let _ = response.push_str(expected_message.as_str());
                    let _ = response.push_str("\r\n");

                    // Show running statistics
                    let _ = response.push_str("  Statistics:\r\n");
                    let _ = response.push_str("    Successful reads: ");
                    let _ = write_num(&mut response, successful as u64);
                    let _ = response.push_str("\r\n    Corrupted reads: ");
                    let _ = write_num(&mut response, corrupted as u64);
                    let _ = response.push_str("\r\n    Total reads: ");
                    let _ = write_num(&mut response, total_reads as u64);
                    if total_reads > 0 {
                        let success_rate = (successful as f32 / total_reads as f32) * 100.0;
                        let _ = response.push_str("\r\n    Success rate: ");
                        let _ = write_num(&mut response, success_rate as u64);
                        let _ = response.push('%');
                    }
                    let _ = response.push_str("\r\n");

                    // Show last received message
                    if let Some(message) = mtu.get_last_message().await {
                        let _ = response.push_str("  Last Message: ");
                        let _ = response.push_str(message.as_str());
                    } else {
                        let _ = response.push_str("  Last Message: None");
                    }
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuBaud(baud_rate) => {
                info!("CLI: MTU baud rate set to {}", baud_rate);
                if let Some(ref mtu_mutex) = self.mtu {
                    let mtu = mtu_mutex.lock().await;
                    mtu.set_baud_rate(baud_rate).await;
                    let _ = response.push_str("MTU baud rate set to ");
                    let mut baud_str = heapless::String::<16>::new();
                    let _ = write!(baud_str, "{}", baud_rate);
                    let _ = response.push_str(baud_str.as_str());
                    let _ = response.push_str(" bps");
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuTest(iterations) => {
                info!("CLI: MTU test requested for {} iterations", iterations);
                if let Some(ref mtu_mutex) = self.mtu {
                    let _ = response.push_str("Starting MTU test with ");
                    let _ = write_num(&mut response, iterations as u64);
                    let _ = response.push_str(" iterations...\r\n");

                    // Run the test
                    let mtu = mtu_mutex.lock().await;
                    if let (Some(clock_pin), Some(data_pin)) =
                        (self.mtu_clock_pin.as_mut(), self.mtu_data_pin.as_ref())
                    {
                        match mtu
                            .run_test(
                                iterations,
                                clock_pin,
                                data_pin,
                                self.mtu_clock_led.as_mut(),
                                self.mtu_data_led.as_mut(),
                            )
                            .await
                        {
                            Ok((successful, corrupted)) => {
                                let _ = response.push_str("MTU test completed:\r\n");
                                let _ = response.push_str("  Successful: ");
                                let _ = write_num(&mut response, successful as u64);
                                let _ = response.push('/');
                                let _ = write_num(&mut response, iterations as u64);
                                let _ = response.push_str("\r\n  Corrupted: ");
                                let _ = write_num(&mut response, corrupted as u64);
                                let _ = response.push('/');
                                let _ = write_num(&mut response, iterations as u64);
                                let success_rate = (successful as f32 / iterations as f32) * 100.0;
                                let _ = response.push_str("\r\n  Success rate: ");
                                let _ = write_num(&mut response, success_rate as u64);
                                let _ = response.push('%');
                            }
                            Err(e) => {
                                let _ = response.push_str("MTU test failed: ");
                                let _ = response.push_str("Error during test execution");
                                info!("MTU test error: {:?}", e);
                            }
                        }
                    } else {
                        let _ = response.push_str("Error: MTU GPIO pins not configured");
                    }
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuExpect(expected_message) => {
                info!("CLI: MTU expected message set");
                if let Some(ref mtu_mutex) = self.mtu {
                    let mtu = mtu_mutex.lock().await;
                    mtu.set_expected_message(expected_message.clone()).await;
                    let _ = response.push_str("Expected message set to: ");
                    let _ = response.push_str(expected_message.as_str());
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::MtuReset => {
                info!("CLI: MTU statistics reset requested");
                if let Some(ref mtu_mutex) = self.mtu {
                    let mtu = mtu_mutex.lock().await;
                    mtu.reset_stats().await;
                    let _ = response.push_str("MTU statistics reset");
                } else {
                    let _ = response.push_str("MTU not configured");
                }
            }
            CliCommand::Unknown(cmd) => {
                info!("CLI: Unknown command: {}", cmd.as_str());
                let _ = response.push_str("Unknown command: ");
                let _ = response.push_str(&cmd);
                let _ = response.push_str(". Type 'help' for available commands.");
            }
        }

        Ok(response)
    }

    fn read_temperature(&self) -> Result<f32, CliError> {
        // Read temperature using SoftDevice
        if let Some(softdevice) = self.softdevice {
            match nrf_softdevice::temperature_celsius(softdevice) {
                Ok(temp_fixed) => {
                    // Convert fixed-point I30F2 to float
                    let temp_celsius = temp_fixed.to_num::<f32>();
                    Ok(temp_celsius)
                }
                Err(_) => Err(CliError::UartError),
            }
        } else {
            Err(CliError::UartError) // No SoftDevice available
        }
    }

    async fn perform_scan(&self, scan_time: u16) -> Result<heapless::Vec<[u8; 6], 10>, CliError> {
        if let Some(softdevice) = self.softdevice {
            info!("Starting BLE scan for {} seconds", scan_time);
            let start_time = embassy_time::Instant::now();

            let config = central::ScanConfig {
                timeout: scan_time * 100, // Convert seconds to 10ms units (1 sec = 100 * 10ms)
                ..Default::default()
            };

            let mut discovered_devices = heapless::Vec::<[u8; 6], 10>::new();

            let result = central::scan(softdevice, &config, |params| {
                let addr = params.peer_addr.addr;

                // Check if we've already seen this device
                if !discovered_devices.contains(&addr) {
                    if discovered_devices.push(addr).is_ok() {
                        info!(
                            "BLE Device found: addr={:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                            addr[0], addr[1], addr[2], addr[3], addr[4], addr[5]
                        );
                        None::<()> // Continue scanning
                    } else {
                        info!("Device buffer full (10 devices), stopping scan...");
                        Some(()) // Stop scanning - buffer is full
                    }
                } else {
                    None::<()> // Continue scanning - duplicate device
                }
            })
            .await;

            let end_time = embassy_time::Instant::now();
            let actual_duration = end_time - start_time;
            info!("BLE scan finished after {}ms", actual_duration.as_millis());

            match result {
                Ok(_) => {
                    info!(
                        "BLE scan completed successfully with {} unique devices",
                        discovered_devices.len()
                    );
                    Ok(discovered_devices)
                }
                Err(central::ScanError::Timeout) => {
                    // Timeout is expected and normal - treat as success
                    info!(
                        "BLE scan completed (timeout) with {} unique devices",
                        discovered_devices.len()
                    );
                    Ok(discovered_devices)
                }
                Err(e) => {
                    info!("BLE scan error: {:?}", e);
                    Err(CliError::UartError)
                }
            }
        } else {
            info!("No SoftDevice available for scanning");
            Err(CliError::UartError)
        }
    }
}

// Helper function to write numbers to string without using std::fmt
fn write_num(s: &mut String<256>, mut num: u64) -> Result<(), ()> {
    if num == 0 {
        return s.push('0').map_err(|_| ());
    }

    let mut digits = heapless::Vec::<u8, 20>::new();
    while num > 0 {
        let _ = digits.push((num % 10) as u8);
        num /= 10;
    }

    for &digit in digits.iter().rev() {
        s.push((b'0' + digit) as char).map_err(|_| ())?;
    }

    Ok(())
}

// Helper function to write hex byte to string
fn write_hex_byte(s: &mut String<256>, byte: u8) -> Result<(), ()> {
    let hex_chars = b"0123456789abcdef";
    let high = (byte >> 4) & 0x0f;
    let low = byte & 0x0f;

    s.push(hex_chars[high as usize] as char).map_err(|_| ())?;
    s.push(hex_chars[low as usize] as char).map_err(|_| ())?;

    Ok(())
}
