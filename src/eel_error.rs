#[derive(Debug)]
pub enum EelError {
    Io(String),
    Poll(String),
    EmptyMessage(String),
}

impl std::fmt::Display for EelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EelError::Io(err) => write!(f, "IO error: {}", err),
            EelError::Poll(err) => {
                write!(f, "Trying to poll a future that does not exist, {}", err)
            }
            EelError::EmptyMessage(err) => write!(f, "Receiving on an empty stream"),
        }
    }
}

impl std::error::Error for EelError {}
