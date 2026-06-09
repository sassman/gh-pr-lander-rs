# Configurable Terminal Multiplexer for Fix-with-Claude

**Date:** 2026-02-18
**Status:** Approved

## Problem

The "Fix with Claude Code" feature hardcodes tmux for session management and PTY embedding. Users who prefer zellij (which now supports attach/detach per [zellij#69](https://github.com/zellij-org/zellij/issues/69)) cannot use the feature without tmux installed.

## Decision

Add a `multiplexer` config option under `[fix_with_claude_code]` supporting `tmux` and `zellij`, with zellij as the default.

## Config

```toml
[fix_with_claude_code]
multiplexer = "zellij"  # or "tmux"
```

## Data Model

New enum in `crates/gh-pr-fix-with-claude/src/config.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Multiplexer {
    Tmux,
    #[default]
    Zellij,
}
```

Added as a field to `FixWithClaudeConfig`.

## Approach: Enum + Match Dispatch

Each multiplexer operation matches on the `Multiplexer` enum to select the right shell commands. No traits — everything in one place.

### Command Mapping

| Operation | tmux | zellij |
|-----------|------|--------|
| Spawn detached | `tmux new-session -d -s <name> <cmd>` | `zellij -s <name> -- <cmd>` |
| Check alive | `tmux has-session -t <name>` | `zellij list-sessions` + grep |
| Kill | `tmux kill-session -t <name>` | `zellij kill-session <name>` |
| Attach (PTY) | `tmux attach-session -t <name>` | `zellij attach <name>` |

## Files Changed

1. **`config.rs`** — Add `Multiplexer` enum and field
2. **`session.rs`** — Match on multiplexer in `spawn_claude_session`, `is_session_alive`, `kill_session`
3. **`pty.rs`** — Rename `open_tmux_pty` to `open_multiplexer_pty`, match on multiplexer
4. **`lib.rs`** — Update re-exports (`Multiplexer`, renamed PTY function)
5. **`claude_session_middleware.rs`** — Pass `&Multiplexer` to `is_session_alive`/`kill_session`
6. **`claude_terminal_middleware.rs`** — Pass `&Multiplexer` to `open_multiplexer_pty`
7. **`app_config.rs`** — TOML generation includes `multiplexer` key

## Session Name Format

Unchanged: `gh-pr-lander-fix-with-claude--{org}-{repo}-pr-{number}` (compatible with both tmux and zellij).
