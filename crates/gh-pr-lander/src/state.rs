use crate::logger::OwnedLogRecord;
use crate::views::{SplashView, View};

/// Debug console state
#[derive(Debug, Clone)]
pub struct DebugConsoleState {
    pub visible: bool,
    pub logs: Vec<OwnedLogRecord>,
    pub scroll_offset: usize, // Current scroll position (0 = bottom/latest)
}

impl Default for DebugConsoleState {
    fn default() -> Self {
        Self {
            visible: false,
            logs: Vec::new(),
            scroll_offset: 0,
        }
    }
}

/// Splash screen state
#[derive(Debug, Clone)]
pub struct SplashState {
    pub bootstrapping: bool,
    pub animation_frame: usize, // Current frame of the snake animation (0-15)
}

impl Default for SplashState {
    fn default() -> Self {
        Self {
            bootstrapping: true,
            animation_frame: 0,
        }
    }
}

/// Application state
pub struct AppState {
    pub running: bool,
    pub active_view: Box<dyn View>,
    pub splash: SplashState,
    pub debug_console: DebugConsoleState,
    pub theme: crate::theme::Theme,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("running", &self.running)
            .field("active_view", &self.active_view)
            .field("splash", &self.splash)
            .field("debug_console", &self.debug_console)
            .field("theme", &"<theme>")
            .finish()
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            running: self.running,
            active_view: self.active_view.clone(),
            splash: self.splash.clone(),
            debug_console: self.debug_console.clone(),
            theme: self.theme.clone(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            active_view: Box::new(SplashView::new()),
            splash: SplashState::default(),
            debug_console: DebugConsoleState::default(),
            theme: crate::theme::Theme::default(),
        }
    }
}
