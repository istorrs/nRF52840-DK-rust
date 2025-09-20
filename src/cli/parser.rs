use super::CliCommand;
use heapless::String;

pub struct CommandParser;

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandParser {
    pub fn new() -> Self {
        Self
    }

    pub fn get_available_commands() -> &'static [&'static str] {
        &[
            "help",
            "version",
            "status",
            "uptime",
            "clear",
            "reset",
            "echo",
            "led_on",
            "led_off",
            "button",
            "temp",
            "bt_scan",
            "mtu_start",
            "mtu_stop",
            "mtu_status",
        ]
    }

    pub fn autocomplete(partial: &str) -> heapless::Vec<&'static str, 10> {
        let mut matches = heapless::Vec::new();
        let commands = Self::get_available_commands();

        for &cmd in commands {
            if cmd.starts_with(partial) {
                let _ = matches.push(cmd);
            }
        }

        matches
    }

    pub fn parse_command(input: &str) -> CliCommand {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return CliCommand::Empty;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");

        match cmd {
            "help" => CliCommand::Help,
            "version" => CliCommand::Version,
            "status" => CliCommand::Status,
            "uptime" => CliCommand::Uptime,
            "clear" => CliCommand::Clear,
            "reset" => CliCommand::Reset,
            "button" => CliCommand::Button,
            "temp" => CliCommand::Temp,
            "bt_scan" => {
                if let Some(arg) = parts.next() {
                    if let Ok(scan_time) = arg.parse::<u16>() {
                        if scan_time > 0 && scan_time <= 60 {
                            CliCommand::BtScan(Some(scan_time))
                        } else {
                            let mut msg = String::new();
                            let _ = msg.push_str("bt_scan: scan time must be 1-60 seconds");
                            CliCommand::Unknown(msg)
                        }
                    } else {
                        let mut msg = String::new();
                        let _ = msg.push_str("bt_scan: invalid scan time");
                        CliCommand::Unknown(msg)
                    }
                } else {
                    CliCommand::BtScan(None) // Default time
                }
            }
            "mtu_start" => {
                if let Some(arg) = parts.next() {
                    if let Ok(duration) = arg.parse::<u16>() {
                        if duration > 0 && duration <= 300 {
                            CliCommand::MtuStart(Some(duration))
                        } else {
                            let mut msg = String::new();
                            let _ = msg.push_str("mtu_start: duration must be 1-300 seconds");
                            CliCommand::Unknown(msg)
                        }
                    } else {
                        let mut msg = String::new();
                        let _ = msg.push_str("mtu_start: invalid duration");
                        CliCommand::Unknown(msg)
                    }
                } else {
                    CliCommand::MtuStart(None) // Default duration
                }
            }
            "mtu_stop" => CliCommand::MtuStop,
            "mtu_status" => CliCommand::MtuStatus,
            "echo" => {
                let args: heapless::Vec<&str, 8> = parts.collect();
                let mut echo_string = heapless::String::new();
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        let _ = echo_string.push(' ');
                    }
                    let _ = echo_string.push_str(arg);
                }
                CliCommand::Echo(echo_string)
            }
            "led_on" => {
                if let Some(arg) = parts.next() {
                    if let Ok(led_num) = arg.parse::<u8>() {
                        if led_num == 3 || led_num == 4 {
                            CliCommand::LedOn(led_num)
                        } else {
                            let mut msg = String::new();
                            let _ = msg.push_str("led_on: LED must be 3 or 4");
                            CliCommand::Unknown(msg)
                        }
                    } else {
                        let mut msg = String::new();
                        let _ = msg.push_str("led_on: Invalid LED number");
                        CliCommand::Unknown(msg)
                    }
                } else {
                    let mut msg = String::new();
                    let _ = msg.push_str("led_on: Missing LED number");
                    CliCommand::Unknown(msg)
                }
            }
            "led_off" => {
                if let Some(arg) = parts.next() {
                    if let Ok(led_num) = arg.parse::<u8>() {
                        if led_num == 3 || led_num == 4 {
                            CliCommand::LedOff(led_num)
                        } else {
                            let mut msg = String::new();
                            let _ = msg.push_str("led_off: LED must be 3 or 4");
                            CliCommand::Unknown(msg)
                        }
                    } else {
                        let mut msg = String::new();
                        let _ = msg.push_str("led_off: Invalid LED number");
                        CliCommand::Unknown(msg)
                    }
                } else {
                    let mut msg = String::new();
                    let _ = msg.push_str("led_off: Missing LED number");
                    CliCommand::Unknown(msg)
                }
            }
            _ => {
                let mut unknown_cmd = String::new();
                let _ = unknown_cmd.push_str(cmd);
                CliCommand::Unknown(unknown_cmd)
            }
        }
    }
}
