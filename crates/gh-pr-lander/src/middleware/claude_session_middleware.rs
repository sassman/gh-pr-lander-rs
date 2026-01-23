//! Claude Code Session Middleware
//!
//! Handles spawning, attaching to, and cleaning up Claude Code sessions.

use crate::actions::{Action, ClaudeSessionAction, StatusBarAction};
use crate::dispatcher::Dispatcher;
use crate::domain_models::Pr;
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_pr_fix_with_claude::{checkout_pr_branch, spawn_claude_session, CheckoutParams, PrId};
use tokio::runtime::Runtime;

pub struct ClaudeSessionMiddleware {
    runtime: Runtime,
}

impl ClaudeSessionMiddleware {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Get the cursor PR for the selected repo
    fn get_cursor_pr(state: &AppState) -> Option<&Pr> {
        let repo_idx = state.main_view.selected_repository;
        let repo_data = state.main_view.repo_data.get(&repo_idx)?;
        repo_data.prs.get(repo_data.selected_pr)
    }
}

impl Middleware for ClaudeSessionMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::ClaudeSession(ClaudeSessionAction::Start) => {
                let repo_idx = state.main_view.selected_repository;
                let repo = match state.main_view.repositories.get(repo_idx) {
                    Some(r) => r.clone(),
                    None => return false,
                };

                let pr = match Self::get_cursor_pr(state) {
                    Some(pr) => pr.clone(),
                    None => return false,
                };

                let pr_id: PrId = PrId::from(&pr);

                // Check if session already exists
                if state.claude_sessions.has_session(&pr_id) {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::info(
                        format!("Session already running for PR #{}", pr.number),
                        "claude",
                    )));
                    return false;
                }

                dispatcher.dispatch(Action::StatusBar(StatusBarAction::running(
                    format!("Starting Claude session for PR #{}...", pr.number),
                    "claude",
                )));

                let dispatcher = dispatcher.clone();
                let temp_dir = state.app_config.temp_dir.clone();
                let ssh_url = repo.ssh_url();
                let org = repo.org.clone();
                let repo_name = repo.repo.clone();
                let host = repo.host.clone();
                let pr_number = pr.number;
                let pr_title = pr.title.clone();

                self.runtime.spawn_blocking(move || {
                    let params = CheckoutParams {
                        org: org.clone(),
                        repo: repo_name.clone(),
                        pr_number,
                        ssh_url,
                        host,
                        temp_dir,
                    };

                    match checkout_pr_branch(&params) {
                        Ok(pr_dir) => {
                            match spawn_claude_session(
                                &org,
                                &repo_name,
                                pr_number,
                                &pr_title,
                                &pr_dir,
                            ) {
                                Ok(screen_name) => {
                                    dispatcher.dispatch(Action::ClaudeSession(
                                        ClaudeSessionAction::Started {
                                            pr_id,
                                            screen_name: screen_name.clone(),
                                            work_dir: pr_dir.to_string_lossy().to_string(),
                                        },
                                    ));
                                    dispatcher.dispatch(Action::StatusBar(
                                        StatusBarAction::success(
                                            format!(
                                                "Claude session started for PR #{} ({})",
                                                pr_number, screen_name
                                            ),
                                            "claude",
                                        ),
                                    ));
                                }
                                Err(err) => {
                                    dispatcher.dispatch(Action::ClaudeSession(
                                        ClaudeSessionAction::Error(err),
                                    ));
                                }
                            }
                        }
                        Err(err) => {
                            dispatcher.dispatch(Action::ClaudeSession(
                                ClaudeSessionAction::Error(err),
                            ));
                        }
                    }
                });

                false // Consume action
            }

            Action::ClaudeSession(ClaudeSessionAction::Attach) => {
                let pr = match Self::get_cursor_pr(state) {
                    Some(pr) => pr,
                    None => return false,
                };

                let pr_id: PrId = PrId::from(pr);

                if let Some(session) = state.claude_sessions.get_session(&pr_id) {
                    // Dispatch suspend action — main loop will handle terminal hand-off
                    dispatcher.dispatch(Action::ClaudeSession(
                        ClaudeSessionAction::SuspendForAttach {
                            screen_name: session.screen_name.clone(),
                        },
                    ));
                } else {
                    dispatcher.dispatch(Action::StatusBar(StatusBarAction::warning(
                        format!("No active session for PR #{}", pr.number),
                        "claude",
                    )));
                }
                false
            }

            Action::ClaudeSession(ClaudeSessionAction::Error(msg)) => {
                log::error!("Claude session error: {}", msg);
                dispatcher.dispatch(Action::StatusBar(StatusBarAction::error(
                    format!("Claude: {}", msg),
                    "claude",
                )));
                false
            }

            // Pass through all other actions
            _ => true,
        }
    }
}
