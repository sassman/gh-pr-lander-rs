# Configurable Multiplexer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Allow users to choose between tmux and zellij as the terminal multiplexer for "Fix with Claude Code" sessions.

**Architecture:** Enum + match dispatch. A `Multiplexer` enum (Tmux/Zellij, default Zellij) is added to `FixWithClaudeConfig`. Each session operation matches on the enum to select shell commands. No traits needed.

**Tech Stack:** Rust, serde (for TOML deserialization), std::process::Command, portable-pty

**Design doc:** `docs/plans/2026-02-18-multiplexer-config-design.md`

---

### Task 1: Add Multiplexer enum and config field

**Files:**
- Modify: `crates/gh-pr-fix-with-claude/src/config.rs:86-110`

**Step 1: Write the failing test**

Add to existing `mod tests` in `config.rs`:

```rust
#[test]
fn test_multiplexer_default_is_zellij() {
    let config = FixWithClaudeConfig::default();
    assert_eq!(config.multiplexer, Multiplexer::Zellij);
}

#[test]
fn test_multiplexer_tmux_from_toml() {
    let toml = r#"
prompt = "Fix it"
multiplexer = "tmux"
    "#;
    let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.multiplexer, Multiplexer::Tmux);
}

#[test]
fn test_multiplexer_zellij_from_toml() {
    let toml = r#"
prompt = "Fix it"
multiplexer = "zellij"
    "#;
    let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.multiplexer, Multiplexer::Zellij);
}

#[test]
fn test_multiplexer_default_when_omitted() {
    let toml = r#"
prompt = "Fix it"
    "#;
    let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
    assert_eq!(config.multiplexer, Multiplexer::Zellij);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gh-pr-fix-with-claude -- test_multiplexer`
Expected: FAIL - `Multiplexer` type does not exist

**Step 3: Write minimal implementation**

Add the enum before `FixWithClaudeConfig` (after line 73, before line 75):

```rust
/// Terminal multiplexer backend for Claude Code sessions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Multiplexer {
    Tmux,
    #[default]
    Zellij,
}
```

Add field to `FixWithClaudeConfig` struct (after line 96):

```rust
    /// Terminal multiplexer to use (tmux or zellij)
    #[serde(default)]
    pub multiplexer: Multiplexer,
```

Update `Default for FixWithClaudeConfig` impl (line 103-110) to include:

```rust
impl Default for FixWithClaudeConfig {
    fn default() -> Self {
        Self {
            prompt: default_prompt(),
            permissions: Permissions::default(),
            multiplexer: Multiplexer::default(),
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p gh-pr-fix-with-claude -- test_multiplexer`
Expected: All 4 new tests PASS

**Step 5: Run full crate tests**

