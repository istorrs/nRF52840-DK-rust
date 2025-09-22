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
        // Realistic water meter response message
        let _ = default_message.push_str(
            "V;RB00000200;IB61564400;A1000;Z3214;XT0746;MT0683;RR00000000;GX000000;GN000000\r",
        );

        Self {
            meter_type: MeterType::Sensus,
            response_message: default_message,
            response_delay_ms: 50,
            enabled: true,
        }
    }
}
