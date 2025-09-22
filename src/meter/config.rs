use defmt::Format;
use heapless::String;

#[derive(Debug, Clone, Format)]
pub enum MeterType {
    Sensus,
    Neptune,
}

impl MeterType {
    pub fn framing(&self) -> crate::mtu::UartFraming {
        match self {
            MeterType::Sensus => crate::mtu::UartFraming::SevenE1,
            MeterType::Neptune => crate::mtu::UartFraming::SevenE2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeterConfig {
    pub meter_type: MeterType,
    pub response_message: String<256>,
    pub response_delay_ms: u64,
    pub enabled: bool,
}

impl Default for MeterConfig {
    fn default() -> Self {
        let mut default_message = String::new();
        // Test pattern: 3 characters with recognizable bit patterns + carriage return
        // 0x55 = 01010101 (alternating pattern)
        // 0x33 = 00110011 (two bits pattern)
        // 0x0F = 00001111 (four bits pattern)
        // 0x0D = 00001101 (carriage return)
        let _ = default_message.push(0x55 as char);
        let _ = default_message.push(0x33 as char);
        let _ = default_message.push(0x0F as char);
        let _ = default_message.push('\r'); // End message marker

        Self {
            meter_type: MeterType::Sensus,
            response_message: default_message,
            response_delay_ms: 50,
            enabled: true,
        }
    }
}
