use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json;

pub mod eel_error;
pub use eel_error::*;

pub enum AppState {
    Idle,
    Listening,
    Handshake(FileInfo),
    Accepting(TransferState),
    Sending(TransferState),
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppState::Idle => write!(f, "Idle"),
            AppState::Listening => write!(f, "Listening"),
            AppState::Accepting(_) => write!(f, "Accepting"),
            AppState::Sending(_) => write!(f, "Sending"),
            AppState::Handshake(_) => write!(f, "Handshake"),
        }
    }
}

#[derive(Debug)]
pub enum TransferState {
    Transferring(f32),
    Result(Result<(), EelError>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub path: Option<PathBuf>,
    pub size: u64,
    pub hash: String,
    pub name: String
}

// for testing porpoises
impl Default for FileInfo {
    fn default() -> FileInfo {
        let a = FileInfo {
            path: None,
            size: 0,
            hash: "".to_string(),
            name: "testfile".to_string(),
        };

        let json = serde_json::to_string(&a).unwrap();
        println!("{}", json);
        a
    }
}
