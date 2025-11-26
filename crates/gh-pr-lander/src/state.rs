use crate::logger::OwnedLogRecord;

/// Identifies which view is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Splash,
    Main,
    DebugConsole,
}

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
#[derive(Debug, Clone)]
pub struct AppState {
    pub running: bool,
    pub active_view: ActiveView,
    pub splash: SplashState,
    pub debug_console: DebugConsoleState,
    pub theme: crate::theme::Theme,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            running: true,
            active_view: ActiveView::Splash,
            splash: SplashState::default(),
            debug_console: DebugConsoleState::default(),
            theme: crate::theme::Theme::default(),
        }
    }
}
