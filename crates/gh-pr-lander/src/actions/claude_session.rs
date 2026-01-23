//! Claude Code session actions

use gh_pr_fix_with_claude::PrId;

/// Actions for Claude Code session management
#[derive(Debug, Clone)]
pub enum ClaudeSessionAction {
    /// User-initiated: spawn Claude to fix this PR
    Start,
    /// User-initiated: attach to running session for this PR
    Attach,
    /// Internal: session successfully started
    Started {
        pr_id: PrId,
        screen_name: String,
        work_dir: String,
    },
    /// Internal: session ended (detected terminated or cleaned up)
    Completed { pr_id: PrId },
    /// Internal: session start failed
    Error(String),
    /// Internal: request terminal suspend for screen attach
    SuspendForAttach { screen_name: String },
}
