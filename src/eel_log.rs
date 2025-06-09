use chrono::Local;
use crate::{Animation, AppState, FileInfo};

pub struct EelWatcher {
    pub app_state: AppState,
    pub messages: String,
    pub progress: f32,
    pub metadata: Option<FileInfo>,
    pub animation: Animation,
}

impl EelWatcher {
    pub fn new() -> Self {
        EelWatcher {
            app_state: AppState::Idle,
            messages: String::new(),
            progress: 0.0,
            metadata: None,
            animation: Animation::Idle,
        }
    }
    
    pub fn set_progress(&mut self, progress: f32) {
        self.progress = progress;
    }
    
    pub fn set_metadata(&mut self, metadata: FileInfo) {
        self.metadata = Some(metadata);
    }
    
    pub fn set_state(&mut self, state: AppState) {
        self.app_state = state;
    }
    
    pub fn log(&mut self, msg: &str) {
        let now = Local::now();
        let time_out_of_time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        
        let msg = format!("{}: {}\n", time_out_of_time, msg);
        
        self.messages.push_str(msg.as_str());
    }
}