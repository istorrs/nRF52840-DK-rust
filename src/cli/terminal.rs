use super::{parser::CommandParser, CliError, CLI_BUFFER_SIZE};
use embassy_nrf::uarte::Uarte;
use heapless::String;

pub struct Terminal<'d> {
    pub uart: Uarte<'d, embassy_nrf::peripherals::UARTE1>,
    line_buffer: String<CLI_BUFFER_SIZE>,
    cursor_pos: usize,
}

impl<'d> Terminal<'d> {
    pub fn new(uart: Uarte<'d, embassy_nrf::peripherals::UARTE1>) -> Self {
        Self {
            uart,
            line_buffer: String::new(),
            cursor_pos: 0,
        }
    }

    pub async fn write_str(&mut self, s: &str) -> Result<(), CliError> {
        // Send each character individually to debug transmission
        for &byte in s.as_bytes() {
            self.uart
                .write(&[byte])
                .await
                .map_err(|_| CliError::UartError)?;
        }
        Ok(())
    }

    pub async fn write_line(&mut self, s: &str) -> Result<(), CliError> {
        self.write_str(s).await?;
        self.write_str("\r\n").await
    }

    pub async fn print_prompt(&mut self) -> Result<(), CliError> {
        self.write_str("nRF52840-DK CLI> ").await
    }

    pub async fn handle_char(
        &mut self,
        ch: u8,
    ) -> Result<Option<String<CLI_BUFFER_SIZE>>, CliError> {
        match ch {
            b'\r' | b'\n' => {
                // Enter pressed - return the command
                self.write_str("\r\n").await?;
                let command = self.line_buffer.clone();
                self.line_buffer.clear();
                self.cursor_pos = 0;
                Ok(Some(command))
            }
            b'\x08' | b'\x7f' => {
                // Backspace
                if !self.line_buffer.is_empty() && self.cursor_pos > 0 {
                    self.line_buffer.pop();
                    self.cursor_pos -= 1;
                    // Send backspace sequence: backspace + space + backspace
                    self.write_str("\x08 \x08").await?;
                }
                Ok(None)
            }
            b'\t' => {
                // Tab - autocomplete
                self.handle_tab_completion().await?;
                Ok(None)
            }
            0x20..=0x7E => {
                // Printable ASCII character
                if self.line_buffer.len() < CLI_BUFFER_SIZE - 1
                    && self.line_buffer.push(ch as char).is_ok()
                {
                    self.cursor_pos += 1;
                    // Echo the character
                    let echo = [ch];
                    self.uart
                        .write(&echo)
                        .await
                        .map_err(|_| CliError::UartError)?;
                }
                Ok(None)
            }
            _ => {
                // Ignore other control characters for now
                Ok(None)
            }
        }
    }

    pub async fn clear_screen(&mut self) -> Result<(), CliError> {
        // ANSI escape sequence to clear screen and move cursor to top
        self.write_str("\x1b[2J\x1b[H").await
    }

    async fn handle_tab_completion(&mut self) -> Result<(), CliError> {
        // Clone the current line to avoid borrowing issues
        let current_line: String<CLI_BUFFER_SIZE> = self.line_buffer.clone();
        let current_line_str = current_line.as_str();
        let words: heapless::Vec<&str, 8> = current_line_str.split_whitespace().collect();

        // Only autocomplete the first word (command)
        if words.is_empty() || (!current_line_str.ends_with(' ') && words.len() == 1) {
            let partial = if words.is_empty() { "" } else { words[0] };
            let matches = CommandParser::autocomplete(partial);

            match matches.len() {
                0 => {
                    // No matches - do nothing
                }
                1 => {
                    // Single match - complete it
                    let completion = matches[0];
                    let partial_len = partial.len();

                    // Clear current partial command
                    for _ in 0..partial_len {
                        if self.cursor_pos > 0 {
                            self.line_buffer.pop();
                            self.cursor_pos -= 1;
                            self.write_str("\x08 \x08").await?;
                        }
                    }
                    // Write the completion
                    for ch in completion.chars() {
                        if self.line_buffer.len() < CLI_BUFFER_SIZE - 1
                            && self.line_buffer.push(ch).is_ok()
                        {
                            self.cursor_pos += 1;
                            let echo = [ch as u8];
                            self.uart
                                .write(&echo)
                                .await
                                .map_err(|_| CliError::UartError)?;
                        }
                    }
                    // Add a space after completion
                    if self.line_buffer.len() < CLI_BUFFER_SIZE - 1
                        && self.line_buffer.push(' ').is_ok()
                    {
                        self.cursor_pos += 1;
                        self.uart
                            .write(b" ")
                            .await
                            .map_err(|_| CliError::UartError)?;
                    }
                }
                _ => {
                    // Multiple matches - show them
                    self.write_str("\r\n").await?;
                    for (i, &cmd) in matches.iter().enumerate() {
                        if i > 0 {
                            self.write_str("  ").await?;
                        }
                        self.write_str(cmd).await?;
                    }
                    self.write_str("\r\n").await?;
                    // Redraw prompt and current line
                    self.print_prompt().await?;
                    self.write_str(&current_line).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn show_help(&mut self) -> Result<(), CliError> {
        self.write_line("Available commands:").await?;
        self.write_line("  help        - Show this help").await?;
        self.write_line("  version     - Show firmware version")
            .await?;
        self.write_line("  status      - Show system status")
            .await?;
        self.write_line("  uptime      - Show system uptime")
            .await?;
        self.write_line("  clear       - Clear terminal").await?;
        self.write_line("  reset       - Reset system").await?;
        self.write_line("  echo <text> - Echo text back").await?;
        self.write_line("  led_on <3|4>  - Turn on LED 3 or 4")
            .await?;
        self.write_line("  led_off <3|4> - Turn off LED 3 or 4")
            .await?;
        self.write_line("  button      - Show button states")
            .await?;
        self.write_line("  temp        - Show temperature").await?;
        self.write_line("  bt_on       - Enable BLE").await?;
        self.write_line("  bt_off      - Disable BLE").await?;
        self.write_line("  bt_scan     - Scan for BLE devices")
            .await?;
        self.write_line("").await?;
        self.write_line("Use TAB to autocomplete commands").await?;
        Ok(())
    }
}