Run: `cargo test -p gh-pr-fix-with-claude`
Expected: All existing tests still pass (existing tests don't specify `multiplexer`, it uses default)

**Step 6: Commit**

```
feat(config): add Multiplexer enum (tmux/zellij) to FixWithClaudeConfig
```

---

### Task 2: Update TOML generation to include multiplexer

**Files:**
- Modify: `crates/gh-pr-fix-with-claude/src/config.rs:120-139` (generate_toml_section)

**Step 1: Update existing test**

Update `test_generate_toml_section` in `config.rs`:

```rust
#[test]
fn test_generate_toml_section() {
    let toml = FixWithClaudeConfig::generate_toml_section();
    assert!(toml.contains("[fix_with_claude_code]"));
    assert!(toml.contains("multiplexer"));
    assert!(toml.contains("prompt"));
    assert!(toml.contains("[fix_with_claude_code.permissions]"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p gh-pr-fix-with-claude -- test_generate_toml_section`
Expected: FAIL - generated TOML doesn't contain "multiplexer"

**Step 3: Write minimal implementation**

In `generate_toml_section()`, add after the `[fix_with_claude_code]\n` line (after line 125):

```rust
output.push_str("# Terminal multiplexer: \"zellij\" (default) or \"tmux\"\n");
output.push_str("multiplexer = \"zellij\"\n");
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p gh-pr-fix-with-claude -- test_generate_toml_section`
Expected: PASS

**Step 5: Commit**

```
feat(config): include multiplexer in generated TOML
```

---

### Task 3: Update session.rs - spawn with multiplexer dispatch

**Files:**
- Modify: `crates/gh-pr-fix-with-claude/src/session.rs:1-76`

**Step 1: Update spawn_claude_session**

The function already receives `config: &FixWithClaudeConfig` which now has `config.multiplexer`. Replace the tmux-hardcoded spawn logic with match dispatch.

Replace lines 22-72 (the kill + spawn body) with:

```rust
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
```

Also add the import at line 5:

```rust
use crate::config::{FixWithClaudeConfig, Multiplexer};
```

(replacing `use crate::config::FixWithClaudeConfig;`)

Update the module doc comment (line 1-3) to:

```rust
//! Session management for Claude Code
//!
//! Provides spawn, attach, kill, and liveness-check operations for tmux and zellij.
```

**Step 2: Run to verify it compiles**

Run: `cargo check -p gh-pr-fix-with-claude`
Expected: PASS (compiles)

**Step 3: Commit**

```
feat(session): dispatch spawn_claude_session by multiplexer
```

---

### Task 4: Update session.rs - is_session_alive, kill_session, attach_session

**Files:**
- Modify: `crates/gh-pr-fix-with-claude/src/session.rs:78-118`

**Step 1: Add Multiplexer param and match dispatch**

Replace `is_session_alive` (lines 78-85):

```rust
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
```

Replace `kill_session` (lines 87-93):

```rust
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
```

Replace `attach_session` (lines 95-118):

```rust
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
```

**Step 2: Run to verify it compiles**

Run: `cargo check -p gh-pr-fix-with-claude`
Expected: PASS

**Step 3: Commit**

```
feat(session): add multiplexer dispatch to is_session_alive, kill_session, attach_session
```

---

### Task 5: Update pty.rs - rename and add multiplexer dispatch

**Files:**
- Modify: `crates/gh-pr-fix-with-claude/src/pty.rs`

**Step 1: Replace entire pty.rs**

```rust
//! PTY spawn helper
//!
//! Opens a pseudo-terminal and spawns a multiplexer attach inside it.

use crate::config::Multiplexer;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::io::{Read, Write};

/// Handle to an open PTY with reader/writer/master
pub struct EmbeddedPty {
    pub reader: Box<dyn Read + Send>,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
}

/// Open a PTY and spawn a multiplexer attach inside it.
pub fn open_multiplexer_pty(
    multiplexer: &Multiplexer,
    session_name: &str,
    cols: u16,
    rows: u16,
) -> Result<EmbeddedPty, String> {
    let pty_system = native_pty_system();

    let size = PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system
        .openpty(size)
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let cmd = match multiplexer {
        Multiplexer::Tmux => {
            let mut cmd = CommandBuilder::new("tmux");
            cmd.args(["attach-session", "-t", session_name]);
            cmd
        }
        Multiplexer::Zellij => {
            let mut cmd = CommandBuilder::new("zellij");
            cmd.args(["attach", session_name]);
            cmd
        }
    };

    let mux_name = match multiplexer {
        Multiplexer::Tmux => "tmux",
        Multiplexer::Zellij => "zellij",
    };

    pair.slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn {mux_name}: {}", e))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take PTY writer: {}", e))?;

    Ok(EmbeddedPty {
        reader,
        writer,
        master: pair.master,
    })
}
```

**Step 2: Run to verify it compiles**

Run: `cargo check -p gh-pr-fix-with-claude`
Expected: PASS

**Step 3: Commit**

```
feat(pty): rename open_tmux_pty to open_multiplexer_pty with dispatch
```

---

### Task 6: Update lib.rs exports

**Files:**
- Modify: `crates/gh-pr-fix-with-claude/src/lib.rs`

**Step 1: Update re-exports**

Replace line 16:
```rust
pub use config::{FixWithClaudeConfig, Permissions, Tool};
```
with:
```rust
pub use config::{FixWithClaudeConfig, Multiplexer, Permissions, Tool};
```

Replace line 19:
```rust
pub use pty::{EmbeddedPty, open_tmux_pty};
```
with:
```rust
pub use pty::{EmbeddedPty, open_multiplexer_pty};
```

Update doc comment (line 4):
```rust
//! Sessions run in a configurable terminal multiplexer (tmux or zellij) for attach/detach support.
```

**Step 2: Run to verify it compiles**

Run: `cargo check -p gh-pr-fix-with-claude`
Expected: PASS

**Step 3: Commit**

```
refactor(lib): update exports for multiplexer abstraction
```

---

### Task 7: Update claude_session_middleware.rs

**Files:**
- Modify: `crates/gh-pr-lander/src/middleware/claude_session_middleware.rs`

**Step 1: Update import (line 12-14)**

Replace:
```rust
use gh_pr_fix_with_claude::{
    checkout_pr_branch, is_session_alive, kill_session, spawn_claude_session, CheckoutParams, PrId,
};
```
with:
```rust
use gh_pr_fix_with_claude::{
    checkout_pr_branch, is_session_alive, kill_session, spawn_claude_session, CheckoutParams,
    Multiplexer, PrId,
};
```

**Step 2: Update kill_session call (line 193)**

Replace:
```rust
kill_session(&session.screen_name);
```
with:
```rust
kill_session(&session.screen_name, &state.app_config.fix_with_claude_code.multiplexer);
```

**Step 3: Update is_session_alive call (line 213)**

Replace:
```rust
.filter(|(_, session)| !is_session_alive(&session.screen_name))
```
with:
```rust
.filter(|(_, session)| !is_session_alive(&session.screen_name, &state.app_config.fix_with_claude_code.multiplexer))
```

**Step 4: Run to verify it compiles**

Run: `cargo check -p gh-pr-lander`
Expected: PASS

**Step 5: Commit**

```
refactor(middleware): pass multiplexer to session lifecycle calls
```

---

### Task 8: Update claude_terminal_middleware.rs

**Files:**
- Modify: `crates/gh-pr-lander/src/middleware/claude_terminal_middleware.rs`

**Step 1: Update import (line 14)**

Replace:
```rust
use gh_pr_fix_with_claude::{key_event_to_bytes, open_tmux_pty, TerminalScreen};
```
with:
```rust
use gh_pr_fix_with_claude::{key_event_to_bytes, open_multiplexer_pty, TerminalScreen};
```

**Step 2: Update open call (line 55)**

Replace:
```rust
match open_tmux_pty(session_name, cols, rows) {
```
with:
```rust
match open_multiplexer_pty(&state.app_config.fix_with_claude_code.multiplexer, session_name, cols, rows) {
```

**Step 3: Update module doc (lines 1-7)**

Replace:
```rust
//! - Spawns tmux attach in a PTY on Open
```
with:
```rust
//! - Spawns multiplexer attach in a PTY on Open
```

**Step 4: Run to verify it compiles**

Run: `cargo check -p gh-pr-lander`
Expected: PASS

**Step 5: Commit**

```
refactor(middleware): use open_multiplexer_pty in terminal middleware
```

---

### Task 9: Update app_config.rs tests

**Files:**
- Modify: `crates/gh-pr-config/src/app_config.rs:244-298`

**Step 1: Add multiplexer config test**

Add to `mod tests`:

```rust
#[test]
fn test_fix_with_claude_multiplexer_from_toml() {
    let toml = r#"
ide_command = "zed"

[fix_with_claude_code]
multiplexer = "tmux"
prompt = "Fix PR #{pr_number}: {pr_title}"
    "#;
    let config: AppConfig = toml::from_str(toml).unwrap();
    assert_eq!(
        config.fix_with_claude_code.multiplexer,
        gh_pr_fix_with_claude::Multiplexer::Tmux
    );
}
```

**Step 2: Update test_generate_default_toml (line 291-298)**

Add assertion:
```rust
assert!(toml.contains("multiplexer"));
```

**Step 3: Run tests**

Run: `cargo test -p gh-pr-config`
Expected: All PASS

**Step 4: Commit**

```
test(config): add multiplexer config tests to app_config
```

---

### Task 10: Final verification

**Step 1: Format**

Run: `cargo fmt`

**Step 2: Lint**

Run: `cargo clippy`
Expected: No warnings

**Step 3: Full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 4: Build**

Run: `cargo build`
Expected: Clean build

**Step 5: Commit any formatting changes**

If `cargo fmt` changed anything:
```
chore: format code
```
