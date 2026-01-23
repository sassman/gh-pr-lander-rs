//! Claude Code integration for gh-pr-lander
//!
//! Provides background Claude Code sessions that can analyze and fix PRs.
//! Sessions run in GNU screen for attach/detach support.

mod checkout;
mod pr_id;
mod session;
mod state;

pub use checkout::{checkout_pr_branch, CheckoutParams};
pub use pr_id::PrId;
pub use session::{attach_session, is_session_alive, kill_session, spawn_claude_session};
pub use state::{ClaudeSession, ClaudeSessionsState};
