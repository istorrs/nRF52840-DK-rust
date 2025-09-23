use super::config::UartFraming;
use super::error::{MtuError, MtuResult};
use heapless::Vec;

#[derive(Debug, Clone)]
pub struct UartFrame {
    pub bits: Vec<u8, 16>, // Max 16 bits per frame
    pub framing: UartFraming,
}

impl UartFrame {
    pub fn new(bits: Vec<u8, 16>, framing: UartFraming) -> MtuResult<Self> {
        if bits.len() != framing.bits_per_frame() {
            return Err(MtuError::FramingError);
        }
        Ok(Self { bits, framing })
    }

    pub fn validate(&self) -> MtuResult<()> {
        let expected_bits = self.framing.bits_per_frame();
        if self.bits.len() != expected_bits {
            return Err(MtuError::FramingErrorInvalidBitCount);
        }

        // Check start bit (must be 0)
        if self.bits[0] != 0 {
            return Err(MtuError::FramingErrorInvalidStartBit);
        }

        // Check stop bits (must be 1)
        match self.framing {
            UartFraming::SevenE1 => {
                if self.bits[9] != 1 {
                    return Err(MtuError::FramingErrorInvalidStopBit);
                }
            }
            UartFraming::SevenE2 => {
                if self.bits[9] != 1 || self.bits[10] != 1 {
                    return Err(MtuError::FramingErrorInvalidStopBit);
                }
            }
        }

        // Check even parity
        let data_bits = &self.bits[1..8]; // 7 data bits
        let parity_bit = self.bits[8];
        let data_ones = data_bits.iter().filter(|&&bit| bit == 1).count();
        let expected_parity = if data_ones % 2 == 0 { 0 } else { 1 }; // Even parity

        if parity_bit != expected_parity {
            return Err(MtuError::FramingErrorParityMismatch);
        }

        Ok(())
    }
}

pub fn extract_char_from_frame(frame: &UartFrame) -> MtuResult<char> {
    frame.validate()?;

    // Extract 7 data bits (bits 1-7)
    let mut char_value = 0u8;
    for (i, &bit) in frame.bits[1..8].iter().enumerate() {
        if bit == 1 {
            char_value |= 1 << i;
        }
    }

    // Convert to ASCII character
    if char_value <= 127 {
        Ok(char_value as char)
    } else {
        Err(MtuError::FramingError)
    }
}

pub fn bits_to_frame(bits: &[u8], framing: UartFraming) -> MtuResult<UartFrame> {
    let mut frame_bits: Vec<u8, 16> = Vec::new();

    for &bit in bits {
        if frame_bits.push(bit).is_err() {
            return Err(MtuError::FramingError);
        }
    }

    UartFrame::new(frame_bits, framing)
}
