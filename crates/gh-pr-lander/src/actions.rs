use ratatui::crossterm::event::KeyEvent;

use crate::{logger::OwnedLogRecord, state::ActiveView};

/// Actions represent all possible state changes in the application.
/// Actions are prefixed by scope to indicate which part of the app they affect.
#[derive(Debug, Clone)]
pub enum Action {
    // Global actions (not tied to any specific view)
    GlobalKeyPressed(KeyEvent),
    GlobalClose,
    GlobalQuit,
    GlobalActivateView(ActiveView),

    // Local actions (dispatched to active view for handling)
    LocalKeyPressed(char), // Key pressed in active view context

    // Navigation actions (semantic, vim-style)
    NavigateNext,     // j, down arrow
    NavigatePrevious, // k, up arrow
    NavigateLeft,     // h, left arrow
    NavigateRight,    // l, right arrow

    // Scroll actions
    ScrollToTop,        // gg
    ScrollToBottom,     // G
    ScrollPageDown,     // Page Down
    ScrollPageUp,       // Page Up
    ScrollHalfPageDown, // Ctrl+d
    ScrollHalfPageUp,   // Ctrl+u

    // Debug console actions
    DebugConsoleClear,                    // Clear debug console logs
    DebugConsoleLogAdded(OwnedLogRecord), // New log record added

    // Bootstrap actions
    BootstrapStart,
    BootstrapEnd,

    // Animation/Timer actions
    Tick, // Periodic tick for animations (500ms interval)

    // No-op action
    None,
}
