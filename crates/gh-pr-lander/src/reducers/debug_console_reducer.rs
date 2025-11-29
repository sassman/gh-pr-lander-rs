use crate::actions::Action;
use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::logger::OwnedLogRecord;
use crate::state::DebugConsoleState;
use crate::views::ViewId;

/// Reducer for debug console state
pub fn reduce(mut state: DebugConsoleState, action: &Action) -> DebugConsoleState {
    let is_active = state.visible;
    match action {
        Action::PushView(view) if view.view_id() == ViewId::DebugConsole => {
            state.visible = true;
        }
        Action::ReplaceView(_) | Action::GlobalClose => {
            state.visible = false;
            // Reset scroll when leaving
            state.scroll_offset = 0;
        }
        Action::DebugConsoleLogAdded(log_record) => {
            state.logs.push(log_record.clone());
            // If we're at the bottom (scroll_offset == 0), stay at bottom
            // Otherwise, keep current scroll position
        }
        Action::DebugConsoleDumpLogs => {
            // Dump logs to file
            if let Err(e) = dump_logs_to_file(&state.logs) {
                log::warn!("Failed to dump debug logs to file: {}", e);
            }
        }
        Action::LocalKeyPressed(c) if *c == 'c' && is_active => {
            // Handle local 'c' key - clear logs
            state.logs.clear();
            state.scroll_offset = 0;
        }
        Action::DebugConsoleClear if is_active => {
            state.logs.clear();
            state.scroll_offset = 0;
        }
        Action::NavigateNext if is_active => {
            // Scroll down (increase offset = go back in history)
            if state.scroll_offset > 0 {
                state.scroll_offset = state.scroll_offset.saturating_sub(1);
            }
        }
        Action::NavigatePrevious if is_active => {
            // Scroll up (decrease offset = go forward in history)
            state.scroll_offset = state.scroll_offset.saturating_add(1);
        }
        Action::ScrollToTop if is_active => {
            // Go to oldest log (maximum offset)
            state.scroll_offset = state.logs.len();
        }
        Action::ScrollToBottom if is_active => {
            // Go to newest log (offset = 0)
            state.scroll_offset = 0;
        }
        Action::ScrollPageDown if is_active => {
            // Scroll down one page (10 lines)
            state.scroll_offset = state.scroll_offset.saturating_sub(10);
        }
        Action::ScrollPageUp if is_active => {
            // Scroll up one page (10 lines)
            state.scroll_offset = state.scroll_offset.saturating_add(10);
        }
        Action::ScrollHalfPageDown if is_active => {
            // Scroll down half page (5 lines)
            state.scroll_offset = state.scroll_offset.saturating_sub(5);
        }
        Action::ScrollHalfPageUp if is_active => {
            // Scroll up half page (5 lines)
            state.scroll_offset = state.scroll_offset.saturating_add(5);
        }
        _ => {
            // Unhandled actions - no state change
        }
    }

    state
}

fn dump_logs_to_file(logs: &[OwnedLogRecord]) -> anyhow::Result<()> {
    use chrono::Local;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let filename = format!("debug-{}.log", timestamp);
    let mut path = PathBuf::from(".");
    path.push(filename);

    let mut file = File::create(&path)?;
    for log in logs {
        writeln!(file, "{}", log)?;
    }

    log::info!("Debug logs dumped to file: {:?}", path);
    Ok(())
}

impl PanelCapabilityProvider for DebugConsoleState {
    fn capabilities(&self) -> PanelCapabilities {
        // Debug console supports vim navigation and vertical scrolling with vim bindings
        PanelCapabilities::VIM_NAVIGATION_BINDINGS
            | PanelCapabilities::SCROLL_VERTICAL
            | PanelCapabilities::VIM_SCROLL_BINDINGS
    }
}
