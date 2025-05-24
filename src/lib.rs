use std::fs::File;
use std::path::PathBuf;

pub enum AppState {
    Idle,
    Listening,
    Handshake(FileInfo),
    Accepting(f32),
    Sending(FileInfo),
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppState::Idle => write!(f, "Idle"),
            AppState::Listening => write!(f, "Listening"),
            AppState::Accepting(_) => write!(f, "Accepting"),
            AppState::Sending(FileInfo) => write!(f, "Sending"),
            AppState::Handshake(_) => write!(f, "Handshake"),
        }
    }
}

pub struct FileInfo {
    path: PathBuf,
    // idk about that chief, will figure it out as I go
    size: u64,
    hash: String,
    handle: Option<File>,
    name: String,
    format: String,
}

impl Default for FileInfo {
    fn default() -> FileInfo {
        FileInfo {
            path: Default::default(),
            size: 0,
            hash: "".to_string(),
            handle: None,
            name: "testfile".to_string(),
            format: "mp3".to_string(),
        }
    }
}
