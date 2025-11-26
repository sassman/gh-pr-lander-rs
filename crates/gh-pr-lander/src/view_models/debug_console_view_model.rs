use crate::logger::OwnedLogRecord;
use crate::state::DebugConsoleState;
use crate::theme::Theme;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

/// View model for debug console - handles presentation logic
pub struct DebugConsoleViewModel<'a> {
    state: &'a DebugConsoleState,
}

impl<'a> DebugConsoleViewModel<'a> {
    pub fn new(state: &'a DebugConsoleState) -> Self {
        Self { state }
    }

    /// Get the visible logs based on scroll offset and available height
    pub fn visible_logs(&self, available_height: usize) -> &[OwnedLogRecord] {
        let total_logs = self.state.logs.len();

        // Calculate the end index (where to stop showing logs)
        let end_index = total_logs.saturating_sub(self.state.scroll_offset);

        // Calculate the start index (where to start showing logs)
        let start_index = end_index.saturating_sub(available_height);

        if start_index < end_index && start_index < total_logs {
            &self.state.logs[start_index..end_index]
        } else {
            &[]
        }
    }

    /// Format a log record as a styled Line
    pub fn format_log_line(record: &OwnedLogRecord, theme: &Theme) -> Line<'static> {
        // Get current timestamp
        let datetime: chrono::DateTime<chrono::Local> = record.ts.into();
        let timestamp = datetime.format("%H:%M:%S%.3f").to_string();

        let level_style = match record.level {
            log::Level::Error => theme.log_error(),
            log::Level::Warn => theme.log_warning(),
            log::Level::Info => theme.log_info(),
            log::Level::Debug => theme.log_debug(),
            log::Level::Trace => theme.muted(),
        };

        Line::from(vec![
            Span::styled(format!("[{}]", timestamp), theme.muted().dim()),
            Span::raw(" "),
            Span::styled(format!("[{}]", record.level), level_style.bold()),
            Span::raw(" "),
            Span::styled(record.message.clone(), theme.text()),
        ])
    }

    /// Get the title for the debug console with scroll indicator
    pub fn title(&self) -> String {
        if self.state.scroll_offset > 0 {
            format!(
                " Debug Console (` to toggle, c to clear) - ↑{} ",
                self.state.scroll_offset
            )
        } else {
            " Debug Console (` to toggle, c to clear) ".to_string()
        }
    }
}
