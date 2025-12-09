//! View model for command palette
//!
//! Pre-computes all display data for the command palette view, separating
//! data preparation from rendering logic.

use crate::command_id::CommandId;
use crate::commands::{filter_commands, get_issue_commands, get_palette_commands_with_hints};
use crate::state::AppState;
use crate::utils::issue_extractor::RepoContext;
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
    /// Build view model from application state
    pub fn from_state(state: &AppState) -> Self {
        let theme = &state.theme;

        // Get static commands
        let mut all_commands = get_palette_commands_with_hints(&state.keymap);

        // Add dynamic issue commands based on selected PRs and repo context
        let pr_texts = Self::get_selected_pr_texts(state);
        let repo_ctx = Self::get_repo_context(state);
        log::debug!(
            "CommandPaletteViewModel: issue_tracker config count={}, pr_texts={:?}, repo_ctx={:?}",
            state.app_config.issue_tracker.len(),
            pr_texts,
            repo_ctx
        );
        let issue_commands =
            get_issue_commands(&state.app_config.issue_tracker, &pr_texts, &repo_ctx);
        log::debug!(
            "CommandPaletteViewModel: generated {} issue commands",
            issue_commands.len()
        );
        all_commands.extend(issue_commands);

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

        // Build visible rows
        let visible_rows: Vec<CommandRow> = filtered_commands
            .iter()
            .enumerate()
            .map(|(idx, cmd)| {
                let is_selected = idx == state.command_palette.selected_index;

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
        let selected_command = filtered_commands
            .get(state.command_palette.selected_index)
            .map(|cmd| SelectedCommandDetails {
                description: cmd.description().to_string(),
            });

        // Build footer hints from keymap
        let footer_hints = FooterHints {
            navigate_up: state
                .keymap
                .compact_hint_for_command(CommandId::NavigatePrevious)
                .unwrap_or_else(|| "↑".to_string()),
            navigate_down: state
                .keymap
                .compact_hint_for_command(CommandId::NavigateNext)
                .unwrap_or_else(|| "↓".to_string()),
            close: state
                .keymap
                .hint_for_command(CommandId::GlobalClose)
                .unwrap_or("Esc")
                .to_string(),
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

    /// Get PR texts from currently selected/active PRs for issue extraction
    fn get_selected_pr_texts(state: &AppState) -> Vec<String> {
        let repo_idx = state.main_view.selected_repository;
        let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) else {
            return vec![];
        };

        // If PRs are explicitly selected, use those; otherwise use cursor PR
        let pr_numbers: Vec<usize> = if repo_data.selected_pr_numbers.is_empty() {
            // Use cursor PR
            repo_data
                .prs
                .get(repo_data.selected_pr)
                .map(|pr| vec![pr.number])
                .unwrap_or_default()
        } else {
            // Use explicitly selected PRs
            repo_data.selected_pr_numbers.iter().copied().collect()
        };

        // Build text for each PR (title + description)
        pr_numbers
            .iter()
            .filter_map(|&num| repo_data.prs.iter().find(|pr| pr.number == num))
            .map(|pr| format!("{} {}", pr.title, pr.body))
            .collect()
    }

    /// Get repository context for issue extraction
    fn get_repo_context(state: &AppState) -> RepoContext {
        let repo_idx = state.main_view.selected_repository;
        state
            .main_view
            .repositories
            .get(repo_idx)
            .map(|repo| {
                RepoContext::new(
                    &repo.org,
                    &repo.repo,
                    repo.host.as_deref().unwrap_or(gh_client::DEFAULT_HOST),
                )
            })
            .unwrap_or_default()
    }
}
