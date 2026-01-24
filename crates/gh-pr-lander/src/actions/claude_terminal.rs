//! Claude Terminal actions
//!
//! Actions for the embedded terminal panel that displays Claude sessions.

use gh_pr_fix_with_claude::TerminalScreen;
use ratatui::crossterm::event::KeyEvent;

use crate::state::claude_terminal::PtyWriter;

/// Actions for the embedded Claude terminal panel
pub enum ClaudeTerminalAction {
    /// Open an embedded terminal attached to the given tmux session
    Open { session_name: String },
    /// Terminal screen buffer was updated
    ScreenUpdated(TerminalScreen),
    /// The PTY process exited
    PtyExited,
    /// Terminal area was resized
    Resize { cols: u16, rows: u16 },
    /// Forward a key press to the PTY
    KeyInput(KeyEvent),
    /// Store the PTY writer handle in state
    SetWriter(PtyWriter),
}

impl std::fmt::Debug for ClaudeTerminalAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open { session_name } => f
                .debug_struct("Open")
                .field("session_name", session_name)
                .finish(),
            Self::ScreenUpdated(_) => write!(f, "ScreenUpdated(...)"),
            Self::PtyExited => write!(f, "PtyExited"),
            Self::Resize { cols, rows } => f
                .debug_struct("Resize")
                .field("cols", cols)
                .field("rows", rows)
                .finish(),
            Self::KeyInput(key) => f.debug_tuple("KeyInput").field(key).finish(),
            Self::SetWriter(_) => write!(f, "SetWriter(...)"),
        }
    }
}

impl Clone for ClaudeTerminalAction {
    fn clone(&self) -> Self {
        match self {
            Self::Open { session_name } => Self::Open {
                session_name: session_name.clone(),
            },
            Self::ScreenUpdated(screen) => Self::ScreenUpdated(screen.clone()),
            Self::PtyExited => Self::PtyExited,
            Self::Resize { cols, rows } => Self::Resize {
                cols: *cols,
                rows: *rows,
            },
            Self::KeyInput(key) => Self::KeyInput(*key),
            Self::SetWriter(writer) => Self::SetWriter(writer.clone()),
        }
    }
}
