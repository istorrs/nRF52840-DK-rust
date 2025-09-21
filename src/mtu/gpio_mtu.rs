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
        let mut last_data_bit = 1; // Assume line starts high (idle state)
        let mut clock_pulse_index = 0u32; // Track clock pulse index for debugging
        let expected_frame_bits = framing.bits_per_frame() - 1; // Exclude start bit from collection

        // Synchronous clock generation and data sampling with UART framing
        while start_time.elapsed() < duration && self.running.load(Ordering::Relaxed) {
            clock_pulse_index += 1;
            let timestamp = start_time.elapsed();
            
            // Send clock pulse (rising edge)
            clock_pin.set_high();
            if let Some(led) = clock_led.as_mut() {
                led.set_low(); // LED on during clock high
            }
            
            // Wait half the bit duration, then sample in the middle of the bit period
            Timer::after(bit_duration / 2).await;
            
            // Sample data line in the middle of the bit period when signal is stable
            let data_bit = if data_pin.is_high() { 1 } else { 0 };
            let pin_state = if data_bit == 1 { "HIGH" } else { "LOW" };
            
            // Set clock low and wait the remaining half bit duration
            clock_pin.set_low();
            if let Some(led) = clock_led.as_mut() {
                led.set_high(); // LED off during clock low
            }
            Timer::after(bit_duration / 2).await;

            // Log every received bit with timestamp and clock pulse index
            info!("MTU: Clock #{} @ {:?}: RX bit {} (pin: {})", 
                  clock_pulse_index, timestamp, data_bit, pin_state);

            // UART frame detection and assembly
            // Only detect start bit on high-to-low transition (not just any low bit)
            if !in_frame && last_data_bit == 1 && data_bit == 0 {
                in_frame = true;
                frame_bits.clear();
                frame_bit_count = 0;
                if let Some(led) = data_led.as_mut() {
                    led.set_low(); // LED on for frame start
                }
                info!("MTU: Clock #{} @ {:?}: START BIT detected (transition 1->0)", clock_pulse_index, timestamp);
                // Don't add the start bit to frame_bits - it's just for synchronization
            } else if in_frame {
                // Collect bits for current frame (excluding start bit)
                if frame_bits.push(data_bit).is_err() {
                    info!("MTU: Clock #{} @ {:?}: Frame buffer overflow, resetting", clock_pulse_index, timestamp);
                    in_frame = false;
                    continue;
                }
                frame_bit_count += 1;
                info!("MTU: Clock #{} @ {:?}: Frame bit #{}: {}", clock_pulse_index, timestamp, frame_bit_count, data_bit);

                // Check if we have a complete frame
                if frame_bit_count >= expected_frame_bits {
                    in_frame = false;
                    if let Some(led) = data_led.as_mut() {
                        led.set_high(); // LED off for frame end
                    }
                    info!("MTU: Clock #{} @ {:?}: FRAME COMPLETE ({} bits): [{}]", 
                          clock_pulse_index, timestamp, frame_bit_count, 
                          frame_bits.iter().map(|&b| if b == 1 { '1' } else { '0' }).collect::<heapless::String<32>>().as_str());

                    // Process complete frame
                    match UartFrame::new(frame_bits.clone(), framing) {
                        Ok(frame) => {
                            match extract_char_from_frame(&frame) {
                                Ok(ch) => {
                                    info!("MTU: Clock #{} @ {:?}: DECODED character: '{}' (ASCII {})", 
                                          clock_pulse_index, timestamp, ch as char, ch as u8);
                                    if message_chars.push(ch).is_err() {
                                        info!("MTU: Message buffer full");
                                    }

                                    // Check for end of message (carriage return)
                                    if ch == '\r' {
                                        let message: String<256> = message_chars.iter().collect();
                                        info!(
                                            "MTU: Clock #{} @ {:?}: COMPLETE MESSAGE received: '{}'",
                                            clock_pulse_index, timestamp, message.as_str()
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
                                    info!("MTU: Clock #{} @ {:?}: INVALID character in frame: [{}]", 
                                          clock_pulse_index, timestamp, 
                                          frame_bits.iter().map(|&b| if b == 1 { '1' } else { '0' }).collect::<heapless::String<32>>().as_str());
                                }
                            }
                        }
                        Err(_) => {
                            info!("MTU: Clock #{} @ {:?}: INVALID UART frame: [{}]", 
                                  clock_pulse_index, timestamp, 
                                  frame_bits.iter().map(|&b| if b == 1 { '1' } else { '0' }).collect::<heapless::String<32>>().as_str());
                        }
                    }
                }
            }
            
            // Update last data bit for transition detection
            last_data_bit = data_bit;
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
