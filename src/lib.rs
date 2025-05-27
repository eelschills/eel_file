use std::fs::File;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json;

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

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    path: PathBuf,
    // idk about that chief, will figure it out as I go
    size: u64,
    hash: String,
    name: String,
    extension: String,
}

// for testing porpoises
impl Default for FileInfo {
    fn default() -> FileInfo {
        let a = FileInfo {
            path: Default::default(),
            size: 0,
            hash: "".to_string(),
            name: "testfile".to_string(),
            extension: "mp3".to_string(),
        };

        let json = serde_json::to_string(&a).unwrap();
        println!("{}", json);
        
        a
    }
}
