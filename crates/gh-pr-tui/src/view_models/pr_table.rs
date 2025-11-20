//! View model for PR table view
//!
//! Separates presentation logic from domain models and view rendering.
//! Pre-computes all display text, colors, and styles in the view model.

use crate::pr::{MergeableStatus, Pr};
use crate::state::{LoadingState, PrNumber, Repo, RepoData};
use crate::theme::Theme;
use ratatui::style::Color;

/// View model for the entire PR table
#[derive(Debug, Clone)]
pub struct PrTableViewModel {
    /// Header with title and status
    pub header: PrTableHeaderViewModel,

    /// Pre-computed rows ready to display
    pub rows: Vec<PrRowViewModel>,

    /// Current cursor position (for keyboard navigation)
    pub cursor_index: Option<usize>,
}

/// View model for table header
#[derive(Debug, Clone)]
pub struct PrTableHeaderViewModel {
    /// Title text: "GitHub PRs: org/repo@branch"
    pub title: String,

    /// Status text: "Loaded [Ctrl+r to refresh]", etc.
    pub status_text: String,

    /// Status color (from theme)
    pub status_color: Color,
}

/// View model for a single PR row
#[derive(Debug, Clone)]
pub struct PrRowViewModel {
    /// Pre-formatted cell texts
    pub pr_number: String, // "#123"
    pub title: String,       // "Fix: broken tests"
    pub author: String,      // "sassman"
    pub comments: String,    // "5"
    pub status_text: String, // "✓ Ready"

    /// Pre-computed styles
    pub bg_color: Color, // Background (alternating, selected, etc.)
    pub fg_color: Color,     // Text color
    pub status_color: Color, // Status-specific color

    /// Metadata for interactions (not displayed)
    pub pr_number_raw: usize, // For opening PR
    pub is_selected: bool, // Space key selection
    pub is_cursor: bool,   // Keyboard navigation position
    pub row_style: RowStyle,
}

/// Pre-determined row style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStyle {
    Normal,         // Regular row
    Selected,       // Space-selected
    Cursor,         // Keyboard focus
    SelectedCursor, // Both selected and focused
}

impl PrTableViewModel {
    /// Transform state into display-ready view model
    pub fn from_repo_data(
        repo_data: &RepoData,
        selected_repo: &Repo,
        cursor_index: Option<usize>,
        theme: &Theme,
    ) -> Self {
        // Build header
        let header = Self::build_header(repo_data, selected_repo, theme);

        // Build rows
        let rows = repo_data
            .prs
            .iter()
            .enumerate()
            .map(|(index, pr)| {
                Self::build_row(
                    pr,
                    index,
                    cursor_index,
                    &repo_data.selected_pr_numbers,
                    theme,
                )
            })
            .collect();

        Self {
            header,
            rows,
            cursor_index,
        }
    }

    fn build_header(
        repo_data: &RepoData,
        selected_repo: &Repo,
        theme: &Theme,
    ) -> PrTableHeaderViewModel {
        let title = format!(
            "GitHub PRs: {}/{}@{}",
            selected_repo.org, selected_repo.repo, selected_repo.branch
        );

        let (status_text, status_color) =
            Self::format_loading_state(&repo_data.loading_state, theme);

        PrTableHeaderViewModel {
            title,
            status_text,
            status_color,
        }
    }

