use super::config::MtuConfig;
use super::error::MtuResult;
use super::uart_framing::{extract_char_from_frame, UartFrame};
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::info;
use embassy_nrf::gpio::{Input, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;

pub struct GpioMtu {
    config: Mutex<ThreadModeRawMutex, MtuConfig>,
    running: AtomicBool,
    last_message: Mutex<ThreadModeRawMutex, Option<String<256>>>,
    // Channel for communication between clock and data tasks
    bit_channel: Channel<ThreadModeRawMutex, u8, 64>,
    clock_cycle_counter: Mutex<ThreadModeRawMutex, u64>,
}

impl GpioMtu {
    pub fn new(config: MtuConfig) -> Self {
        Self {
            config: Mutex::new(config),
            running: AtomicBool::new(false),
            last_message: Mutex::new(None),
            bit_channel: Channel::new(),
            clock_cycle_counter: Mutex::new(0),
        }
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
                info!(
                    "MTU: Message CORRUPTED - Expected: '{}', Received: '{}' - Stats: {}/{}",
                    expected.as_str(),
                    received.as_str(),
                    config.successful_reads,
                    config.successful_reads + config.corrupted_reads
                );
                false
            }
        } else {
            config.corrupted_reads += 1;
            info!(
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
        mut clock_led: Option<&mut Output<'_>>,
        mut data_led: Option<&mut Output<'_>>,
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
                .run_mtu_operation(
                    test_duration,
                    clock_pin,
                    data_pin,
                    clock_led.as_deref_mut(),
                    data_led.as_deref_mut(),
                )
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
                            info!(
                                "MTU: Test {}: CORRUPTED - Received: '{}', Expected: '{}'",
                                iteration,
                                received.as_str(),
                                expected_message.as_str()
                            );
                        } else {
                            info!("MTU: Test {}: CORRUPTED - No message received", iteration);
                        }
                    }
                }
                Err(e) => {
                    corrupted_messages += 1;
                    info!("MTU: Test {}: ERROR - Operation failed: {:?}", iteration, e);
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
        clock_led: Option<&mut Output<'_>>,
        data_led: Option<&mut Output<'_>>,
    ) -> MtuResult<()> {
        // Run the actual MTU operation
        let result = self
            .run_mtu_operation(duration, clock_pin, data_pin, clock_led, data_led)
            .await;

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
        mut clock_led: Option<&mut Output<'_>>,
        mut data_led: Option<&mut Output<'_>>,
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

        // Get channel senders/receivers
        let bit_sender = self.bit_channel.sender();
        let bit_receiver = self.bit_channel.receiver();

        // Run both tasks until timeout or completion
        let timeout_task = Timer::after(duration);

        // Start both tasks and wait for completion
        let result = {
            let clock_task =
                self.clock_task(clock_pin, data_pin, clock_led.as_deref_mut(), bit_sender);
            let data_task = self.data_task(bit_receiver, data_led.as_deref_mut(), start_time);

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
                info!("MTU: Operation timeout reached");
            }
        }

        // Set clock to idle state (HIGH)
        clock_pin.set_high();
        if let Some(led) = clock_led.as_mut() {
            led.set_high(); // LED off
        }
        if let Some(led) = data_led.as_mut() {
            led.set_high(); // LED off
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
        mut clock_led: Option<&mut Output<'_>>,
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
            if let Some(led) = clock_led.as_mut() {
                led.set_high(); // LED off during clock low
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
                info!("MTU: Bit queue full, dropping bit");
            }

            // Clock HIGH phase and sample data (like RPI line 283-295)
            clock_pin.set_high();
            if let Some(led) = clock_led.as_mut() {
                led.set_low(); // LED on during clock high
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
        mut data_led: Option<&mut Output<'_>>,
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
            if let Some(led) = data_led.as_mut() {
                led.set_low(); // LED on for frame start
            }

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
                        info!("MTU: Timeout waiting for UART bit {}", bits_received);
                        break;
                    }
                }
            }

            if bits_received != frame_size {
                info!(
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
                                "MTU: UART frame -> char: {:?} (ASCII {})",
                                ch as char, ch as u8
                            );

                            // Check for end of message (carriage return)
                            if ch == '\r' {
                                let message: String<256> = received_chars.iter().collect();
                                info!("MTU: Received complete message: {:?}", message.as_str());

                                // Store the received message
                                {
                                    let mut last_msg = self.last_message.lock().await;
                                    *last_msg = Some(message);
                                }

                                received_chars.clear();
                                if let Some(led) = data_led.as_mut() {
                                    led.set_high(); // LED off for frame end
                                }
                                break; // Exit to stop receiving (like RPI line 455)
                            }
                        }
                        Err(e) => {
                            info!("MTU: Failed to extract character from frame: {:?}", e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    info!("MTU: Invalid UART frame: {:?}", e);
                    info!("MTU: Invalid frame bits: {:?}", frame_bits.as_slice());
                    continue;
                }
            }

            if let Some(led) = data_led.as_mut() {
                led.set_high(); // LED off for frame end
            }
        }

        info!("MTU: Data task stopped");
        Ok(())
    }
}
