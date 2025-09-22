use embassy_time::Duration;
use heapless::String;

#[derive(Debug, Clone)]
pub struct MtuConfig {
    /// Clock cycle duration (period for HIGH/LOW states)
    pub cycle_duration: Duration,

    /// Power-up delay before starting clock cycles (ms)
    pub power_up_delay_ms: u64,

    /// Bit timeout for incomplete frames (ms)
    pub bit_timeout_ms: u64,

    /// Maximum runtime for MTU operation
    pub runtime: Duration,

    /// UART framing configuration
    pub framing: UartFraming,

    /// Baud rate for communication
    pub baud_rate: u32,

    /// Expected message for testing (default is meter's default response)
    pub expected_message: String<256>,

    /// Running count of successful message reads
    pub successful_reads: u32,

    /// Running count of corrupted/failed message reads
    pub corrupted_reads: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum UartFraming {
    /// 7 data bits, even parity, 1 stop bit (Sensus Standard)
    SevenE1,
    /// 7 data bits, even parity, 2 stop bits (Neptune)
    SevenE2,
}

impl UartFraming {
    pub fn bits_per_frame(self) -> usize {
        match self {
            UartFraming::SevenE1 => 10, // 1 start + 7 data + 1 parity + 1 stop
            UartFraming::SevenE2 => 11, // 1 start + 7 data + 1 parity + 2 stop
        }
    }
}

impl MtuConfig {
    /// Calculate bit duration in microseconds from baud rate
    pub fn bit_duration_micros(&self) -> u64 {
        1_000_000 / self.baud_rate as u64
    }

    /// Get bit duration as Embassy Duration
    pub fn bit_duration(&self) -> Duration {
        Duration::from_micros(self.bit_duration_micros())
    }
}

impl Default for MtuConfig {
    fn default() -> Self {
        let mut expected_message = String::new();
        // Default expected message matches meter's default response
        let _ = expected_message.push_str(
            "V;RB00000200;IB61564400;A1000;Z3214;XT0746;MT0683;RR00000000;GX000000;GN000000\r",
        );

        Self {
            cycle_duration: Duration::from_micros(1000), // 1ms period = 500Hz
            power_up_delay_ms: 10, // Very short delay to be ready before meter starts
            bit_timeout_ms: 2000,
            runtime: Duration::from_secs(30),
            framing: UartFraming::SevenE1, // Sensus Standard default
            baud_rate: 1200,               // Default to 1200 baud
            expected_message,
            successful_reads: 0,
            corrupted_reads: 0,
        }
    }
}
