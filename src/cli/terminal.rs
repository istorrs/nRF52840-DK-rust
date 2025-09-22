use super::{parser::CommandParser, CliError, CLI_BUFFER_SIZE};
use embassy_nrf::{gpio::Output, uarte::Uarte};
use embassy_time::{Duration, Timer};
use heapless::{String, Vec};

const HISTORY_SIZE: usize = 10;

pub struct Terminal<'d> {
    pub uart: Uarte<'d, embassy_nrf::peripherals::UARTE1>,
    tx_led: Option<Output<'d>>,
    line_buffer: String<CLI_BUFFER_SIZE>,
    cursor_pos: usize,
    command_history: Vec<String<CLI_BUFFER_SIZE>, HISTORY_SIZE>,
    history_index: Option<usize>,
    escape_state: EscapeState,
}

#[derive(Clone, Copy, PartialEq)]
enum EscapeState {
    Normal,
    Escape,
    Csi,
}

impl<'d> Terminal<'d> {
    pub fn new(uart: Uarte<'d, embassy_nrf::peripherals::UARTE1>) -> Self {
        Self {
            uart,
            tx_led: None,
            line_buffer: String::new(),
            cursor_pos: 0,
            command_history: Vec::new(),
            history_index: None,
            escape_state: EscapeState::Normal,
        }
    }

    pub fn with_tx_led(mut self, tx_led: Output<'d>) -> Self {
        self.tx_led = Some(tx_led);
        self
    }

