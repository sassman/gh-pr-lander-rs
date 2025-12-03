use crate::command_id::CommandId;
use crate::keybindings::Keymap;
use crate::logger::OwnedLogRecord;
use crate::state::DebugConsoleState;
use gh_pr_lander_theme::Theme;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

/// Pre-computed footer hints for keyboard shortcuts
#[derive(Debug, Clone)]
pub struct DebugConsoleFooterHints {
    /// Combined scroll hint (e.g., "j/↓/k/↑")
    pub scroll: String,
    /// Combined top/bottom hint (e.g., "gg/G")
    pub top_bottom: String,
    /// Close hint (e.g., "`")
    pub close: String,
}

/// View model for debug console - handles presentation logic
pub struct DebugConsoleViewModel<'a> {
    state: &'a DebugConsoleState,
    /// Pre-computed footer hints
    pub footer_hints: DebugConsoleFooterHints,
}

impl<'a> DebugConsoleViewModel<'a> {
    pub fn new(state: &'a DebugConsoleState, keymap: &Keymap) -> Self {
        let footer_hints = DebugConsoleFooterHints {
            scroll: format!(
                "{}/{}",
                keymap
                    .compact_hint_for_command(CommandId::NavigateNext)
                    .unwrap_or_else(|| "j/↓".to_string()),
                keymap
                    .compact_hint_for_command(CommandId::NavigatePrevious)
                    .unwrap_or_else(|| "k/↑".to_string()),
            ),
            top_bottom: format!(
                "{}/{}",
                keymap
                    .compact_hint_for_command(CommandId::NavigateToTop)
                    .unwrap_or_else(|| "gg".to_string()),
                keymap
                    .compact_hint_for_command(CommandId::NavigateToBottom)
                    .unwrap_or_else(|| "G".to_string()),
            ),
            close: keymap
                .compact_hint_for_command(CommandId::DebugToggleConsoleView)
                .unwrap_or_else(|| "`".to_string()),
        };

        Self {
            state,
            footer_hints,
        }
    }

    /// Get the visible logs based on scroll offset and available height
    ///
    /// scroll_offset = 0 means we're at the bottom (showing newest logs)
    /// scroll_offset > 0 means we've scrolled up (showing older logs)
    pub fn visible_logs(&self, available_height: usize) -> &[OwnedLogRecord] {
        let total_logs = self.state.logs.len();

        if total_logs == 0 || available_height == 0 {
            return &[];
        }

        // Cap scroll_offset to valid range (can't scroll past showing the first log)
        let max_scroll = total_logs.saturating_sub(available_height);
        let effective_scroll = self.state.scroll_offset.min(max_scroll);

        // end is the index AFTER the last visible log
        // When effective_scroll=0, end=total_logs (newest logs at the end)
        let end = total_logs.saturating_sub(effective_scroll);

        // start is the index of the first visible log
        let start = end.saturating_sub(available_height);

        &self.state.logs[start..end]
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
                " Debug Console (c to clear) - ↓{} ",
                self.state.scroll_offset
            )
        } else {
            " Debug Console (c to clear) ".to_string()
        }
    }
}
