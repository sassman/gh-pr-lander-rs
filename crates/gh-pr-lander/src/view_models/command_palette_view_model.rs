//! View model for command palette
//!
//! Pre-computes all display data for the command palette view, separating
//! data preparation from rendering logic.

use crate::command_id::CommandId;
use crate::commands::{build_palette_commands, filter_commands};
use crate::state::AppState;
use ratatui::style::Color;

/// View model for the command palette
#[derive(Debug, Clone)]
pub struct CommandPaletteViewModel {
    /// Total number of commands (before filtering)
    pub total_commands: usize,
    /// Pre-formatted input text for display
    pub input_text: String,
    /// Is input empty (for placeholder styling)
    pub input_is_empty: bool,
    /// Visible command rows (after filtering)
    pub visible_rows: Vec<CommandRow>,
    /// Currently selected command details
    pub selected_command: Option<SelectedCommandDetails>,
    /// Maximum category width for column sizing
    pub max_category_width: u16,
    /// Footer hints for navigation
    pub footer_hints: FooterHints,
}

/// Pre-computed footer hints for keyboard shortcuts
#[derive(Debug, Clone)]
pub struct FooterHints {
    /// Hint for navigate up (e.g., "k/↑")
    pub navigate_up: String,
    /// Hint for navigate down (e.g., "j/↓")
    pub navigate_down: String,
    /// Hint for close (e.g., "q/Esc")
    pub close: String,
}

/// A single row in the command list
#[derive(Debug, Clone)]
pub struct CommandRow {
    /// Is this row selected?
    pub is_selected: bool,
    /// Selection indicator ("> " or "  ")
    pub indicator: String,
    /// Formatted shortcut hint (12 chars + 1 space)
    pub shortcut_hint: String,
    /// Command title
    pub title: String,
    /// Formatted category with brackets and right-alignment
    pub category: String,
    /// Text color for this row (reserved for future use)
    #[allow(dead_code)]
    pub fg_color: Color,
    /// Background color for this row (reserved for future use)
    #[allow(dead_code)]
    pub bg_color: Color,
}

/// Details about the selected command
#[derive(Debug, Clone)]
pub struct SelectedCommandDetails {
    /// Command description
    pub description: String,
}

impl CommandPaletteViewModel {
    /// Build view model from application state.
    ///
    /// `inner_height` is the row count the results table will actually render
    /// into — the view computes it from its layout and hands it down. Offset
    /// is derived here, so neither the reducer nor app state needs to know
    /// about widget geometry.
    pub fn from_state(state: &AppState, inner_height: u16) -> Self {
        let theme = &state.theme;

        // Single source of truth — same list reducer clamps and middleware executes.
        let all_commands = build_palette_commands(state);

        let total_commands = all_commands.len();

        // Filter commands based on query
        let filtered_commands = filter_commands(&all_commands, &state.command_palette.query);

        // Pre-format input text
        let input_text = state.command_palette.query.clone();
        let input_is_empty = input_text.is_empty();

        // Calculate max category width for right-alignment
        // Add 2 for brackets [] and 2 for padding
        let max_category_width = filtered_commands
            .iter()
            .map(|cmd| cmd.category().len())
            .max()
            .unwrap_or(10) as u16
            + 4;

        // Clamp selection against the current filtered list (defensive — the
        // reducer already clamps on navigation, but typing changes the list).
        let selected_index = state
            .command_palette
            .selected_index
            .min(filtered_commands.len().saturating_sub(1));

        // Derive the visible window from selection + rendered area:
        //   anchor selection at the bottom of the viewport when scrolled,
        //   otherwise show from the top.
        let viewport = inner_height as usize;
        let (offset, end) = if viewport == 0 || filtered_commands.is_empty() {
            (0, filtered_commands.len())
        } else {
            let offset = selected_index
                .saturating_sub(viewport.saturating_sub(1))
                .min(filtered_commands.len().saturating_sub(viewport.min(filtered_commands.len())));
            let end = (offset + viewport).min(filtered_commands.len());
            (offset, end)
        };

        let visible_rows: Vec<CommandRow> = filtered_commands[offset..end]
            .iter()
            .enumerate()
            .map(|(local_idx, cmd)| {
                let absolute_idx = offset + local_idx;
                let is_selected = absolute_idx == selected_index;

                // Selection indicator
                let indicator = if is_selected {
                    "> ".to_string()
                } else {
                    "  ".to_string()
                };

                // Format shortcut hint (13 chars: 12 for hint + 1 space)
                let shortcut_hint = if let Some(ref hint) = cmd.shortcut_hint {
                    format!("{:12} ", hint)
                } else {
                    "             ".to_string()
                };

                // Format category with right alignment
                let category = format!("[{}]", cmd.category());
                let category = format!("{:>width$}", category, width = max_category_width as usize);

                // Colors
                let (fg_color, bg_color) = if is_selected {
                    // Use active_fg (yellow) for text and selected_bg for background
                    (theme.active_fg, theme.selected_bg)
                } else {
                    (theme.text().fg.unwrap_or(Color::White), Color::Reset)
                };

                CommandRow {
                    is_selected,
                    indicator,
                    shortcut_hint,
                    title: cmd.title().to_string(),
                    category,
                    fg_color,
                    bg_color,
                }
            })
            .collect();

        // Get selected command details
        let selected_command =
            filtered_commands
                .get(selected_index)
                .map(|cmd| SelectedCommandDetails {
                    description: cmd.description().to_string(),
                });

        // Build footer hints from keymap. The command palette has TEXT_INPUT
        // capability, so single-ASCII-char bindings (j, k, …) are swallowed by
        // the input field and shouldn't appear in the navigation hint.
        let keep_non_text_input = |hint: &str| !(hint.len() == 1 && hint.is_ascii());

        let footer_hints = FooterHints {
            navigate_up: state
                .keymap
                .compact_hint_for_command_filtered(
                    CommandId::NavigatePrevious,
                    keep_non_text_input,
                )
                .unwrap_or_else(|| "↑".to_string()),
            navigate_down: state
                .keymap
                .compact_hint_for_command_filtered(
                    CommandId::NavigateNext,
                    keep_non_text_input,
                )
                .unwrap_or_else(|| "↓".to_string()),
            close: state
                .keymap
                .compact_hint_for_command_filtered(CommandId::GlobalClose, keep_non_text_input)
                .unwrap_or_else(|| "Esc".to_string()),
        };

        Self {
            total_commands,
            input_text,
            input_is_empty,
            visible_rows,
            selected_command,
            max_category_width,
            footer_hints,
        }
    }
}
