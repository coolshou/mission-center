#[repr(u8)]
pub enum Message {
    GetInstalledApps = 0,
    Exit = 254,
    #[allow(dead_code)]
    Unknown = 255,
}

impl From<u8> for Message {
    fn from(value: u8) -> Self {
        match value {
            0 => Message::GetInstalledApps,
            254 => Message::Exit,
            _ => Message::Unknown,
        }
    }
}

impl From<Message> for u8 {
    fn from(value: Message) -> Self {
        value as u8
    }
}