    pub async fn write_str(&mut self, s: &str) -> Result<(), CliError> {
        // Flash TX LED during transmission if available
        if let Some(ref mut led) = self.tx_led {
            led.set_low(); // Turn on LED (active low)
        }

        // Send each character individually to debug transmission
        for &byte in s.as_bytes() {
            self.uart
                .write(&[byte])
                .await
                .map_err(|_| CliError::UartError)?;
        }

        // Small delay to make TX flash visible, then turn off TX LED
        if let Some(ref mut led) = self.tx_led {
            Timer::after(Duration::from_millis(10)).await;
            led.set_high(); // Turn off LED (active low)
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
        match self.escape_state {
            EscapeState::Normal => match ch {
                b'\r' | b'\n' => {
                    // Enter pressed - return the command
                    self.write_str("\r\n").await?;
                    let command = self.line_buffer.clone();

                    // Add to history if non-empty and different from last entry
                    if !command.is_empty() {
                        let should_add = self.command_history.is_empty()
                            || self.command_history.last() != Some(&command);

                        if should_add {
                            if self.command_history.len() >= HISTORY_SIZE {
                                self.command_history.remove(0);
                            }
                            let _ = self.command_history.push(command.clone());
                        }
                    }

                    self.line_buffer.clear();
                    self.cursor_pos = 0;
                    self.history_index = None;
                    Ok(Some(command))
                }
                b'\x1b' => {
                    // ESC - start escape sequence
                    self.escape_state = EscapeState::Escape;
                    Ok(None)
                }
                b'\x08' | b'\x7f' => {
                    // Backspace
                    if !self.line_buffer.is_empty() && self.cursor_pos > 0 {
                        self.delete_char_before_cursor().await?;
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
                    if self.line_buffer.len() < CLI_BUFFER_SIZE - 1 {
                        if let Ok(()) = self.insert_char_at_cursor(ch as char).await {
                            // Character inserted successfully
                        }
                    }
                    Ok(None)
                }
                _ => {
                    // Ignore other control characters
                    Ok(None)
                }
            },
            EscapeState::Escape => {
                match ch {
                    b'[' => {
                        // ESC[ - Control Sequence Introducer
                        self.escape_state = EscapeState::Csi;
                        Ok(None)
                    }
                    _ => {
                        // Unknown escape sequence, reset to normal
                        self.escape_state = EscapeState::Normal;
                        Ok(None)
                    }
                }
            }
            EscapeState::Csi => {
                match ch {
                    b'A' => {
                        // Up arrow - previous command in history
                        self.handle_history_up().await?;
                        self.escape_state = EscapeState::Normal;
                        Ok(None)
                    }
                    b'B' => {
                        // Down arrow - next command in history
                        self.handle_history_down().await?;
                        self.escape_state = EscapeState::Normal;
                        Ok(None)
                    }
                    b'C' => {
                        // Right arrow - move cursor right
                        self.handle_cursor_right().await?;
                        self.escape_state = EscapeState::Normal;
                        Ok(None)
                    }
                    b'D' => {
                        // Left arrow - move cursor left
                        self.handle_cursor_left().await?;
                        self.escape_state = EscapeState::Normal;
                        Ok(None)
                    }
                    _ => {
                        // Other CSI sequences, ignore for now
                        self.escape_state = EscapeState::Normal;
                        Ok(None)
                    }
                }
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
        self.write_line("  bt_scan [time] - Scan for BLE devices (default 10s)")
            .await?;
        self.write_line("  mtu_start [dur] - Start MTU operation (default 30s)")
            .await?;
        self.write_line("  mtu_stop    - Stop MTU operation")
            .await?;
        self.write_line("  mtu_status  - Show MTU status").await?;
        self.write_line("  mtu_baud <rate> - Set MTU baud rate (1-115200, default 1200)")
            .await?;
        self.write_line("").await?;
        self.write_line("Use TAB to autocomplete commands").await?;
        self.write_line("Use UP/DOWN arrows to navigate command history")
            .await?;
        self.write_line("Use LEFT/RIGHT arrows to move cursor and edit")
            .await?;
        Ok(())
    }

    pub async fn show_meter_help(&mut self) -> Result<(), CliError> {
        self.write_line("Water Meter Simulator Commands:").await?;
        self.write_line("  help        - Show this help").await?;
        self.write_line("  version     - Show simulator version")
            .await?;
        self.write_line("  status      - Show meter configuration")
            .await?;
        self.write_line("  clear       - Clear terminal").await?;
        self.write_line("  type <sensus|neptune> - Set meter type")
            .await?;
        self.write_line("  message <text> - Set response message")
            .await?;
        self.write_line("  enable      - Enable meter responses")
            .await?;
        self.write_line("  disable     - Disable meter responses")
            .await?;
        self.write_line("").await?;
        self.write_line("Examples:").await?;
        self.write_line("  type sensus").await?;
        self.write_line("  message WATER12345").await?;
        self.write_line("").await?;
        self.write_line("Pin Configuration:").await?;
        self.write_line("  P0.02 - Clock input (from MTU)").await?;
        self.write_line("  P0.03 - Data output (to MTU)").await?;

        Ok(())
    }

    async fn handle_history_up(&mut self) -> Result<(), CliError> {
        if self.command_history.is_empty() {
            return Ok(());
        }

        let new_index = match self.history_index {
            None => self.command_history.len() - 1,
            Some(current) => {
                if current > 0 {
                    current - 1
                } else {
                    return Ok(()); // Already at oldest command
                }
            }
        };

        self.history_index = Some(new_index);
        self.replace_current_line(&self.command_history[new_index].clone())
            .await
    }

    async fn handle_history_down(&mut self) -> Result<(), CliError> {
        let new_index = match self.history_index {
            None => return Ok(()), // Not in history mode
            Some(current) => {
                if current < self.command_history.len() - 1 {
                    Some(current + 1)
                } else {
                    None // Back to empty line
                }
            }
        };

        self.history_index = new_index;

        match new_index {
            Some(idx) => {
                self.replace_current_line(&self.command_history[idx].clone())
                    .await
            }
            None => {
                // Clear line - back to empty
                let empty_line = String::new();
                self.replace_current_line(&empty_line).await
            }
        }
    }

    async fn replace_current_line(
        &mut self,
        new_line: &String<CLI_BUFFER_SIZE>,
    ) -> Result<(), CliError> {
        // Clear current line
        for _ in 0..self.cursor_pos {
            self.write_str("\x08 \x08").await?;
        }

        // Update buffer and cursor
        self.line_buffer.clear();
        let _ = self.line_buffer.push_str(new_line);
        self.cursor_pos = new_line.len();

        // Display new line
        self.write_str(new_line).await
    }

    async fn handle_cursor_right(&mut self) -> Result<(), CliError> {
        if self.cursor_pos < self.line_buffer.len() {
            self.cursor_pos += 1;
            // Send ANSI escape sequence to move cursor right
            self.write_str("\x1b[C").await?;
        }
        Ok(())
    }

    async fn handle_cursor_left(&mut self) -> Result<(), CliError> {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            // Send ANSI escape sequence to move cursor left
            self.write_str("\x1b[D").await?;
        }
        Ok(())
    }

    async fn insert_char_at_cursor(&mut self, ch: char) -> Result<(), CliError> {
        if self.cursor_pos == self.line_buffer.len() {
            // Simple case: inserting at end
            if self.line_buffer.push(ch).is_ok() {
                self.cursor_pos += 1;
                // Echo the character
                let echo = [ch as u8];
                self.uart
                    .write(&echo)
                    .await
                    .map_err(|_| CliError::UartError)?;
            }
        } else {
            // Complex case: inserting in middle - need to rebuild string
            let mut new_buffer = String::new();

            // Copy characters before cursor
            for (i, existing_ch) in self.line_buffer.chars().enumerate() {
                if i == self.cursor_pos {
                    // Insert new character at cursor position
                    if new_buffer.push(ch).is_err() {
                        return Err(CliError::BufferFull);
                    }
                }
                if new_buffer.push(existing_ch).is_err() {
                    return Err(CliError::BufferFull);
                }
            }

            // If cursor is at the end, we still need to add the character
            #[allow(clippy::collapsible_if)]
            if self.cursor_pos == self.line_buffer.chars().count() {
                if new_buffer.push(ch).is_err() {
                    return Err(CliError::BufferFull);
                }
            }

            // Replace the buffer
            self.line_buffer = new_buffer;
            self.cursor_pos += 1;

            // Redraw from cursor position to end of line
            self.redraw_line_from_cursor().await?;
        }
        Ok(())
    }

    async fn redraw_line_from_cursor(&mut self) -> Result<(), CliError> {
        // Save current cursor position
        let saved_cursor = self.cursor_pos;

        // Get the part of the line from current cursor to end
        let chars_to_redraw: heapless::Vec<char, CLI_BUFFER_SIZE> =
            self.line_buffer.chars().skip(saved_cursor - 1).collect();

        // Write the characters from cursor position onward
        for ch in chars_to_redraw.iter() {
            let echo = [*ch as u8];
            self.uart
                .write(&echo)
                .await
                .map_err(|_| CliError::UartError)?;
        }

        // Move cursor back to correct position
        let chars_written = chars_to_redraw.len();
        if chars_written > 1 {
            // Move cursor back (chars_written - 1) positions
            for _ in 1..chars_written {
                self.write_str("\x1b[D").await?;
            }
        }

        Ok(())
    }

    async fn delete_char_before_cursor(&mut self) -> Result<(), CliError> {
        if self.cursor_pos == self.line_buffer.len() {
            // Simple case: deleting from end
            self.line_buffer.pop();
            self.cursor_pos -= 1;
            // Send backspace sequence: backspace + space + backspace
            self.write_str("\x08 \x08").await?;
        } else {
            // Complex case: deleting from middle - need to rebuild string
            let mut new_buffer = String::new();

            // Copy all characters except the one before cursor
            for (i, ch) in self.line_buffer.chars().enumerate() {
                if i != self.cursor_pos - 1 {
                    // Skip the character before cursor position
                    if new_buffer.push(ch).is_err() {
                        return Err(CliError::BufferFull);
                    }
                }
            }

            // Replace the buffer
            self.line_buffer = new_buffer;
            self.cursor_pos -= 1;

            // Move cursor left, then redraw from current position to end
            self.write_str("\x1b[D").await?; // Move cursor left
            self.redraw_line_from_cursor_with_clear().await?;
        }
        Ok(())
    }

    async fn redraw_line_from_cursor_with_clear(&mut self) -> Result<(), CliError> {
        // Save current cursor position
        let saved_cursor = self.cursor_pos;

        // Get the part of the line from current cursor to end
        let chars_to_redraw: heapless::Vec<char, CLI_BUFFER_SIZE> =
            self.line_buffer.chars().skip(saved_cursor).collect();

        // Write the characters from cursor position onward
        for ch in chars_to_redraw.iter() {
            let echo = [*ch as u8];
            self.uart
                .write(&echo)
                .await
                .map_err(|_| CliError::UartError)?;
        }

        // Clear the extra character that was there before
        self.write_str(" ").await?;

        // Move cursor back to correct position
        let total_chars_written = chars_to_redraw.len() + 1; // +1 for the space
        for _ in 0..total_chars_written {
            self.write_str("\x1b[D").await?;
        }

        Ok(())
    }
}
