//! Telnet 協定常數和解析
//!
//! 實作 RFC 854 Telnet 協定的基本命令

/// Telnet IAC (Interpret As Command) - 0xFF
pub const IAC: u8 = 255;

/// Telnet 命令
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TelnetCommand {
    /// Sub-negotiation End
    Se = 240,
    /// No Operation
    Nop = 241,
    /// Data Mark
    DataMark = 242,
    /// Break
    Break = 243,
    /// Interrupt Process
    InterruptProcess = 244,
    /// Abort Output
    AbortOutput = 245,
    /// Are You There
    AreYouThere = 246,
    /// Erase Character
    EraseCharacter = 247,
    /// Erase Line
    EraseLine = 248,
    /// Go Ahead
    GoAhead = 249,
    /// Sub-negotiation Begin
    Sb = 250,
    /// Will
    Will = 251,
    /// Won't
    Wont = 252,
    /// Do
    Do = 253,
    /// Don't
    Dont = 254,
}

impl TelnetCommand {
    /// 從位元組解析 Telnet 命令
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            240 => Some(Self::Se),
            241 => Some(Self::Nop),
            242 => Some(Self::DataMark),
            243 => Some(Self::Break),
            244 => Some(Self::InterruptProcess),
            245 => Some(Self::AbortOutput),
            246 => Some(Self::AreYouThere),
            247 => Some(Self::EraseCharacter),
            248 => Some(Self::EraseLine),
            249 => Some(Self::GoAhead),
            250 => Some(Self::Sb),
            251 => Some(Self::Will),
            252 => Some(Self::Wont),
            253 => Some(Self::Do),
            254 => Some(Self::Dont),
            _ => None,
        }
    }
}

/// Telnet 選項
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TelnetOption {
    /// Binary Transmission
    BinaryTransmission = 0,
    /// Echo
    Echo = 1,
    /// Suppress Go Ahead
    SuppressGoAhead = 3,
    /// Terminal Type
    TerminalType = 24,
    /// Window Size (NAWS)
    Naws = 31,
    /// Terminal Speed
    TerminalSpeed = 32,
    /// Remote Flow Control
    RemoteFlowControl = 33,
    /// Linemode
    Linemode = 34,
    /// Environment Variables (New)
    NewEnviron = 39,
    /// Charset
    Charset = 42,
    /// MCCP2 (MUD Client Compression Protocol)
    Mccp2 = 86,
    /// MCCP3
    Mccp3 = 87,
    /// GMCP (Generic MUD Communication Protocol)
    Gmcp = 201,
    /// Unknown option
    Unknown(u8),
}

impl TelnetOption {
    /// 從位元組解析 Telnet 選項
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::BinaryTransmission,
            1 => Self::Echo,
            3 => Self::SuppressGoAhead,
            24 => Self::TerminalType,
            31 => Self::Naws,
            32 => Self::TerminalSpeed,
            33 => Self::RemoteFlowControl,
            34 => Self::Linemode,
            39 => Self::NewEnviron,
            42 => Self::Charset,
            86 => Self::Mccp2,
            87 => Self::Mccp3,
            201 => Self::Gmcp,
            other => Self::Unknown(other),
        }
    }

    /// 獲取選項的位元組值
    pub fn as_byte(&self) -> u8 {
        match self {
            Self::BinaryTransmission => 0,
            Self::Echo => 1,
            Self::SuppressGoAhead => 3,
            Self::TerminalType => 24,
            Self::Naws => 31,
            Self::TerminalSpeed => 32,
            Self::RemoteFlowControl => 33,
            Self::Linemode => 34,
            Self::NewEnviron => 39,
            Self::Charset => 42,
            Self::Mccp2 => 86,
            Self::Mccp3 => 87,
            Self::Gmcp => 201,
            Self::Unknown(b) => *b,
        }
    }
}

/// Telnet 資料解析結果
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Data variant 保留給未來使用
pub enum TelnetEvent {
    /// 純文字資料
    Data(Vec<u8>),
    /// Telnet 命令（如 WILL, WONT, DO, DONT）
    Command(TelnetCommand, TelnetOption),
    /// Sub-negotiation 資料
    Subnegotiation(TelnetOption, Vec<u8>),
}

