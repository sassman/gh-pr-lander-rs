//! GitHub Operations Middleware
//!
//! Handles PR operations that require GitHub API calls:
//! - Open in browser
//! - Merge PR
//! - Rebase/update PR branch
//! - Approve PR
//! - Close PR

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_client::{CachedGitHubClient, GitHubClient, MergeMethod, OctocrabClient, ReviewEvent};

/// Middleware for GitHub PR operations
pub struct GitHubMiddleware {
    /// GitHub client for API calls (using CacheMode::None since these are mutations)
    client: Option<CachedGitHubClient<OctocrabClient>>,
    /// Tokio runtime for async operations
    runtime: tokio::runtime::Handle,
}

impl GitHubMiddleware {
    /// Create a new GitHub middleware
    pub fn new(
        client: Option<CachedGitHubClient<OctocrabClient>>,
        runtime: tokio::runtime::Handle,
    ) -> Self {
        Self { client, runtime }
    }

    /// Get target PRs for an operation (selected PRs or cursor PR)
    fn get_target_prs(&self, state: &AppState) -> Vec<(usize, usize)> {
        let repo_idx = state.main_view.selected_repository;

        if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
            // If there are selected PRs, use those
            if !repo_data.selected_pr_numbers.is_empty() {
                return repo_data
                    .selected_pr_numbers
                    .iter()
                    .map(|&pr_num| (repo_idx, pr_num))
                    .collect();
            }

            // Otherwise use the cursor PR
            if let Some(pr) = repo_data.prs.get(repo_data.selected_pr) {
                return vec![(repo_idx, pr.number)];
            }
        }

        vec![]
    }

    /// Get current PR's HTML URL for opening in browser
    fn get_current_pr_url(&self, state: &AppState) -> Option<String> {
        let repo_idx = state.main_view.selected_repository;
        let repo_data = state.main_view.repo_data.get(&repo_idx)?;
        let pr = repo_data.prs.get(repo_data.selected_pr)?;
        Some(pr.html_url.clone())
    }

    /// Get repository info for a PR operation
    fn get_repo_info(&self, state: &AppState, repo_idx: usize) -> Option<(String, String)> {
        state
            .main_view
            .repositories
            .get(repo_idx)
            .map(|r| (r.org.clone(), r.repo.clone()))
    }
}

impl Middleware for GitHubMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::PrOpenInBrowser => {
                if let Some(url) = self.get_current_pr_url(state) {
                    log::info!("Opening PR in browser: {}", url);
                    if let Err(e) = open::that(&url) {
                        log::error!("Failed to open URL in browser: {}", e);
                    }
                }
                false // Consume action
            }

            Action::PrMergeRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for merge");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrMergeStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .merge_pull_request(
                                    &owner,
                                    &repo,
                                    pr_number as u64,
                                    MergeMethod::default(),
                                    None,
                                    None,
                                )
                                .await
                            {
                                Ok(result) if result.merged => {
                                    log::info!("Successfully merged PR #{}", pr_number);
                                    dispatcher.dispatch(Action::PrMergeSuccess(repo_idx, pr_number));
                                    // Trigger refresh to update PR list
                                    dispatcher.dispatch(Action::PrRefresh);
                                }
                                Ok(result) => {
                                    log::error!("Merge failed: {}", result.message);
                                    dispatcher.dispatch(Action::PrMergeError(
                                        repo_idx,
                                        pr_number,
                                        result.message,
                                    ));
                                }
                                Err(e) => {
                                    log::error!("Merge error: {}", e);
                                    dispatcher.dispatch(Action::PrMergeError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrRebaseRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for rebase");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrRebaseStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .update_pull_request_branch(&owner, &repo, pr_number as u64)
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully rebased PR #{}", pr_number);
                                    dispatcher.dispatch(Action::PrRebaseSuccess(repo_idx, pr_number));
                                    // Trigger refresh to update PR status
                                    dispatcher.dispatch(Action::PrRefresh);
                                }
                                Err(e) => {
                                    log::error!("Rebase error: {}", e);
                                    dispatcher.dispatch(Action::PrRebaseError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrApproveRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for approval");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrApproveStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .create_review(
                                    &owner,
                                    &repo,
                                    pr_number as u64,
                                    ReviewEvent::Approve,
                                    None,
                                )
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully approved PR #{}", pr_number);
                                    dispatcher
                                        .dispatch(Action::PrApproveSuccess(repo_idx, pr_number));
                                }
                                Err(e) => {
                                    log::error!("Approve error: {}", e);
                                    dispatcher.dispatch(Action::PrApproveError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            Action::PrCloseRequest => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                let targets = self.get_target_prs(state);
                if targets.is_empty() {
                    log::warn!("No PRs selected for closing");
                    return false;
                }

                for (repo_idx, pr_number) in targets {
                    if let Some((owner, repo)) = self.get_repo_info(state, repo_idx) {
                        let dispatcher = dispatcher.clone();
                        let client = client.clone();

                        dispatcher.dispatch(Action::PrCloseStart(repo_idx, pr_number));

                        self.runtime.spawn(async move {
                            match client
                                .close_pull_request(&owner, &repo, pr_number as u64)
                                .await
                            {
                                Ok(()) => {
                                    log::info!("Successfully closed PR #{}", pr_number);
                                    dispatcher.dispatch(Action::PrCloseSuccess(repo_idx, pr_number));
                                    // Trigger refresh to update PR list
                                    dispatcher.dispatch(Action::PrRefresh);
                                }
                                Err(e) => {
                                    log::error!("Close error: {}", e);
                                    dispatcher.dispatch(Action::PrCloseError(
                                        repo_idx,
                                        pr_number,
                                        e.to_string(),
                                    ));
                                }
                            }
                        });
                    }
                }
                false // Consume action
            }

            _ => true, // Pass through other actions
        }
    }
}
