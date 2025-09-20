use super::config::MtuConfig;
use super::error::{MtuError, MtuResult};
use super::uart_framing::{extract_char_from_frame, UartFrame};
use core::sync::atomic::{AtomicBool, Ordering};
use defmt::info;
use embassy_nrf::gpio::{Input, Output};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;

pub struct GpioMtu {
    config: MtuConfig,
    running: AtomicBool,
    last_message: Mutex<ThreadModeRawMutex, Option<String<256>>>,
}

impl GpioMtu {
    pub fn new(config: MtuConfig) -> Self {
        Self {
            config,
            running: AtomicBool::new(false),
            last_message: Mutex::new(None),
        }
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

    // Actual MTU operation using GPIO pins
    pub async fn run_mtu_operation(
        &self,
        duration: Duration,
        clock_pin: &mut Output<'_>,
        data_pin: &Input<'_>,
    ) -> MtuResult<()> {
        info!("MTU: Starting GPIO-based operation for {:?}", duration);

        let start_time = Instant::now();

        // Power up delay as specified in config
        Timer::after(Duration::from_millis(self.config.power_up_delay_ms)).await;

        // Main MTU loop - wake up meter and read response
        while start_time.elapsed() < duration && self.running.load(Ordering::Relaxed) {
            info!("MTU: Waking up meter");

            // Wake up the meter by sending clock pulses
            if let Err(e) = self.wake_up_meter(clock_pin).await {
                info!("MTU: Wake up failed: {:?}", e);
                Timer::after(self.config.cycle_duration).await;
                continue;
            }

            // Try to read a complete UART frame from the meter
            match self.read_uart_frame(data_pin).await {
                Ok(frame) => {
                    // Extract character from frame using framing protocol
                    match extract_char_from_frame(&frame) {
                        Ok(ch) => {
                            info!("MTU: Received character: {}", ch as char);
                            // Build up message string
                            let mut message = String::<256>::new();
                            if message.push(ch).is_err() {
                                return Err(MtuError::FramingError);
                            }

                            // Store the message
                            {
                                let mut msg = self.last_message.lock().await;
                                *msg = Some(message);
                            }
                        }
                        Err(_) => {
                            info!("MTU: Invalid frame received");
                        }
                    }
                }
                Err(e) => {
                    info!("MTU: Read error: {:?}", e);
                }
            }

            // Wait for next cycle
            Timer::after(self.config.cycle_duration).await;
        }

        info!("MTU: Operation completed");
        Ok(())
    }

    // Wake up the meter by sending clock pulses
    async fn wake_up_meter(&self, clock_pin: &mut Output<'_>) -> MtuResult<()> {
        // Send wake-up clock pulses (typically 10-20 pulses)
        for _ in 0..15 {
            clock_pin.set_high();
            Timer::after(Duration::from_micros(104)).await; // ~9600 baud timing
            clock_pin.set_low();
            Timer::after(Duration::from_micros(104)).await;
        }

        info!("MTU: Wake-up pulses sent");
        Ok(())
    }

    // Read a complete UART frame from the meter
    async fn read_uart_frame(&self, data_pin: &Input<'_>) -> MtuResult<UartFrame> {
        let mut frame_bits = heapless::Vec::<u8, 16>::new();

        // Wait for start bit (data line goes low)
        let timeout = Instant::now() + Duration::from_millis(self.config.bit_timeout_ms);
        while data_pin.is_high() && Instant::now() < timeout {
            Timer::after(Duration::from_micros(10)).await;
        }

        if Instant::now() >= timeout {
            return Err(MtuError::TimeoutError);
        }

        info!("MTU: Start bit detected");

        // Sample at 9600 baud (104 microseconds per bit)
        let bit_duration = Duration::from_micros(104);

        // Wait half bit time to get to middle of start bit
        Timer::after(Duration::from_micros(52)).await;

        // Sample bits based on framing configuration
        let expected_bits = self.config.framing.bits_per_frame();
        for _ in 0..expected_bits {
            Timer::after(bit_duration).await;
            let bit_value = if data_pin.is_high() { 1 } else { 0 }; // UART logic levels
            if frame_bits.push(bit_value).is_err() {
                return Err(MtuError::FramingError);
            }
        }

        // Create frame with collected bits
        let frame = UartFrame::new(frame_bits, self.config.framing)?;
        info!("MTU: Frame received with {} bits", expected_bits);
        Ok(frame)
    }
}
