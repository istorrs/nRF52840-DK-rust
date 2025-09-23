use super::config::MtuConfig;
use super::error::MtuResult;
use super::uart_framing::{extract_char_from_frame, UartFrame};
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::{error, info, warn};
use embassy_nrf::gpio::{Input, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;

use super::error::MtuError;

// Communication structure between critical timing tasks and LED/logging task
#[derive(Clone, Copy)]
pub struct MtuEvent {
    pub clock_cycle: u64,
    pub event_type: MtuEventType,
    pub time_delta_micros: u64,
}

#[derive(Clone, Copy)]
pub enum MtuEventType {
    // Operation lifecycle events
    OperationStart,
    OperationEnd,

    // Critical timing events (minimal data for speed)
    ClockCycle { bit_value: u8 },

    // Data processing events with logging details
    CharacterReceived { ch: char, ascii: u8, message_len: usize },
    MessageComplete { length: usize },
    PartialMessage { length: usize },

    // Error events with diagnostic data
    FramingError {
        error_type: MtuError,
        frame_bits_len: usize,
        start_bit: u8,
        parity_bit: u8,
        stop_bit: u8,
        data_ones_count: usize,
        expected_parity: u8,
    },

    // LED control events (minimal)
    ClockHigh,
    ClockLow,
    DataBit(u8),
}

pub struct GpioMtu {
    config: Mutex<ThreadModeRawMutex, MtuConfig>,
    running: AtomicBool,
    last_message: Mutex<ThreadModeRawMutex, Option<String<256>>>,
    // Channel for communication between clock and data tasks
    bit_channel: Channel<ThreadModeRawMutex, u8, 64>,
    // Sender for LED events (receiver is owned by LED task)
    led_event_sender: Option<Sender<'static, ThreadModeRawMutex, MtuEvent, 32>>,
    clock_cycle_counter: Mutex<ThreadModeRawMutex, u64>,
}

impl GpioMtu {
    pub fn new(config: MtuConfig) -> Self {
        Self {
            config: Mutex::new(config),
            running: AtomicBool::new(false),
            last_message: Mutex::new(None),
            bit_channel: Channel::new(),
            led_event_sender: None,
            clock_cycle_counter: Mutex::new(0),
        }
    }

    // Set the LED event sender (called from main after creating static channel)
    pub fn set_led_event_sender(
        &mut self,
        sender: Sender<'static, ThreadModeRawMutex, MtuEvent, 32>,
    ) {
        self.led_event_sender = Some(sender);
    }

    pub async fn set_baud_rate(&self, baud_rate: u32) {
        let mut config = self.config.lock().await;
        config.baud_rate = baud_rate;
        info!("MTU: Baud rate set to {}", baud_rate);
    }

    pub async fn get_baud_rate(&self) -> u32 {
        let config = self.config.lock().await;
        config.baud_rate
    }

    pub async fn get_config(&self) -> MtuConfig {
        let config = self.config.lock().await;
        config.clone()
    }

    pub async fn start(&self) -> MtuResult<()> {
        self.running.store(true, Ordering::Relaxed);
        info!("MTU: Starting operation");
        Ok(())
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        info!("MTU: Stopping operation");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub async fn get_last_message(&self) -> Option<String<256>> {
        let msg = self.last_message.lock().await;
        msg.clone()
    }

    pub async fn clear_last_message(&self) {
        let mut msg = self.last_message.lock().await;
        *msg = None;
    }

    pub async fn set_expected_message(&self, expected: String<256>) {
        let mut config = self.config.lock().await;
        info!("MTU: Expected message set to: {}", expected.as_str());
        config.expected_message = expected;
    }

    pub async fn get_expected_message(&self) -> String<256> {
        let config = self.config.lock().await;
        config.expected_message.clone()
    }

    pub async fn get_stats(&self) -> (u32, u32) {
        let config = self.config.lock().await;
        (config.successful_reads, config.corrupted_reads)
    }

    pub async fn reset_stats(&self) {
        let mut config = self.config.lock().await;
        config.successful_reads = 0;
        config.corrupted_reads = 0;
        info!("MTU: Statistics reset");
    }

    // Helper method to evaluate and record a message result
    async fn record_message_result(&self, received_message: Option<String<256>>) -> bool {
        let mut config = self.config.lock().await;
        let expected = config.expected_message.clone();

        if let Some(received) = received_message {
            if received == expected {
                config.successful_reads += 1;
                info!(
                    "MTU: Message SUCCESS - Stats: {}/{}",
                    config.successful_reads,
                    config.successful_reads + config.corrupted_reads
                );
                true
            } else {
                config.corrupted_reads += 1;
                error!(
                    "MTU: Message CORRUPTED - Expected: '{}', Received: '{}' - Stats: {}/{}",
                    expected.as_str(),
                    received.as_str(),
                    config.successful_reads,
                    config.successful_reads + config.corrupted_reads
                );

                // Show character-by-character differences
                let expected_chars: heapless::Vec<char, 256> = expected.chars().collect();
                let received_chars: heapless::Vec<char, 256> = received.chars().collect();
                let min_len = expected_chars.len().min(received_chars.len());

                error!(
                    "MTU: Length comparison - Expected: {}, Received: {}",
                    expected_chars.len(),
                    received_chars.len()
                );

                for (i, (&exp_char, &rec_char)) in
                    expected_chars.iter().zip(received_chars.iter()).enumerate()
                {
                    if exp_char != rec_char {
                        error!(
                            "MTU: Character mismatch at position {}: Expected '{}' (ASCII {}), Received '{}' (ASCII {})",
                            i, exp_char, exp_char as u8, rec_char, rec_char as u8
                        );
                    }
                }

                // Report extra characters if lengths differ
                if expected_chars.len() > received_chars.len() {
                    for i in min_len..expected_chars.len() {
                        error!(
                            "MTU: Missing character at position {}: Expected '{}' (ASCII {})",
                            i, expected_chars[i], expected_chars[i] as u8
                        );
                    }
                } else if received_chars.len() > expected_chars.len() {
                    for i in min_len..received_chars.len() {
                        error!(
                            "MTU: Extra character at position {}: Received '{}' (ASCII {})",
                            i, received_chars[i], received_chars[i] as u8
                        );
                    }
                }

                false
            }
        } else {
            config.corrupted_reads += 1;
            error!(
                "MTU: Message CORRUPTED - No message received - Stats: {}/{}",
                config.successful_reads,
                config.successful_reads + config.corrupted_reads
            );
            false
        }
    }

    // Test function that runs MTU operations multiple times and compares results
    pub async fn run_test(
        &self,
        iterations: u16,
        clock_pin: &mut Output<'_>,
        data_pin: &Input<'_>,
    ) -> MtuResult<(u16, u16)> {
        // Returns (successful_messages, corrupted_messages)
        info!("MTU: Starting test with {} iterations", iterations);

        let mut successful_messages = 0u16;
        let mut corrupted_messages = 0u16;
        let expected_message = self.get_expected_message().await;

        info!("MTU: Expected message: {}", expected_message.as_str());

        for iteration in 1..=iterations {
            info!("MTU: Test iteration {}/{}", iteration, iterations);

            // Clear any previous message
            self.clear_last_message().await;

            // Run single MTU operation (shorter duration for testing)
            let test_duration = Duration::from_secs(10); // 10 seconds per test

            match self
                .run_mtu_operation(test_duration, clock_pin, data_pin)
                .await
            {
                Ok(_) => {
                    // Check if we received a message and record the result
                    let received_message = self.get_last_message().await;
                    if self.record_message_result(received_message.clone()).await {
                        successful_messages += 1;
                        info!(
                            "MTU: Test {}: SUCCESS - Message matches expected",
                            iteration
                        );
                    } else {
                        corrupted_messages += 1;
                        if let Some(received) = received_message {
                            error!(
                                "MTU: Test {}/{}: FAILED - Received: '{}', Expected: '{}'",
                                iteration,
                                iterations,
                                received.as_str(),
                                expected_message.as_str()
                            );
                        } else {
                            error!(
                                "MTU: Test {}/{}: FAILED - No message received",
                                iteration, iterations
                            );
                        }
                    }
                }
                Err(e) => {
                    corrupted_messages += 1;
                    error!("MTU: Test {}: ERROR - Operation failed: {:?}", iteration, e);
                }
            }

            // Small delay between tests
            Timer::after(Duration::from_millis(500)).await;
        }

        info!(
            "MTU: Test completed - {}/{} successful, {}/{} corrupted",
            successful_messages, iterations, corrupted_messages, iterations
        );

        Ok((successful_messages, corrupted_messages))
    }

    // MTU operation with statistics tracking (wrapper for manual operations)
    pub async fn run_mtu_operation_with_stats(
        &self,
        duration: Duration,
        clock_pin: &mut Output<'_>,
        data_pin: &Input<'_>,
    ) -> MtuResult<()> {
        // Run the actual MTU operation
        let result = self.run_mtu_operation(duration, clock_pin, data_pin).await;

        // Record statistics for this operation
        let received_message = self.get_last_message().await;
        self.record_message_result(received_message).await;

        result
    }

    // MTU operation implementing RPI architecture with separated clock and data tasks
    pub async fn run_mtu_operation(
        &self,
        duration: Duration,
        clock_pin: &mut Output<'_>,
        data_pin: &Input<'_>,
    ) -> MtuResult<()> {
        use embassy_futures::select::{select, Either};

        let config = self.config.lock().await;
        let power_up_delay_ms = config.power_up_delay_ms;
        drop(config);

        info!("MTU: Starting RPI-style meter reading for {:?}", duration);

        // Set running flag for the duration of this operation
        self.running.store(true, Ordering::Relaxed);

        let start_time = Instant::now();

        // Power up sequence: Set clock HIGH and hold for power_up_delay_ms
        clock_pin.set_high();
        info!(
            "MTU: Setting clock HIGH for {}ms power-up hold period",
            power_up_delay_ms
        );
        Timer::after(Duration::from_millis(power_up_delay_ms)).await;
        info!("MTU: Power-up hold complete, starting clock and data tasks");

        // Send operation start event
        if let Some(ref led_sender) = self.led_event_sender {
            let start_event = MtuEvent {
                clock_cycle: 0,
                event_type: MtuEventType::OperationStart,
                time_delta_micros: 0,
            };
            let _ = led_sender.try_send(start_event);
        }

        // Get channel senders/receivers
        let bit_sender = self.bit_channel.sender();
        let bit_receiver = self.bit_channel.receiver();

        // Run both tasks until timeout or completion
        let timeout_task = Timer::after(duration);

        // Start both tasks and wait for completion
        let result = {
            let clock_task = self.clock_task(clock_pin, data_pin, bit_sender);
            let data_task = self.data_task(bit_receiver, start_time);

            select(select(clock_task, data_task), timeout_task).await
        };

        match result {
            Either::First(Either::First(_)) => {
                info!("MTU: Clock task completed");
            }
            Either::First(Either::Second(_)) => {
                info!("MTU: Data task completed (message received)");
            }
            Either::Second(_) => {
                warn!("MTU: Operation timeout reached");
            }
        }

        // Set clock to idle state (HIGH)
        clock_pin.set_high();

        // Send operation end event
        if let Some(ref led_sender) = self.led_event_sender {
            let end_event = MtuEvent {
                clock_cycle: 0,
                event_type: MtuEventType::OperationEnd,
                time_delta_micros: start_time.elapsed().as_micros(),
            };
            let _ = led_sender.try_send(end_event);
        }

        // Clear running flag
        self.running.store(false, Ordering::Relaxed);

        info!("MTU: Operation completed after {:?}", start_time.elapsed());
        Ok(())
    }

    // Clock task - equivalent to RPI clock thread (lines 225-307)
    async fn clock_task(
        &self,
        clock_pin: &mut Output<'_>,
        data_pin: &Input<'_>,
        bit_sender: Sender<'_, ThreadModeRawMutex, u8, 64>,
    ) -> MtuResult<()> {

        let mut clock_cycle_count = 0u64;
        let mut last_clock_time = Instant::now();

        while self.running.load(Ordering::Relaxed) {
            clock_cycle_count += 1;
            let timestamp = Instant::now();
            let time_delta = timestamp.duration_since(last_clock_time);
            last_clock_time = timestamp;

            // Get cycle timing from config
            let config = self.config.lock().await;
            let cycle_duration = config.bit_duration();
            drop(config);
            let half_cycle = cycle_duration / 2;

            // Clock LOW phase
            clock_pin.set_low();

            // Send clock low event for LED
            if let Some(ref led_sender) = self.led_event_sender {
                let clock_low_event = MtuEvent {
                    clock_cycle: clock_cycle_count,
                    event_type: MtuEventType::ClockLow,
                    time_delta_micros: time_delta.as_micros(),
                };
                let _ = led_sender.try_send(clock_low_event);
            }

            Timer::after(half_cycle).await;

            let data_val = data_pin.is_high();
            let data_bit = if data_val { 1 } else { 0 };

            // Send sampled bit to data processing task (like RPI line 293-295)
            if bit_sender.try_send(data_bit).is_err() {
                // Critical: don't log here, send error event instead
                if let Some(ref led_sender) = self.led_event_sender {
                    let error_event = MtuEvent {
                        clock_cycle: clock_cycle_count,
                        event_type: MtuEventType::FramingError {
                            error_type: MtuError::ChannelError,
                            frame_bits_len: 0,
                            start_bit: 0,
                            parity_bit: 0,
                            stop_bit: 0,
                            data_ones_count: 0,
                            expected_parity: 0,
                        },
                        time_delta_micros: time_delta.as_micros(),
                    };
                    let _ = led_sender.try_send(error_event);
                }
            }

            // Send clock cycle event (combines timing info and bit data)
            if let Some(ref led_sender) = self.led_event_sender {
                let cycle_event = MtuEvent {
                    clock_cycle: clock_cycle_count,
                    event_type: MtuEventType::ClockCycle { bit_value: data_bit },
                    time_delta_micros: time_delta.as_micros(),
                };
                let _ = led_sender.try_send(cycle_event);
            }

            // Clock HIGH phase
            clock_pin.set_high();

            // Send clock high event for LED
            if let Some(ref led_sender) = self.led_event_sender {
                let clock_high_event = MtuEvent {
                    clock_cycle: clock_cycle_count,
                    event_type: MtuEventType::ClockHigh,
                    time_delta_micros: 0,
                };
                let _ = led_sender.try_send(clock_high_event);
            }

            // Update cycle counter
            {
                let mut counter = self.clock_cycle_counter.lock().await;
                *counter = clock_cycle_count;
            }

            Timer::after(half_cycle).await;
        }

        Ok(())
    }

    // Data processing task - equivalent to RPI data thread (lines 325-471)
    async fn data_task(
        &self,
        bit_receiver: Receiver<'_, ThreadModeRawMutex, u8, 64>,
        _start_time: Instant,
    ) -> MtuResult<()> {

        let mut received_chars = heapless::Vec::<char, 256>::new();

        while self.running.load(Ordering::Relaxed) {
            // Wait for start bit (0) - like RPI lines 341-366
            let mut start_bit = None;
            while self.running.load(Ordering::Relaxed) {
                match bit_receiver.receive().await {
                    0 => {
                        start_bit = Some(0);
                        break;
                    }
                    1 => {
                        // Skip high idle bits (like RPI lines 351-353)
                        continue;
                    }
                    _ => {
                        // Invalid bit value, skip
                        continue;
                    }
                }
            }

            if !self.running.load(Ordering::Relaxed) {
                break;
            }

            let _cycle_count = {
                let counter = self.clock_cycle_counter.lock().await;
                *counter
            };

            // Collect complete frame - like RPI lines 375-402
            let config = self.config.lock().await;
            let frame_size = match config.framing {
                crate::mtu::config::UartFraming::SevenE1 => 10, // 1 start + 7 data + 1 parity + 1 stop
                crate::mtu::config::UartFraming::SevenE2 => 11, // 1 start + 7 data + 1 parity + 2 stop
            };
            drop(config);

            let mut frame_bits = heapless::Vec::<u8, 16>::new();
            let _ = frame_bits.push(start_bit.unwrap());

            // Receive remaining bits with timeout
            let mut bits_received = 1;
            while bits_received < frame_size && self.running.load(Ordering::Relaxed) {
                // Use select with timeout like RPI lines 383-401
                use embassy_futures::select::{select, Either};
                let bit_timeout = Timer::after(Duration::from_secs(2)); // 2 second timeout for slow baud rates

                match select(bit_receiver.receive(), bit_timeout).await {
                    Either::First(bit_val) => {
                        let _ = frame_bits.push(bit_val);
                        bits_received += 1;
                        //info!("MTU: UART bit {}: {}", bits_received - 1, bit_val);
                    }
                    Either::Second(_) => {
                        break;
                    }
                }
            }

            if bits_received != frame_size {
                continue;
            }

            // Process the complete frame (like RPI lines 409-463)
            let config = self.config.lock().await;
            let framing = config.framing;
            drop(config);

            match UartFrame::new(frame_bits.clone(), framing) {
                Ok(frame) => {
                    match extract_char_from_frame(&frame) {
                        Ok(ch) => {
                            let _ = received_chars.push(ch);

                            // Send character received event
                            if let Some(ref led_sender) = self.led_event_sender {
                                let char_event = MtuEvent {
                                    clock_cycle: 0, // Data task doesn't track cycles
                                    event_type: MtuEventType::CharacterReceived {
                                        ch,
                                        ascii: ch as u8,
                                        message_len: received_chars.len(),
                                    },
                                    time_delta_micros: 0,
                                };
                                let _ = led_sender.try_send(char_event);
                            }

                            // Check for end of message (carriage return)
                            if ch == '\r' {
                                let message: String<256> = received_chars.iter().collect();

                                // Send message complete event
                                if let Some(ref led_sender) = self.led_event_sender {
                                    let complete_event = MtuEvent {
                                        clock_cycle: 0,
                                        event_type: MtuEventType::MessageComplete {
                                            length: message.len(),
                                        },
                                        time_delta_micros: 0,
                                    };
                                    let _ = led_sender.try_send(complete_event);
                                }

                                // Store the received message
                                {
                                    let mut last_msg = self.last_message.lock().await;
                                    *last_msg = Some(message);
                                }

                                received_chars.clear();
                                break; // Exit to stop receiving (like RPI line 455)
                            }
                        }
                        Err(e) => {
                            // Send detailed frame error event for background logging
                            if let Some(ref led_sender) = self.led_event_sender {
                                // Calculate frame analysis data
                                let (start_bit, parity_bit, stop_bit, data_ones_count, expected_parity) = if frame_bits.len() >= 10 {
                                    let start = frame_bits[0];
                                    let data_bits = &frame_bits[1..8];
                                    let parity = frame_bits[8];
                                    let stop = frame_bits[9];
                                    let ones = data_bits.iter().filter(|&&bit| bit == 1).count();
                                    let exp_parity = if ones % 2 == 0 { 0 } else { 1 };
                                    (start, parity, stop, ones, exp_parity)
                                } else {
                                    (0, 0, 0, 0, 0)
                                };

                                let error_event = MtuEvent {
                                    clock_cycle: 0,
                                    event_type: MtuEventType::FramingError {
                                        error_type: e,
                                        frame_bits_len: frame_bits.len(),
                                        start_bit,
                                        parity_bit,
                                        stop_bit,
                                        data_ones_count,
                                        expected_parity,
                                    },
                                    time_delta_micros: 0,
                                };
                                let _ = led_sender.try_send(error_event);
                            }
                            continue;
                        }
                    }
                }
                Err(e) => {
                    // Send frame creation error event
                    if let Some(ref led_sender) = self.led_event_sender {
                        let error_event = MtuEvent {
                            clock_cycle: 0,
                            event_type: MtuEventType::FramingError {
                                error_type: e,
                                frame_bits_len: frame_bits.len(),
                                start_bit: 0,
                                parity_bit: 0,
                                stop_bit: 0,
                                data_ones_count: 0,
                                expected_parity: 0,
                            },
                            time_delta_micros: 0,
                        };
                        let _ = led_sender.try_send(error_event);
                    }
                    continue;
                }
            }
        }

        // Report any partial message when stopping
        if !received_chars.is_empty() {
            if let Some(ref led_sender) = self.led_event_sender {
                let partial_event = MtuEvent {
                    clock_cycle: 0,
                    event_type: MtuEventType::PartialMessage {
                        length: received_chars.len(),
                    },
                    time_delta_micros: 0,
                };
                let _ = led_sender.try_send(partial_event);
            }
        }
        Ok(())
    }
}

// MTU LED and logging task - handles LED control and all logging for MTU operations
// Follows meter pattern: critical timing tasks send events, background task handles logging
pub async fn run_mtu_led_logging_task(
    event_receiver: Receiver<'_, ThreadModeRawMutex, MtuEvent, 32>,
    mut clock_led: Output<'_>,
    mut data_led: Output<'_>,
) -> ! {
    info!("MTU: LED and logging task started");

    loop {
        // Wait for events from MTU operations
        let event = event_receiver.receive().await;

        match event.event_type {
            MtuEventType::OperationStart => {
                info!("MTU: LED task - operation started");
                // Both LEDs off at start
                clock_led.set_high();
                data_led.set_high();
            }
            MtuEventType::OperationEnd => {
                info!("MTU: LED task - operation ended");
                // Both LEDs off at end
                clock_led.set_high();
                data_led.set_high();
            }

            // Critical timing events with logging
            MtuEventType::ClockCycle { bit_value } => {
                info!(
                    "MTU: CLK #{} TICK {} - RX bit {}",
                    event.clock_cycle,
                    event.time_delta_micros,
                    bit_value
                );

                // Data LED reflects the received bit value (no delays)
                if bit_value == 1 {
                    data_led.set_low();   // LED on for bit = 1
                } else {
                    data_led.set_high();  // LED off for bit = 0
                }
            }

            // Data processing events with detailed logging
            MtuEventType::CharacterReceived { ch, ascii, message_len } => {
                info!(
                    "MTU: UART frame -> char: {:?} (ASCII {}) - Message length: {}",
                    ch, ascii, message_len
                );
            }

            MtuEventType::MessageComplete { length } => {
                info!("MTU: Received complete message (length: {})", length);
            }

            MtuEventType::PartialMessage { length } => {
                error!("MTU: Data task stopped with {} partial characters", length);
            }

            // Detailed error logging with frame analysis
            MtuEventType::FramingError {
                error_type,
                frame_bits_len,
                start_bit,
                parity_bit,
                stop_bit,
                data_ones_count,
                expected_parity,
            } => {
                match error_type {
                    MtuError::ChannelError => {
                        warn!("MTU: Bit queue full, dropping bit");
                    }
                    _ => {
                        error!("MTU: UART framing error: {:?}", error_type);

                        if frame_bits_len >= 10 {
                            error!(
                                "MTU: Frame analysis - Start:{} Data:{} Parity:{} (exp:{}) Stop:{} [length:{}]",
                                start_bit, data_ones_count, parity_bit, expected_parity, stop_bit, frame_bits_len
                            );
                        } else {
                            error!("MTU: Invalid frame length: {} bits", frame_bits_len);
                        }
                    }
                }
            }

            // Legacy LED events (for compatibility)
            MtuEventType::ClockHigh => {
                clock_led.set_high();  // Clock LED on when clock is high
            }
            MtuEventType::ClockLow => {
                clock_led.set_low();   // Clock LED off when clock is low  
            }
            MtuEventType::DataBit(bit) => {
                if bit == 1 {
                    data_led.set_low();   // Data LED on for bit = 1
                } else {
                    data_led.set_high();  // Data LED off for bit = 0
                }
            }
        }
    }
}
