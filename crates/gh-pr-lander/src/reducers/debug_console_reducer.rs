use crate::actions::DebugConsoleAction;
use crate::capabilities::{PanelCapabilities, PanelCapabilityProvider};
use crate::logger::OwnedLogRecord;
use crate::state::DebugConsoleState;

/// Reducer for debug console state.
///
/// Accepts only DebugConsoleAction, making it type-safe and focused.
pub fn reduce_debug_console(
    mut state: DebugConsoleState,
    action: &DebugConsoleAction,
) -> DebugConsoleState {
    // Calculate max scroll based on visible height (if known)
    let max_scroll = if state.visible_height > 0 {
        state.logs.len().saturating_sub(state.visible_height)
    } else {
        state.logs.len()
    };

    match action {
        DebugConsoleAction::NavigateNext => {
            // Scroll towards newer logs (decrease offset, towards 0)
            // First cap to max_scroll in case we're beyond it
            state.scroll_offset = state.scroll_offset.min(max_scroll);
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
        }
        DebugConsoleAction::NavigatePrevious => {
            // Scroll towards older logs (increase offset, capped at max_scroll)
            if state.scroll_offset < max_scroll {
                state.scroll_offset = state.scroll_offset.saturating_add(1);
            }
        }
        DebugConsoleAction::NavigateToTop => {
            // Go to oldest logs
            state.scroll_offset = max_scroll;
        }
        DebugConsoleAction::NavigateToBottom => {
            // Go to newest logs (offset = 0)
            state.scroll_offset = 0;
        }
        DebugConsoleAction::Clear => {
            state.logs.clear();
            state.scroll_offset = 0;
        }
        DebugConsoleAction::LogAdded(log_record) => {
            state.logs.push(log_record.clone());
            if state.scroll_offset > 0 {
                // Keep viewing the same logs (offset increases as new logs are added)
                state.scroll_offset = state.scroll_offset.saturating_add(1);
            }
            // If scroll_offset == 0, stay at bottom (auto-scroll to newest)
        }
        DebugConsoleAction::DumpLogs => {
            // Dump logs to file
            if let Err(e) = dump_logs_to_file(&state.logs) {
                log::warn!("Failed to dump debug logs to file: {}", e);
            }
        }
        DebugConsoleAction::SetVisibleHeight(height) => {
            state.visible_height = *height;
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
