//! Claude Terminal Reducer
//!
//! Handles state updates for the embedded terminal panel.

use crate::actions::{Action, ClaudeTerminalAction};
use crate::state::AppState;
use crate::views::{claude_terminal_view::ClaudeTerminalView, ViewId};

/// Reduce Claude terminal actions into state updates
pub fn reduce(state: &mut AppState, action: &Action) {
    if let Action::ClaudeTerminal(sub) = action {
        match sub {
            ClaudeTerminalAction::Open { session_name } => {
                state.claude_terminal.session_name = Some(session_name.clone());
                state.view_stack.push(Box::new(ClaudeTerminalView::new()));
            }
            ClaudeTerminalAction::ScreenUpdated(screen) => {
                state.claude_terminal.screen = Some(screen.clone());
            }
            ClaudeTerminalAction::PtyExited => {
                state.claude_terminal.screen = None;
                state.claude_terminal.session_name = None;
                state.claude_terminal.pty_writer = None;
                state.claude_terminal.last_size = (0, 0);
                // Pop view if the top is the terminal
                if state
                    .view_stack
                    .last()
                    .is_some_and(|v| v.view_id() == ViewId::ClaudeTerminal)
                {
                    state.view_stack.pop();
                }
            }
            ClaudeTerminalAction::SetWriter(writer) => {
                state.claude_terminal.pty_writer = Some(writer.clone());
            }
            ClaudeTerminalAction::Resize { cols, rows } => {
                state.claude_terminal.last_size = (*cols, *rows);
            }
            // KeyInput is handled by middleware only
            ClaudeTerminalAction::KeyInput(_) => {}
        }
    }
}
