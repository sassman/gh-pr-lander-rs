//! Tmux session management for Claude Code
//!
//! Provides spawn, attach, kill, and liveness-check operations.

use std::path::Path;
use std::process::Command;

/// Spawn a new Claude Code session in a detached tmux session.
///
/// Returns the tmux session name on success.
pub fn spawn_claude_session(
    org: &str,
    repo: &str,
    pr_number: usize,
    pr_title: &str,
    work_dir: &Path,
) -> Result<String, String> {
    let session_name = format!("claude-{}-{}-pr-{}", org, repo, pr_number);

    // Kill any existing session with the same name (stale)
    let _ = Command::new("tmux")
        .args(["kill-session", "-t", &session_name])
        .output();

    // Build the claude prompt
    let claude_prompt = format!(
        "Please analyze and fix PR #{} titled '{}' in this repository. \
         Check the PR description, review comments, and CI build logs \
         to understand what needs to be fixed. Then implement the fix.",
        pr_number, pr_title
    );

    // Build the shell command to run inside tmux
    // Use POSIX single-quote escaping for the prompt
    let escaped_prompt = claude_prompt.replace('\'', "'\\''");
    let shell_cmd = format!("claude --dangerously-skip-permissions '{}'", escaped_prompt);

    // Spawn detached tmux session running claude
    let output = Command::new("tmux")
        .args(["new-session", "-d", "-s", &session_name, &shell_cmd])
        .current_dir(work_dir)
        .output()
        .map_err(|e| format!("Failed to spawn tmux session: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux new-session failed: {}", stderr));
    }

    log::info!(
        "Claude session '{}' started in {:?}",
        session_name,
        work_dir
    );
    Ok(session_name)
}

/// Check if a tmux session is still alive.
pub fn is_session_alive(session_name: &str) -> bool {
    Command::new("tmux")
        .args(["has-session", "-t", session_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Kill a tmux session.
pub fn kill_session(session_name: &str) {
    let _ = Command::new("tmux")
        .args(["kill-session", "-t", session_name])
        .output();
    log::info!("Killed tmux session: {}", session_name);
}

/// Attach to an existing tmux session.
///
/// This function blocks until the user detaches from the session.
/// The caller is responsible for terminal suspend/resume.
///
/// Returns Ok(()) when the user detaches, Err if attach failed.
pub fn attach_session(session_name: &str) -> Result<(), String> {
    let status = Command::new("tmux")
        .args(["attach-session", "-t", session_name])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to attach to tmux session: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "tmux attach exited with status: {}",
            status.code().unwrap_or(-1)
        ))
    }
}
