use ratatui::crossterm::event::KeyEvent;

/// Actions represent all possible state changes in the application.
/// Actions are prefixed by scope to indicate which part of the app they affect.
#[derive(Debug, Clone)]
pub enum Action {
    // Global actions (not tied to any specific view)
    GlobalKeyPressed(KeyEvent),
    GlobalClose,
    GlobalQuit,

    // Navigation actions (semantic, vim-style)
    NavNext,     // j, down arrow
    NavPrevious, // k, up arrow
    NavLeft,     // h, left arrow
    NavRight,    // l, right arrow

    // No-op action
    None,
}
