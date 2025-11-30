//! Pull Request model
//!
//! Domain model for GitHub Pull Requests.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A GitHub Pull Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pr {
    /// PR number
    pub number: usize,
    /// PR title
    pub title: String,
    /// PR body/description
    pub body: String,
    /// Author username
    pub author: String,
    /// Number of comments
    pub comments: usize,
    /// Current mergeable status
    pub mergeable: MergeableStatus,
    /// Whether the PR needs rebase (behind base branch)
    pub needs_rebase: bool,
    /// HEAD commit SHA (for CI status checks)
    pub head_sha: String,
    /// When the PR was created
    pub created_at: DateTime<Utc>,
    /// When the PR was last updated
    pub updated_at: DateTime<Utc>,
    /// HTML URL for viewing the PR in browser
    pub html_url: String,
}

impl Pr {
    /// Create a new PR with the given data
    pub fn new(
        number: usize,
        title: impl Into<String>,
        author: impl Into<String>,
        head_sha: impl Into<String>,
    ) -> Self {
        Self {
            number,
            title: title.into(),
            body: String::new(),
            author: author.into(),
            comments: 0,
            mergeable: MergeableStatus::Unknown,
            needs_rebase: false,
            head_sha: head_sha.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            html_url: String::new(),
        }
    }

    /// Set the HTML URL
    pub fn with_html_url(mut self, url: impl Into<String>) -> Self {
        self.html_url = url.into();
        self
    }
}

/// Mergeable status of a Pull Request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MergeableStatus {
    /// Not yet checked
    #[default]
    Unknown,
    /// Background check in progress
    Checking,
    /// Ready to merge (no issues)
    Ready,
    /// Branch is behind, needs rebase
    NeedsRebase,
    /// CI/build checks failed
    BuildFailed,
    /// Has merge conflicts
    Conflicted,
    /// Blocked by reviews or other checks
    Blocked,
    /// Currently rebasing (transient state)
    Rebasing,
    /// Currently merging (transient state)
    Merging,
}

impl MergeableStatus {
    /// Get the display icon for this status
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Unknown => "?",
            Self::Checking => "⋯",
            Self::Ready => "✓",
            Self::NeedsRebase => "↻",
            Self::BuildFailed => "✗",
            Self::Conflicted => "✗",
            Self::Blocked => "⊗",
            Self::Rebasing => "⟳",
            Self::Merging => "⇒",
        }
    }

    /// Get the display label for this status
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Checking => "Checking...",
            Self::Ready => "Ready",
            Self::NeedsRebase => "Needs Rebase",
            Self::BuildFailed => "Build Failed",
            Self::Conflicted => "Conflicts",
            Self::Blocked => "Blocked",
            Self::Rebasing => "Rebasing...",
            Self::Merging => "Merging...",
        }
    }
}

/// Loading state for PR data
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LoadingState {
    /// Not started loading
    #[default]
    Idle,
    /// Currently loading
    Loading,
    /// Successfully loaded
    Loaded,
    /// Failed to load
    Error(String),
}
