use super::config::MtuConfig;
use super::error::{MtuError, MtuResult};
use defmt::info;
use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;
use core::sync::atomic::{AtomicBool, Ordering};

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

    // Simulate MTU operation - this would be replaced with actual GPIO tasks
    pub async fn simulate_mtu_operation(&self, duration: Duration) -> MtuResult<()> {
        info!("MTU: Simulating operation for {:?}", duration);

        let start_time = Instant::now();

        // Simulate receiving a water meter message after some time
        Timer::after(Duration::from_secs(2)).await;

        // Simulate a typical Sensus meter response
        let mut simulated_message = String::<256>::new();
        if simulated_message.push_str("ABCD1234\r").is_err() {
            return Err(MtuError::FramingError);
        }

        {
            let mut msg = self.last_message.lock().await;
            *msg = Some(simulated_message);
        }

        info!("MTU: Simulated message received");

        // Wait for remaining duration or until stopped
        while start_time.elapsed() < duration && self.running.load(Ordering::Relaxed) {
            Timer::after(Duration::from_millis(100)).await;
        }

        Ok(())
    }

}