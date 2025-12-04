//! Diff Viewer Actions
//!
//! Tagged actions for the diff viewer panel.

use gh_diff_viewer::{DiffEvent, PullRequestDiff};

/// Tagged actions for the diff viewer panel
#[derive(Debug, Clone)]
pub enum DiffViewerAction {
    // === Loading ===
    /// Open diff viewer for current PR (triggers async fetch)
    Open,
    /// Loading started
    LoadStart,
    /// Diff loaded successfully
    Loaded {
        diff: PullRequestDiff,
        pr_number: u64,
        pr_title: String,
    },
    /// Loading failed
    LoadError(String),

    // === Navigation (delegated from generic Navigate actions) ===
    /// Navigate to next item (file or line)
    NavigateDown,
    /// Navigate to previous item (file or line)
    NavigateUp,
    /// Navigate left (to file tree or previous pane)
    NavigateLeft,
    /// Navigate right (to diff content or next pane)
    NavigateRight,
    /// Navigate to top
    NavigateToTop,
    /// Navigate to bottom
    NavigateToBottom,

    // === Scrolling ===
    /// Page down
    PageDown,
    /// Page up
    PageUp,

    // === Tree Operations ===
    /// Expand/collapse file in tree
    Toggle,
    /// Expand all files
    ExpandAll,
    /// Collapse all files
    CollapseAll,

    // === Focus Management ===
    /// Switch focus between file tree and diff content
    SwitchPane,

    // === Visual Mode ===
    /// Enter visual mode for line selection
    EnterVisualMode,
    /// Exit visual mode
    ExitVisualMode,

    // === Comments ===
    /// Start adding a comment on current line
    AddComment,
    /// Cancel comment editing
    CancelComment,
    /// Commit the current comment
    CommitComment,
    /// Insert character into comment editor
    CommentChar(char),
    /// Delete character from comment editor
    CommentBackspace,

    // === Review ===
    /// Show review popup
    ShowReviewPopup,
    /// Hide review popup
    HideReviewPopup,
    /// Navigate review popup options
    ReviewOptionNext,
    /// Navigate review popup options
    ReviewOptionPrev,
    /// Submit review with selected option
    SubmitReview,

    // === Events from DiffViewerState ===
    /// Forward an event from the diff viewer state
    Event(DiffEvent),

    // === Viewport ===
    /// Update viewport dimensions
    SetViewport { width: u16, height: u16 },
}
