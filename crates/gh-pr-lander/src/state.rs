/// Application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub running: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self { running: true }
    }
}
