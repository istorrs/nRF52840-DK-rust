use defmt::Format;

#[derive(Debug, Format)]
pub enum MtuError {
    GpioError,
    TimeoutError,
    FramingError,
    ConfigError,
    ChannelError,
}

pub type MtuResult<T> = Result<T, MtuError>;