use embassy_time::Duration;

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
        Self {
            cycle_duration: Duration::from_micros(1000), // 1ms period = 500Hz
            power_up_delay_ms: 10, // Very short delay to be ready before meter starts
            bit_timeout_ms: 2000,
            runtime: Duration::from_secs(30),
            framing: UartFraming::SevenE1, // Sensus Standard default
            baud_rate: 1200, // Default to 1200 baud
        }
    }
}
