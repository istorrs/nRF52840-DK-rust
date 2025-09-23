use defmt::Format;

#[derive(Debug, Format)]
pub enum MtuError {
    GpioError,
    TimeoutError,
    FramingError,
    FramingErrorInvalidBitCount,
    FramingErrorInvalidStartBit,
    FramingErrorInvalidStopBit,
    FramingErrorParityMismatch,
    ConfigError,
    ChannelError,
}

pub type MtuResult<T> = Result<T, MtuError>;