    fn build_row(
        pr: &Pr,
        index: usize,
        cursor_index: Option<usize>,
        selected_prs: &std::collections::HashSet<PrNumber>,
        theme: &Theme,
    ) -> PrRowViewModel {
        // Pre-compute display text
        let pr_number = pr.number.to_string();
        let title = pr.title.clone();
        let author = pr.author.clone();
        let comments = pr.no_comments.to_string();

        // Format status with icon and label
        let status_icon = Self::mergeable_status_icon(pr.mergeable);
        let status_label = Self::mergeable_status_label(pr.mergeable);
        let status_text = format!("{} {}", status_icon, status_label);
        let status_color = Self::mergeable_status_color(pr.mergeable, theme);

        // Determine row state
        let is_selected = selected_prs.contains(&PrNumber::from_pr(pr));
        let is_cursor = cursor_index == Some(index);

        // Compute background color
        let bg_color = if is_cursor {
            theme.active_bg // Cursor takes precedence (whether selected or not)
        } else if is_selected {
            theme.selected_bg // Just selected (Space key)
        } else {
            // Alternating row colors
            if index.is_multiple_of(2) {
                theme.table_row_bg_normal
            } else {
                theme.table_row_bg_alt
            }
        };

        let fg_color = if is_cursor {
            theme.active_fg // Yellow for cursor
        } else {
            theme.table_row_fg
        };

        let row_style = match (is_cursor, is_selected) {
            (true, true) => RowStyle::SelectedCursor,
            (true, false) => RowStyle::Cursor,
            (false, true) => RowStyle::Selected,
            (false, false) => RowStyle::Normal,
        };

        PrRowViewModel {
            pr_number,
            title,
            author,
            comments,
            status_text,
            bg_color,
            fg_color,
            status_color,
            pr_number_raw: pr.number,
            is_selected,
            is_cursor,
            row_style,
        }
    }

    /// Format loading state for display (view model responsibility)
    fn format_loading_state(state: &LoadingState, theme: &Theme) -> (String, Color) {
        match state {
            LoadingState::Idle => ("Idle [Ctrl+r to refresh]".to_string(), theme.text_muted),
            LoadingState::Loading => ("Loading...".to_string(), theme.status_warning),
            LoadingState::Loaded => (
                "Loaded [Ctrl+r to refresh]".to_string(),
                theme.status_success,
            ),
            LoadingState::Error(err) => {
                let err_short = if err.len() > 30 {
                    format!("{}...", &err[..30])
                } else {
                    err.clone()
                };
                (
                    format!("Error: {} [Ctrl+r to retry]", err_short),
                    theme.status_error,
                )
            }
        }
    }

    // --- Presentation helpers for MergeableStatus ---
    // (Moved from MergeableStatus impl in pr.rs)

    fn mergeable_status_icon(status: MergeableStatus) -> &'static str {
        match status {
            MergeableStatus::Unknown => "?",
            MergeableStatus::BuildInProgress => "⋯",
            MergeableStatus::Ready => "✓",
            MergeableStatus::NeedsRebase => "↻",
            MergeableStatus::BuildFailed => "✗",
            MergeableStatus::Conflicted => "✗",
            MergeableStatus::Blocked => "⊗",
            MergeableStatus::Rebasing => "⟳",
            MergeableStatus::Merging => "⇒",
        }
    }

    fn mergeable_status_color(status: MergeableStatus, theme: &Theme) -> Color {
        match status {
            MergeableStatus::Unknown => theme.text_muted,
            MergeableStatus::BuildInProgress => theme.status_warning,
            MergeableStatus::Ready => theme.status_success,
            MergeableStatus::NeedsRebase => theme.status_warning,
            MergeableStatus::BuildFailed => theme.status_error,
            MergeableStatus::Conflicted => theme.status_error,
            MergeableStatus::Blocked => theme.status_error,
            MergeableStatus::Rebasing => theme.status_info,
            MergeableStatus::Merging => theme.status_info,
        }
    }

    fn mergeable_status_label(status: MergeableStatus) -> &'static str {
        match status {
            MergeableStatus::Unknown => "Unknown",
            MergeableStatus::BuildInProgress => "Checking...",
            MergeableStatus::Ready => "Ready",
            MergeableStatus::NeedsRebase => "Needs Rebase",
            MergeableStatus::BuildFailed => "Build Failed",
            MergeableStatus::Conflicted => "Conflicts",
            MergeableStatus::Blocked => "Blocked",
            MergeableStatus::Rebasing => "Rebasing...",
            MergeableStatus::Merging => "Merging...",
        }
    }
}
