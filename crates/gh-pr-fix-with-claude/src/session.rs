//! Session management for Claude Code
//!
//! Provides spawn, attach, kill, and liveness-check operations for tmux and zellij.

use crate::config::{FixWithClaudeConfig, Multiplexer};
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
    config: &FixWithClaudeConfig,
) -> Result<String, String> {
    let session_name = format!("gh-pr-lander-fix-with-claude--{org}-{repo}-pr-{pr_number}");

    // Kill any existing session with the same name (stale)
    match config.multiplexer {
        Multiplexer::Tmux => {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", &session_name])
                .output();
        }
        Multiplexer::Zellij => {
            let _ = Command::new("zellij")
                .args(["kill-session", &session_name])
                .output();
        }
    }

    // Build the claude prompt from config
    let claude_prompt = config.build_prompt(pr_number, pr_title);

    // Build the shell command with appropriate permissions
    let escaped_prompt = claude_prompt.replace('\'', "'\\''");
    let shell_cmd = if config.permissions.is_unrestricted() {
        format!("claude --dangerously-skip-permissions '{escaped_prompt}'")
    } else {
        let mut args = Vec::new();

        // Add allowed tools
        if !config.permissions.tools_allowed().is_empty() {
            let tools: Vec<_> = config
                .permissions
                .tools_allowed()
                .iter()
                .map(|t| t.as_str())
                .collect();
            args.push(format!("--allowedTools '{}'", tools.join(",")));
        }

        // Add denied tools
        if !config.permissions.tools_denied().is_empty() {
            let tools: Vec<_> = config
                .permissions
                .tools_denied()
                .iter()
                .map(|t| t.as_str())
                .collect();
            args.push(format!("--disallowedTools '{}'", tools.join(",")));
        }

        format!("claude {} '{}'", args.join(" "), escaped_prompt)
    };

    // Spawn detached session running claude
    let output = match config.multiplexer {
        Multiplexer::Tmux => Command::new("tmux")
            .args(["new-session", "-d", "-s", &session_name, &shell_cmd])
            .current_dir(work_dir)
            .output()
            .map_err(|e| format!("Failed to spawn tmux session: {e}"))?,
        Multiplexer::Zellij => Command::new("zellij")
            .args(["-s", &session_name, "--", "sh", "-c", &shell_cmd])
            .current_dir(work_dir)
            .output()
            .map_err(|e| format!("Failed to spawn zellij session: {e}"))?,
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mux_name = match config.multiplexer {
            Multiplexer::Tmux => "tmux",
            Multiplexer::Zellij => "zellij",
        };
        return Err(format!("{mux_name} new-session failed: {stderr}"));
    }

    log::info!("Claude session '{session_name}' started in {work_dir:?}");
    Ok(session_name)
}

/// Check if a session is still alive.
pub fn is_session_alive(session_name: &str, multiplexer: &Multiplexer) -> bool {
    match multiplexer {
        Multiplexer::Tmux => Command::new("tmux")
            .args(["has-session", "-t", session_name])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false),
        Multiplexer::Zellij => Command::new("zellij")
            .args(["list-sessions"])
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .any(|line| line.contains(session_name))
            })
            .unwrap_or(false),
    }
}

/// Kill a multiplexer session.
pub fn kill_session(session_name: &str, multiplexer: &Multiplexer) {
    match multiplexer {
        Multiplexer::Tmux => {
            let _ = Command::new("tmux")
                .args(["kill-session", "-t", session_name])
                .output();
        }
        Multiplexer::Zellij => {
            let _ = Command::new("zellij")
                .args(["kill-session", session_name])
                .output();
        }
    }
    log::info!("Killed session: {}", session_name);
}

/// Attach to an existing session.
///
/// This function blocks until the user detaches from the session.
/// The caller is responsible for terminal suspend/resume.
///
/// Returns Ok(()) when the user detaches, Err if attach failed.
pub fn attach_session(session_name: &str, multiplexer: &Multiplexer) -> Result<(), String> {
    let status = match multiplexer {
        Multiplexer::Tmux => Command::new("tmux")
            .args(["attach-session", "-t", session_name])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("Failed to attach to tmux session: {}", e))?,
        Multiplexer::Zellij => Command::new("zellij")
            .args(["attach", session_name])
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .map_err(|e| format!("Failed to attach to zellij session: {}", e))?,
    };

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "attach exited with status: {}",
            status.code().unwrap_or(-1)
        ))
    }
}
