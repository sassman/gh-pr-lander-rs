//! Claude Terminal State
//!
//! State for the embedded terminal panel showing Claude sessions.

use gh_pr_fix_with_claude::TerminalScreen;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Shared PTY writer handle
pub type PtyWriter = Arc<Mutex<Box<dyn Write + Send>>>;

/// State for the embedded Claude terminal panel
#[derive(Clone, Default)]
pub struct ClaudeTerminalState {
    /// Current terminal screen buffer
    pub screen: Option<TerminalScreen>,
    /// Name of the attached tmux session
    pub session_name: Option<String>,
    /// Last known PTY size (cols, rows) - the inner terminal dimensions
    pub last_size: (u16, u16),
    /// Writer handle for forwarding keystrokes to the PTY
    pub pty_writer: Option<PtyWriter>,
    /// Current terminal frame area (width, height) — set by main loop
    pub terminal_area: (u16, u16),
}

/// Compute the inner terminal dimensions for the popup panel given the terminal area
pub fn popup_inner_size(terminal_width: u16, terminal_height: u16) -> (u16, u16) {
    let popup_width = (terminal_width * 80 / 100).max(40);
    let popup_height = (terminal_height * 80 / 100).max(10);
    // Subtract 2 for borders on each axis
    (
        popup_width.saturating_sub(2),
        popup_height.saturating_sub(2),
    )
}

impl std::fmt::Debug for ClaudeTerminalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeTerminalState")
            .field(
                "screen",
                &self
                    .screen
                    .as_ref()
                    .map(|s| format!("{}x{}", s.cols, s.lines)),
            )
            .field("session_name", &self.session_name)
            .field("last_size", &self.last_size)
            .field("terminal_area", &self.terminal_area)
            .field("pty_writer", &self.pty_writer.is_some())
            .finish()
    }
}
