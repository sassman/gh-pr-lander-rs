//! Claude Code Session Middleware
//!
//! Handles spawning, attaching to, and cleaning up Claude Code sessions.

use crate::actions::{
    Action, ClaudeSessionAction, ClaudeTerminalAction, PullRequestAction, StatusBarAction,
};
use crate::dispatcher::Dispatcher;
use crate::domain_models::Pr;
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_pr_fix_with_claude::{
    checkout_pr_branch, is_session_alive, kill_session, spawn_claude_session, CheckoutParams, PrId,
};
use std::collections::HashSet;
use tokio::runtime::Runtime;

pub struct ClaudeSessionMiddleware {
    runtime: Runtime,
    /// Counter for throttled liveness checks (every N ticks)
    tick_counter: usize,
}

const LIVENESS_CHECK_INTERVAL: usize = 30; // ~4.5s at 150ms tick

impl ClaudeSessionMiddleware {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
            tick_counter: 0,
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
                let fix_config = state.app_config.fix_with_claude_code.clone();

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
                                &org, &repo_name, pr_number, &pr_title, &pr_dir, &fix_config,
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
                            dispatcher
                                .dispatch(Action::ClaudeSession(ClaudeSessionAction::Error(err)));
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
                    // Open embedded terminal panel attached to the session
                    dispatcher.dispatch(Action::ClaudeTerminal(ClaudeTerminalAction::Open {
                        session_name: session.screen_name.clone(),
                    }));
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

            // Cleanup sessions for PRs that were closed/merged
            Action::PullRequest(PullRequestAction::Loaded { repo, prs }) => {
                let active_pr_ids: HashSet<PrId> = prs.iter().map(PrId::from).collect();

                let repo_url_prefix = format!(
                    "https://{}/{}/{}/pull/",
                    repo.effective_host(),
                    repo.org,
                    repo.repo
                );

                let stale: Vec<PrId> = state
                    .claude_sessions
                    .sessions
                    .keys()
                    .filter(|pr_id| {
                        pr_id.as_str().starts_with(&repo_url_prefix)
                            && !active_pr_ids.contains(pr_id)
                    })
                    .cloned()
                    .collect();

                for pr_id in stale {
                    if let Some(session) = state.claude_sessions.get_session(&pr_id) {
                        kill_session(&session.screen_name);
                    }
                    dispatcher.dispatch(Action::ClaudeSession(ClaudeSessionAction::Completed {
                        pr_id,
                    }));
                }
                true // Pass through
            }

            // Periodic liveness check (throttled)
            Action::Global(crate::actions::GlobalAction::Tick) => {
                self.tick_counter += 1;
                if self.tick_counter >= LIVENESS_CHECK_INTERVAL
                    && !state.claude_sessions.sessions.is_empty()
                {
                    self.tick_counter = 0;
                    let dead: Vec<PrId> = state
                        .claude_sessions
                        .sessions
                        .iter()
                        .filter(|(_, session)| !is_session_alive(&session.screen_name))
                        .map(|(pr_id, _)| pr_id.clone())
                        .collect();

                    for pr_id in dead {
                        dispatcher.dispatch(Action::ClaudeSession(
                            ClaudeSessionAction::Completed { pr_id },
                        ));
                    }
                }
                true
            }

            // Pass through all other actions
            _ => true,
        }
    }
}
