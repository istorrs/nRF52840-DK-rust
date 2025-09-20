use super::config::MtuConfig;
use super::error::MtuResult;
use super::uart_framing::{extract_char_from_frame, UartFrame};
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::info;
use embassy_nrf::gpio::{Input, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;

pub struct GpioMtu {
    config: Mutex<ThreadModeRawMutex, MtuConfig>,
    running: AtomicBool,
    last_message: Mutex<ThreadModeRawMutex, Option<String<256>>>,
}

impl GpioMtu {
    pub fn new(config: MtuConfig) -> Self {
        Self {
            config: Mutex::new(config),
            running: AtomicBool::new(false),
            last_message: Mutex::new(None),
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

    // Actual MTU operation using GPIO pins - continuous clock at baud rate
    pub async fn run_mtu_operation(
        &self,
        duration: Duration,
        clock_pin: &mut Output<'_>,
        data_pin: &Input<'_>,
        mut clock_led: Option<&mut Output<'_>>,
        mut data_led: Option<&mut Output<'_>>,
    ) -> MtuResult<()> {
        let config = self.config.lock().await;
        let baud_rate = config.baud_rate;
        let framing = config.framing;
        let power_up_delay_ms = config.power_up_delay_ms;
        drop(config); // Release lock early

        info!(
            "MTU: Starting continuous clock generation at {} baud for {:?}",
            baud_rate, duration
        );

        let start_time = Instant::now();
        let bit_duration = Duration::from_micros(1_000_000 / baud_rate as u64);

        // Power up delay as specified in config
        Timer::after(Duration::from_millis(power_up_delay_ms)).await;

        // UART frame assembly state
        let mut frame_bits = heapless::Vec::<u8, 16>::new();
        let mut message_chars = heapless::Vec::<char, 256>::new();
        let mut in_frame = false;
        let mut frame_bit_count = 0;
        let expected_frame_bits = framing.bits_per_frame();

        // Synchronous clock generation and data sampling with UART framing
        while start_time.elapsed() < duration && self.running.load(Ordering::Relaxed) {
            // Send clock pulse (high then low)
            clock_pin.set_high();
            if let Some(led) = clock_led.as_mut() {
                led.set_low(); // LED on during clock high
            }
            Timer::after(bit_duration).await;

            clock_pin.set_low();
            if let Some(led) = clock_led.as_mut() {
                led.set_high(); // LED off during clock low
            }
            Timer::after(bit_duration).await;

            // Sample data line after clock pulse
            let data_bit = if data_pin.is_high() { 1 } else { 0 };

            // Log data line state periodically for debugging
            static mut DEBUG_COUNTER: u32 = 0;
            unsafe {
                DEBUG_COUNTER += 1;
                if DEBUG_COUNTER % 1000 == 0 {
                    info!("MTU: Data line sample #{}: {} (pin level: {})", DEBUG_COUNTER, data_bit, if data_pin.is_high() { "HIGH" } else { "LOW" });
                }
            }

            // UART frame detection and assembly
            if !in_frame && data_bit == 0 {
                // Start bit detected (high to low transition)
                in_frame = true;
                frame_bits.clear();
                frame_bit_count = 0;
                if let Some(led) = data_led.as_mut() {
                    led.set_low(); // LED on for frame start
                }
                info!("MTU: Start bit detected, beginning frame");
            }

            if in_frame {
                // Collect bits for current frame
                if frame_bits.push(data_bit).is_err() {
                    info!("MTU: Frame buffer overflow, resetting");
                    in_frame = false;
                    continue;
                }
                frame_bit_count += 1;

                // Check if we have a complete frame
                if frame_bit_count >= expected_frame_bits {
                    in_frame = false;
                    if let Some(led) = data_led.as_mut() {
                        led.set_high(); // LED off for frame end
                    }

                    // Process complete frame
                    match UartFrame::new(frame_bits.clone(), framing) {
                        Ok(frame) => {
                            match extract_char_from_frame(&frame) {
                                Ok(ch) => {
                                    info!("MTU: Received character: '{}'", ch as char);
                                    if message_chars.push(ch).is_err() {
                                        info!("MTU: Message buffer full");
                                    }

                                    // Check for end of message (carriage return)
                                    if ch == '\r' {
                                        let message: String<256> = message_chars.iter().collect();
                                        info!(
                                            "MTU: Complete message received: {}",
                                            message.as_str()
                                        );

                                        // Store the message
                                        {
                                            let mut msg = self.last_message.lock().await;
                                            *msg = Some(message);
                                        }

                                        message_chars.clear();
                                    }
                                }
                                Err(_) => {
                                    info!("MTU: Invalid character in frame");
                                }
                            }
                        }
                        Err(_) => {
                            info!("MTU: Invalid UART frame");
                        }
                    }
                }
            }
        }

        // Set clock to idle state
        clock_pin.set_high();
        if let Some(led) = clock_led.as_mut() {
            led.set_high(); // LED off
        }

        info!("MTU: Operation completed after {:?}", start_time.elapsed());
        Ok(())
    }
}
