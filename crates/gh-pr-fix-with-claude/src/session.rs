//! GNU screen session management for Claude Code
//!
//! Provides spawn, attach, kill, and liveness-check operations.

use std::path::Path;
use std::process::Command;

/// Spawn a new Claude Code session in a detached GNU screen.
///
/// Returns the screen session name on success.
pub fn spawn_claude_session(
    org: &str,
    repo: &str,
    pr_number: usize,
    pr_title: &str,
    work_dir: &Path,
) -> Result<String, String> {
    let screen_name = format!("claude-{}-{}-pr-{}", org, repo, pr_number);

    // Kill any existing session with the same name (stale)
    let _ = Command::new("screen")
        .args(["-S", &screen_name, "-X", "quit"])
        .output();

    // Build the claude prompt
    let claude_prompt = format!(
        "Please analyze and fix PR #{} titled '{}' in this repository. \
         Check the PR description, review comments, and CI build logs \
         to understand what needs to be fixed. Then implement the fix.",
        pr_number, pr_title
    );

    // Spawn detached screen session running claude
    let output = Command::new("screen")
        .args([
            "-dmS",
            &screen_name,
            "claude",
            "--dangerously-skip-permissions",
            &claude_prompt,
        ])
        .current_dir(work_dir)
        .output()
        .map_err(|e| format!("Failed to spawn screen session: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("screen -dmS failed: {}", stderr));
    }

    log::info!(
        "Claude session '{}' started in {:?}",
        screen_name,
        work_dir
    );
    Ok(screen_name)
}

/// Check if a screen session is still alive.
pub fn is_session_alive(screen_name: &str) -> bool {
    let output = Command::new("screen").args(["-list"]).output();

    match output {
        Ok(output) => {
            let list = String::from_utf8_lossy(&output.stdout);
            list.contains(screen_name)
        }
        Err(_) => false,
    }
}

/// Kill a screen session.
pub fn kill_session(screen_name: &str) {
    let _ = Command::new("screen")
        .args(["-S", screen_name, "-X", "quit"])
        .output();
    log::info!("Killed screen session: {}", screen_name);
}

/// Attach to an existing screen session.
///
/// This function blocks until the user detaches from the session.
/// The caller is responsible for terminal suspend/resume.
///
/// Returns Ok(()) when the user detaches, Err if attach failed.
pub fn attach_session(screen_name: &str) -> Result<(), String> {
    let status = Command::new("screen")
        .args(["-r", screen_name])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to attach to screen session: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "screen -r exited with status: {}",
            status.code().unwrap_or(-1)
        ))
    }
}
