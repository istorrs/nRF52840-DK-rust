use super::config::MeterType;
use heapless::String;

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum MeterCommand {
    Help,
    Clear,
    Version,
    Status,
    SetType(MeterType),
    SetMessage(String<256>),
    Enable,
    Disable,
    Test,
}

pub struct MeterCommandParser;

impl MeterCommandParser {
    pub fn parse_command(input: &str) -> MeterCommand {
        let input = input.trim();

        if input.is_empty() {
            return MeterCommand::Help;
        }

        let parts: heapless::Vec<&str, 8> = input.split_whitespace().collect();

        if parts.is_empty() {
            return MeterCommand::Help;
        }

        match parts[0] {
            "help" | "h" => MeterCommand::Help,
            "clear" | "cls" => MeterCommand::Clear,
            "version" | "ver" => MeterCommand::Version,
            "status" | "stat" => MeterCommand::Status,
            "enable" => MeterCommand::Enable,
            "disable" => MeterCommand::Disable,
            "test" => MeterCommand::Test,
            "type" => {
                if parts.len() >= 2 {
                    match parts[1] {
                        "sensus" | "s" => MeterCommand::SetType(MeterType::Sensus),
                        "neptune" | "n" => MeterCommand::SetType(MeterType::Neptune),
                        _ => MeterCommand::Help,
                    }
                } else {
                    MeterCommand::Help
                }
            }
            "message" | "msg" => {
                if parts.len() >= 2 {
                    // Join all parts after "message" as the message content
                    let message_parts = &parts[1..];
                    let mut full_message = String::<256>::new();
                    for (i, part) in message_parts.iter().enumerate() {
                        if i > 0 {
                            let _ = full_message.push(' ');
                        }
                        let _ = full_message.push_str(part);
                    }

                    let mut message = String::new();
                    if message.push_str(&full_message).is_ok() {
                        // Add carriage return if not present
                        if !message.ends_with('\r') {
                            let _ = message.push('\r');
                        }
                        MeterCommand::SetMessage(message)
                    } else {
                        MeterCommand::Help
                    }
                } else {
                    MeterCommand::Help
                }
            }
            _ => MeterCommand::Help,
        }
    }

    pub fn available_commands() -> &'static [&'static str] {
        &[
            "help", "clear", "version", "status", "type", "message", "enable", "disable", "test",
        ]
    }
}
