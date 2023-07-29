#[repr(u8)]
#[derive(Debug)]
pub enum ExitCode {
    MissingProgramArgument = 1,
    IPCSocketNotFound,
    FileLinkNotFound,
    UnableToCreateSharedMemory,
    SocketConnectionFailed,
    ReadFromSocketFailed,
    UnknownMessageReceived,
    Unknown = 255,
}

impl From<u8> for ExitCode {
    fn from(value: u8) -> Self {
        match value {
            1 => ExitCode::MissingProgramArgument,
            2 => ExitCode::IPCSocketNotFound,
            3 => ExitCode::FileLinkNotFound,
            4 => ExitCode::UnableToCreateSharedMemory,
            5 => ExitCode::SocketConnectionFailed,
            6 => ExitCode::ReadFromSocketFailed,
            7 => ExitCode::UnknownMessageReceived,
            _ => ExitCode::Unknown,
        }
    }
}

impl From<ExitCode> for u8 {
    fn from(value: ExitCode) -> Self {
        value as u8
    }
}
