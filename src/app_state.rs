pub struct AppState {
    pub progress_bar_state: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            progress_bar_state: 0,
        }
    }
}