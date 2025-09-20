use super::{MeterCommand, MeterConfig, MeterType};
use defmt::info;
use embassy_nrf::gpio::{Input, Output};
use embassy_time::{Duration, Instant, Timer};
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use heapless::String;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct MeterHandler<'d> {
    config: Mutex<ThreadModeRawMutex, MeterConfig>,
    clock_pin: Mutex<ThreadModeRawMutex, Input<'d>>,
    data_pin: Mutex<ThreadModeRawMutex, Output<'d>>,
    activity_led: Mutex<ThreadModeRawMutex, Output<'d>>,
    listening: AtomicBool,
    start_time: Instant,
}

impl<'d> MeterHandler<'d> {
    pub fn new(
        config: MeterConfig,
        clock_pin: Input<'d>,
        data_pin: Output<'d>,
        activity_led: Output<'d>,
    ) -> Self {
        Self {
            config: Mutex::new(config),
            clock_pin: Mutex::new(clock_pin),
            data_pin: Mutex::new(data_pin),
            activity_led: Mutex::new(activity_led),
            listening: AtomicBool::new(true),
            start_time: Instant::now(),
        }
    }

    pub async fn execute_command(
        &mut self,
        command: MeterCommand,
    ) -> Result<String<256>, ()> {
        let mut response = String::new();

        match command {
            MeterCommand::Help => {
                let _ = response.push_str("Meter commands:\r\n");
                let _ = response.push_str("  type [sensus|neptune] - Set meter type\r\n");
                let _ = response.push_str("  message <text> - Set response message\r\n");
                let _ = response.push_str("  enable/disable - Enable/disable meter\r\n");
                let _ = response.push_str("  test - Test meter response\r\n");
                let _ = response.push_str("  status - Show current config");
            }
            MeterCommand::Clear => {
                // Clear is handled in terminal
            }
            MeterCommand::Version => {
                let _ = response.push_str("Water Meter Simulator v1.0.0");
            }
            MeterCommand::Status => {
                let config = self.config.lock().await;
                let _ = response.push_str("Meter Status:\r\n");
                let _ = response.push_str("  Type: ");
                match config.meter_type {
                    MeterType::Sensus => {
                        let _ = response.push_str("Sensus (7E1)");
                    }
                    MeterType::Neptune => {
                        let _ = response.push_str("Neptune (7E2)");
                    }
                }
                let _ = response.push_str("\r\n  State: ");
                let _ = response.push_str(if config.enabled { "Enabled" } else { "Disabled" });
                let _ = response.push_str("\r\n  Message: ");
                let _ = response.push_str(config.response_message.as_str());
                let _ = response.push_str("\r\n  Pins: P0.02 (clock in), P0.03 (data out)");

                let uptime = Instant::now() - self.start_time;
                let uptime_secs = uptime.as_secs();
                let hours = uptime_secs / 3600;
                let minutes = (uptime_secs % 3600) / 60;
                let seconds = uptime_secs % 60;

                let _ = response.push_str("\r\n  Uptime: ");
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
            MeterCommand::SetType(meter_type) => {
                let mut config = self.config.lock().await;
                config.meter_type = meter_type.clone();

                let _ = response.push_str("Meter type set to: ");
                match meter_type {
                    MeterType::Sensus => {
                        let _ = response.push_str("Sensus (7E1)");
                    }
                    MeterType::Neptune => {
                        let _ = response.push_str("Neptune (7E2)");
                    }
                }
            }
            MeterCommand::SetMessage(message) => {
                let mut config = self.config.lock().await;
                config.response_message = message.clone();

                let _ = response.push_str("Response message set to: ");
                let _ = response.push_str(message.as_str());
            }
            MeterCommand::Enable => {
                let mut config = self.config.lock().await;
                config.enabled = true;
                self.listening.store(true, Ordering::Relaxed);

                let _ = response.push_str("Meter enabled - listening for MTU wake-up signals");
                info!("Meter: Enabled and listening");
            }
            MeterCommand::Disable => {
                let mut config = self.config.lock().await;
                config.enabled = false;
                self.listening.store(false, Ordering::Relaxed);

                let _ = response.push_str("Meter disabled");
                info!("Meter: Disabled");
            }
            MeterCommand::Test => {
                let config = self.config.lock().await;
                let _ = response.push_str("Testing meter response:\r\n");
                let _ = response.push_str("  Message: ");
                let _ = response.push_str(config.response_message.as_str());
                let _ = response.push_str("\r\n  Type: ");
                match config.meter_type {
                    MeterType::Sensus => {
                        let _ = response.push_str("Sensus (7E1)");
                    }
                    MeterType::Neptune => {
                        let _ = response.push_str("Neptune (7E2)");
                    }
                }

                // Simulate sending the response
                info!("Meter: Test response simulation");
                if let Err(_) = self.send_response(&config.response_message, &config.meter_type).await {
                    let _ = response.push_str("\r\n  Error: Failed to send test response");
                } else {
                    let _ = response.push_str("\r\n  Test response sent successfully");
                }
            }
        }

        Ok(response)
    }

    // Send a response message via GPIO UART simulation
    async fn send_response(&self, message: &str, meter_type: &MeterType) -> Result<(), ()> {
        let mut data_pin = self.data_pin.lock().await;
        let mut activity_led = self.activity_led.lock().await;

        info!("Meter: Sending response: {}", message);

        // Flash activity LED during transmission
        activity_led.set_low();

        // Send each character in the message
        for ch in message.chars() {
            if let Err(_) = self.send_uart_char(&mut data_pin, ch as u8, meter_type).await {
                activity_led.set_high();
                return Err(());
            }
        }

        activity_led.set_high();
        info!("Meter: Response sent successfully");
        Ok(())
    }

    // Send a single character via GPIO UART at 9600 baud
    async fn send_uart_char(&self, data_pin: &mut Output<'_>, byte: u8, meter_type: &MeterType) -> Result<(), ()> {
        let bit_duration = Duration::from_micros(104); // 9600 baud = ~104μs per bit

        // Build frame based on meter type
        let frame_bits = self.build_uart_frame(byte, meter_type);

        // Send start bit (low)
        data_pin.set_low();
        Timer::after(bit_duration).await;

        // Send data bits and parity/stop bits
        for &bit in &frame_bits[1..] {
            if bit == 1 {
                data_pin.set_high();
            } else {
                data_pin.set_low();
            }
            Timer::after(bit_duration).await;
        }

        // Ensure line returns to idle (high)
        data_pin.set_high();
        Timer::after(bit_duration).await;

        Ok(())
    }

    // Build UART frame with proper framing for meter type
    fn build_uart_frame(&self, byte: u8, meter_type: &MeterType) -> heapless::Vec<u8, 12> {
        let mut frame = heapless::Vec::new();

        // Start bit
        let _ = frame.push(0);

        // Data bits (LSB first)
        for i in 0..8 {
            let bit = (byte >> i) & 1;
            let _ = frame.push(bit);
        }

        // Parity and stop bits based on meter type
        match meter_type {
            MeterType::Sensus => {
                // 7E1: 7 data bits + even parity + 1 stop bit
                // Calculate even parity for lower 7 bits
                let data_7bit = byte & 0x7F;
                let parity = (data_7bit.count_ones() % 2) as u8;
                let _ = frame.push(parity);
                let _ = frame.push(1); // stop bit
            }
            MeterType::Neptune => {
                // 7E2: 7 data bits + even parity + 2 stop bits
                let data_7bit = byte & 0x7F;
                let parity = (data_7bit.count_ones() % 2) as u8;
                let _ = frame.push(parity);
                let _ = frame.push(1); // stop bit 1
                let _ = frame.push(1); // stop bit 2
            }
        }

        frame
    }
}

// Helper function to write numbers to string
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