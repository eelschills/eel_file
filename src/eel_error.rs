#[derive(Debug)]
pub enum EelError {
    Io(String),
    Interrupted(String),
    FreeSpace(String),
    PermissionError(String),
    ConnectionError(String),
    InvalidIP(String),
}

impl std::fmt::Display for EelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EelError::Io(err) => write!(f, "IO error: {}", err),
            EelError::Interrupted(err) => write!(f, "Interrupted: {}", err),
            EelError::FreeSpace(err) => write!(f, "Not enough free space to write the file: {}", err),
            EelError::PermissionError(err) => write!(f, "Permission error: {}", err),
            EelError::ConnectionError(err) => write!(f, "Connection error, server is either unreachable or connection was refused: {}", err),
            EelError::InvalidIP(err) => write!(f, "Invalid IP: {}", err),
        }
    }
}

impl std::error::Error for EelError {}