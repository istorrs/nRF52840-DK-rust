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

// Communication structure between clock/data tasks and LED task
#[derive(Clone, Copy)]
pub struct MtuEvent {
    pub clock_cycle: u64,
    pub event_type: MtuEventType,
    pub time_delta_micros: u64,
}

#[derive(Clone, Copy)]
pub enum MtuEventType {
    ClockHigh,
    ClockLow,
    DataBit(u8),
    OperationStart,
    OperationEnd,
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
        info!("MTU: Clock task started (equivalent to RPI clock thread)");

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
            // Send clock low event
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

            info!(
                "MTU: CLK #{} TICK {} - RX bit {}",
                clock_cycle_count,
                time_delta.as_micros(),
                data_bit
            );

            // Send sampled bit to data processing task (like RPI line 293-295)
            if bit_sender.try_send(data_bit).is_err() {
                warn!("MTU: Bit queue full, dropping bit");
            }

            // Send data bit event
            if let Some(ref led_sender) = self.led_event_sender {
                let data_event = MtuEvent {
                    clock_cycle: clock_cycle_count,
                    event_type: MtuEventType::DataBit(data_bit),
                    time_delta_micros: time_delta.as_micros(),
                };
                let _ = led_sender.try_send(data_event);
            }

            // Clock HIGH phase
            clock_pin.set_high();
            // Send clock high event
            if let Some(ref led_sender) = self.led_event_sender {
                let clock_high_event = MtuEvent {
                    clock_cycle: clock_cycle_count,
                    event_type: MtuEventType::ClockHigh,
                    time_delta_micros: time_delta.as_micros(),
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

        info!("MTU: Clock task stopped");
        Ok(())
    }

    // Data processing task - equivalent to RPI data thread (lines 325-471)
    async fn data_task(
        &self,
        bit_receiver: Receiver<'_, ThreadModeRawMutex, u8, 64>,
        _start_time: Instant,
    ) -> MtuResult<()> {
        info!("MTU: Data task started (equivalent to RPI data thread)");

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

            let cycle_count = {
                let counter = self.clock_cycle_counter.lock().await;
                *counter
            };
            info!("MTU: UART start bit detected (cycle={})", cycle_count);

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
                        warn!("MTU: Timeout waiting for UART bit {}", bits_received);
                        break;
                    }
                }
            }

            if bits_received != frame_size {
                warn!(
                    "MTU: Incomplete frame received ({}/{} bits)",
                    bits_received, frame_size
                );
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
                            info!(
                                "MTU: UART frame -> char: {:?} (ASCII {}) - Message length: {}",
                                ch as char,
                                ch as u8,
                                received_chars.len()
                            );

                            // Check for end of message (carriage return)
                            if ch == '\r' {
                                let message: String<256> = received_chars.iter().collect();
                                info!(
                                    "MTU: Received complete message: {:?} (length: {})",
                                    message.as_str(),
                                    message.len()
                                );

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
                            // Show detailed frame analysis for debugging
                            let frame_str: heapless::String<64> = frame_bits
                                .iter()
                                .map(|&b| if b == 1 { '1' } else { '0' })
                                .collect();
                            error!("MTU: UART framing error: {:?}", e);
                            error!("MTU: Frame bits [S|D7..D1|P|T]: {}", frame_str.as_str());

                            // Additional analysis for common errors
                            if frame_bits.len() == 10 {
                                let start_bit = frame_bits[0];
                                let data_bits = &frame_bits[1..8];
                                let parity_bit = frame_bits[8];
                                let stop_bit = frame_bits[9];
                                let data_ones = data_bits.iter().filter(|&&bit| bit == 1).count();
                                let expected_parity = if data_ones % 2 == 0 { 0 } else { 1 };

                                error!("MTU: Frame analysis - Start:{} Data:{} Parity:{} (exp:{}) Stop:{}",
                                       start_bit, data_ones, parity_bit, expected_parity, stop_bit);
                            }
                            continue;
                        }
                    }
                }
                Err(e) => {
                    error!("MTU: Invalid UART frame: {:?}", e);
                    error!("MTU: Invalid frame bits: {:?}", frame_bits.as_slice());
                    continue;
                }
            }
        }

        // Report any partial message when stopping
        if !received_chars.is_empty() {
            let partial_message: String<256> = received_chars.iter().collect();
            error!(
                "MTU: Data task stopped with {} partial characters: '{}'",
                received_chars.len(),
                partial_message.as_str()
            );
        } else {
            info!("MTU: Data task stopped (no partial characters)");
        }
        Ok(())
    }
}

// Standalone LED task function that can be used without holding MTU mutex
pub async fn run_mtu_led_task(
    event_receiver: Receiver<'_, ThreadModeRawMutex, MtuEvent, 32>,
    mut clock_led: Output<'_>,
    mut data_led: Output<'_>,
) -> ! {
    info!("MTU: LED task started");

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
            MtuEventType::ClockHigh => {
                // Clock LED on during clock high (brief flash)
                clock_led.set_low();
                // Use a very short flash at high speeds to be visible
                embassy_time::Timer::after(embassy_time::Duration::from_millis(2)).await;
                clock_led.set_high();
            }
            MtuEventType::ClockLow => {
                // Clock LED off during clock low (already off from previous cycle)
                clock_led.set_high();
            }
            MtuEventType::DataBit(bit) => {
                // Data LED indicates received data activity
                if bit == 1 {
                    data_led.set_low(); // LED on for data bit 1
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(5)).await;
                    data_led.set_high(); // LED off
                }
                // No LED change for data bit 0 (idle state)
            }
        }
    }
}
