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
        let _ = default_message.push_str("WATER001\r");

        Self {
            meter_type: MeterType::Sensus,
            response_message: default_message,
            response_delay_ms: 50,
            enabled: true,
        }
    }
}