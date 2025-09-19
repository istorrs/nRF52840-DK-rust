pub mod commands;
pub mod parser;
pub mod terminal;

pub use commands::CommandHandler;
pub use parser::CommandParser;
pub use terminal::Terminal;

// CLI-related types and constants
pub const CLI_BUFFER_SIZE: usize = 128;
pub const MAX_HISTORY_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub enum CliCommand {
    Help,
    Version,
    Status,
    Uptime,
    Clear,
    Reset,
    Echo(heapless::String<64>),
    LedOn(u8),
    LedOff(u8),
    Button,
    Temp,
    BtScan(Option<u16>), // Optional scan time in seconds
    Empty,
    Unknown(heapless::String<32>),
}

#[derive(Debug)]
pub enum CliError {
    InvalidCommand,
    InvalidArgument,
    UartError,
    BufferFull,
}