/// 解析 Telnet 資料流，分離出文字和命令
pub fn parse_telnet_data(input: &[u8]) -> (Vec<u8>, Vec<TelnetEvent>) {
    let mut data = Vec::new();
    let mut events = Vec::new();
    let mut i = 0;

    while i < input.len() {
        if input[i] == IAC {
            if i + 1 >= input.len() {
                break; // 不完整的 IAC 序列
            }

            if input[i + 1] == IAC {
                // IAC IAC = 轉義的 0xFF
                data.push(IAC);
                i += 2;
                continue;
            }

            if let Some(cmd) = TelnetCommand::from_byte(input[i + 1]) {
                match cmd {
                    TelnetCommand::Will
                    | TelnetCommand::Wont
                    | TelnetCommand::Do
                    | TelnetCommand::Dont => {
                        if i + 2 < input.len() {
                            let option = TelnetOption::from_byte(input[i + 2]);
                            events.push(TelnetEvent::Command(cmd, option));
                            i += 3;
                            continue;
                        }
                    }
                    TelnetCommand::Sb => {
                        // 尋找 Sub-negotiation 結束 (IAC SE)
                        if i + 2 < input.len() {
                            let option = TelnetOption::from_byte(input[i + 2]);
                            let mut sub_data = Vec::new();
                            let mut j = i + 3;

                            while j + 1 < input.len() {
                                if input[j] == IAC && input[j + 1] == TelnetCommand::Se as u8 {
                                    events.push(TelnetEvent::Subnegotiation(option, sub_data));
                                    i = j + 2;
                                    break;
                                }
                                sub_data.push(input[j]);
                                j += 1;
                            }
                            continue;
                        }
                    }
                    _ => {
                        // 其他命令，跳過
                        i += 2;
                        continue;
                    }
                }
            }

            i += 2;
        } else {
            data.push(input[i]);
            i += 1;
        }
    }

    (data, events)
}

/// 生成 Telnet 拒絕回應（對所有選項回應 WONT/DONT）
pub fn generate_refusal(cmd: TelnetCommand, option: TelnetOption) -> Vec<u8> {
    let response_cmd = match cmd {
        TelnetCommand::Will | TelnetCommand::Do => TelnetCommand::Wont,
        TelnetCommand::Wont | TelnetCommand::Dont => return vec![], // 不需要回應
        _ => return vec![],
    };

    // 對於 ECHO 和 SGA，我們接受
    let response_cmd = match option {
        TelnetOption::Echo | TelnetOption::SuppressGoAhead => {
            if cmd == TelnetCommand::Will {
                TelnetCommand::Do
            } else if cmd == TelnetCommand::Do {
                TelnetCommand::Will
            } else {
                response_cmd
            }
        }
        _ => {
            if cmd == TelnetCommand::Do {
                TelnetCommand::Wont
            } else {
                TelnetCommand::Dont
            }
        }
    };

    vec![IAC, response_cmd as u8, option.as_byte()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let input = b"Hello World";
        let (data, events) = parse_telnet_data(input);
        assert_eq!(data, b"Hello World");
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_escaped_iac() {
        let input = [b'A', IAC, IAC, b'B'];
        let (data, events) = parse_telnet_data(&input);
        assert_eq!(data, vec![b'A', IAC, b'B']);
        assert!(events.is_empty());
    }

    #[test]
    fn test_parse_will_command() {
        let input = [IAC, TelnetCommand::Will as u8, TelnetOption::Echo.as_byte()];
        let (data, events) = parse_telnet_data(&input);
        assert!(data.is_empty());
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            TelnetEvent::Command(TelnetCommand::Will, TelnetOption::Echo)
        );
    }

    #[test]
    fn test_parse_mixed_content() {
        let mut input = b"Hello ".to_vec();
        input.extend_from_slice(&[IAC, TelnetCommand::Do as u8, TelnetOption::SuppressGoAhead.as_byte()]);
        input.extend_from_slice(b" World");

        let (data, events) = parse_telnet_data(&input);
        assert_eq!(data, b"Hello  World");
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_generate_refusal_for_unknown_option() {
        let response = generate_refusal(TelnetCommand::Do, TelnetOption::Mccp2);
        assert_eq!(response, vec![IAC, TelnetCommand::Wont as u8, TelnetOption::Mccp2.as_byte()]);
    }

    #[test]
    fn test_accept_echo() {
        let response = generate_refusal(TelnetCommand::Will, TelnetOption::Echo);
        assert_eq!(response, vec![IAC, TelnetCommand::Do as u8, TelnetOption::Echo.as_byte()]);
    }

    #[test]
    fn test_telnet_option_roundtrip() {
        for byte in 0..=255u8 {
            let option = TelnetOption::from_byte(byte);
            assert_eq!(option.as_byte(), byte);
        }
    }
}
