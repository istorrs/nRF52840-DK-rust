use super::{CliCommand, CliError};
use defmt::info;
use embassy_nrf::gpio::Output;
use embassy_time::Instant;
use heapless::String;

pub struct CommandHandler<'d> {
    led_states: [bool; 4],
    start_time: Instant,
    led3: Option<Output<'d>>,
    led4: Option<Output<'d>>,
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
        }
    }

    pub fn with_leds(mut self, led3: Output<'d>, led4: Output<'d>) -> Self {
        self.led3 = Some(led3);
        self.led4 = Some(led4);
        self
    }

    pub async fn execute_command(
        &mut self,
        command: CliCommand,
    ) -> Result<heapless::String<128>, CliError> {
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
                let _ = response.push_str("System reset not implemented yet");
                // TODO: Implement system reset
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
                let _ = response.push_str("Button reading not implemented yet");
                // TODO: Read actual button states
            }
            CliCommand::Temp => {
                info!("CLI: Temperature requested");
                let _ = response.push_str("Temperature reading not implemented yet");
                // TODO: Read temperature sensor
            }
            CliCommand::BtOn => {
                info!("CLI: BLE enable requested");
                let _ = response.push_str("BLE enable not implemented yet");
                // TODO: Send command to BLE task
            }
            CliCommand::BtOff => {
                info!("CLI: BLE disable requested");
                let _ = response.push_str("BLE disable not implemented yet");
                // TODO: Send command to BLE task
            }
            CliCommand::BtScan => {
                info!("CLI: BLE scan requested");
                let _ = response.push_str("BLE scan not implemented yet");
                // TODO: Trigger BLE scan
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
}

// Helper function to write numbers to string without using std::fmt
fn write_num(s: &mut String<128>, mut num: u64) -> Result<(), ()> {
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
