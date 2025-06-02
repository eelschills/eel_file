use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use serde_json;

pub mod eel_error;
pub use eel_error::*;

pub enum AppState {
    Idle,
    Listening,
    Handshake,
    Accepting,
    Sending,
}

impl std::fmt::Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AppState::Idle => write!(f, "Idle"),
            AppState::Listening => write!(f, "Listening"),
            AppState::Accepting => write!(f, "Accepting"),
            AppState::Sending => write!(f, "Sending"),
            AppState::Handshake => write!(f, "Handshake"),
        }
    }
}

pub enum AppEvent {
    AppState(AppState),
    FileInfo(FileInfo),
    Progress(f32),
    StatusMessage(String),
    Error(Box<dyn Error>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileInfo {
    pub path: Option<PathBuf>,
    pub size: u64,
    pub hash: Option<Vec<u8>>,
    pub name: String,
    pub sender_addr: Option<SocketAddr>
}

// for testing porpoises
impl Default for FileInfo {
    fn default() -> FileInfo {
        let a = FileInfo {
            path: None,
            size: 0,
            hash: None,
            name: "testfile".to_string(),
            sender_addr: None
        };

        let json = serde_json::to_string(&a).unwrap();
        println!("{}", json);
        a
    }
}

bitflags! {
    pub struct EelFlags: u8 {
        const shutting_down = 0b0000_0001;
        const allowed_to_close = 0b0000_0010;
        const file_valid = 0b0000_0100;
        const send_ip_valid = 0b0000_1000;
        const receive_ip_valid = 0b0001_0000;
        const send_port_valid = 0b0010_0000;
        const receive_port_valid = 0b0100_0000;
        const reserved = 0b1000_0000;
    }
}