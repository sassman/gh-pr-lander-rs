//! Claude Code integration for gh-pr-lander
//!
//! Provides background Claude Code sessions that can analyze and fix PRs.
//! Sessions run in GNU screen for attach/detach support.

mod checkout;
mod config;
pub mod key_translation;
mod pr_id;
pub mod pty;
mod session;
mod state;
pub mod terminal_screen;

pub use checkout::{CheckoutParams, checkout_pr_branch};
pub use config::{FixWithClaudeConfig, Permissions, Tool};
pub use key_translation::key_event_to_bytes;
pub use pr_id::PrId;
pub use pty::{EmbeddedPty, open_tmux_pty};
pub use session::{attach_session, is_session_alive, kill_session, spawn_claude_session};
pub use state::{ClaudeSession, ClaudeSessionsState};
pub use terminal_screen::{TerminalCell, TerminalColor, TerminalScreen};
