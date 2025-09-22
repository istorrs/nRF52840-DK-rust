use super::{MeterCommand, MeterConfig, MeterType};
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::info;
use embassy_nrf::gpio::{Input, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Instant;
use heapless::String;

pub struct MeterHandler<'d> {
    config: Mutex<ThreadModeRawMutex, MeterConfig>,
    #[allow(dead_code)]
    clock_pin: Mutex<ThreadModeRawMutex, Input<'d>>,
    #[allow(dead_code)] // Used in clock detection task via static transmutation
    data_pin: Mutex<ThreadModeRawMutex, Output<'d>>,
    #[allow(dead_code)] // Used in clock detection task via static transmutation
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

    pub async fn execute_command(&mut self, command: MeterCommand) -> Result<String<256>, ()> {
        let mut response = String::new();

        match command {
            MeterCommand::Empty => {
                // Empty command - just return empty response (no error)
                // This will result in just showing a new prompt
            }
            MeterCommand::Help => {
                let _ = response.push_str("Meter commands:\r\n");
                let _ = response.push_str("  type [sensus|neptune] - Set meter type\r\n");
                let _ = response.push_str("  message <text> - Set response message\r\n");
                let _ = response.push_str("  enable/disable - Enable/disable meter\r\n");
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
                let _ = response.push_str(if config.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                });
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
        }

        Ok(response)
    }

    // Build UART frame with proper framing for meter type
    fn build_uart_frame(&self, byte: u8, meter_type: &MeterType) -> heapless::Vec<u8, 12> {
        let mut frame = heapless::Vec::new();

        // Start bit
        let _ = frame.push(0);

        // Data bits (LSB first) - only 7 bits for 7E1/7E2 framing
        let data_7bit = byte & 0x7F; // Mask to 7 bits
        for i in 0..7 {
            let bit = (data_7bit >> i) & 1;
            let _ = frame.push(bit);
        }

        // Parity and stop bits based on meter type
        match meter_type {
            MeterType::Sensus => {
                // 7E1: 7 data bits + even parity + 1 stop bit
                // Calculate even parity for the 7 data bits
                let parity = (data_7bit.count_ones() % 2) as u8;
                let _ = frame.push(parity);
                let _ = frame.push(1); // stop bit
            }
            MeterType::Neptune => {
                // 7E2: 7 data bits + even parity + 2 stop bits
                let parity = (data_7bit.count_ones() % 2) as u8;
                let _ = frame.push(parity);
                let _ = frame.push(1); // stop bit 1
                let _ = frame.push(1); // stop bit 2
            }
        }

        frame
    }

    // Build complete response frame buffer for all characters in the message
    pub async fn build_response_frames(&self) -> heapless::Vec<u8, 2048> {
        let config = self.config.lock().await;
        let mut frame_buffer = heapless::Vec::new();

        // Build frames for each character in the response message
        for (char_index, ch) in config.response_message.chars().enumerate() {
            let char_frame = self.build_uart_frame(ch as u8, &config.meter_type);
            defmt::info!(
                "Meter: Building frame for char #{}: '{}' (ASCII {}) -> {} bits: [{}]",
                char_index + 1,
                ch,
                ch as u8,
                char_frame.len(),
                char_frame
                    .iter()
                    .map(|&b| if b == 1 { '1' } else { '0' })
                    .collect::<heapless::String<32>>()
                    .as_str()
            );
            for &bit in &char_frame {
                let _ = frame_buffer.push(bit);
            }
        }

        defmt::info!(
            "Meter: Complete frame buffer: {} total bits for {} characters",
            frame_buffer.len(),
            config.response_message.len()
        );
        frame_buffer
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
