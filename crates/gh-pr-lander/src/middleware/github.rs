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

    /// Get current PR's info for CI operations
    fn get_current_pr_ci_info(&self, state: &AppState) -> Option<(usize, u64, String, String, String)> {
        let repo_idx = state.main_view.selected_repository;
        let repo = state.main_view.repositories.get(repo_idx)?;
        let repo_data = state.main_view.repo_data.get(&repo_idx)?;
        let pr = repo_data.prs.get(repo_data.selected_pr)?;

        Some((
            repo_idx,
            pr.number as u64,
            repo.org.clone(),
            repo.repo.clone(),
            pr.head_sha.clone(),
        ))
    }

    /// Build CI logs URL for current PR (GitHub Actions URL pattern)
    fn build_ci_logs_url(&self, state: &AppState) -> Option<String> {
        let repo_idx = state.main_view.selected_repository;
        let repo = state.main_view.repositories.get(repo_idx)?;
        let repo_data = state.main_view.repo_data.get(&repo_idx)?;
        let pr = repo_data.prs.get(repo_data.selected_pr)?;

        // GitHub Actions URL for a specific commit
        Some(format!(
            "https://github.com/{}/{}/actions?query=branch%3A{}",
            repo.org, repo.repo, pr.head_branch
        ))
    }
}

impl Middleware for GitHubMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::PrOpenInBrowser => {
                if let Some(url) = self.get_current_pr_url(state) {
                    log::info!("Opening PR in browser: {}", url);

                    // Use platform-specific commands (matching gh-pr-tui implementation)
                    self.runtime.spawn(async move {
                        #[cfg(target_os = "macos")]
                        let _ = tokio::process::Command::new("open").arg(&url).spawn();

                        #[cfg(target_os = "linux")]
                        let _ = tokio::process::Command::new("xdg-open").arg(&url).spawn();

                        #[cfg(target_os = "windows")]
                        let _ = tokio::process::Command::new("cmd")
                            .args(["/C", "start", &url])
                            .spawn();
                    });
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

            Action::PrOpenBuildLogs => {
                if let Some(url) = self.build_ci_logs_url(state) {
                    log::info!("Opening CI logs in browser: {}", url);

                    // Use platform-specific commands (matching gh-pr-tui implementation)
                    self.runtime.spawn(async move {
                        #[cfg(target_os = "macos")]
                        let _ = tokio::process::Command::new("open").arg(&url).spawn();

                        #[cfg(target_os = "linux")]
                        let _ = tokio::process::Command::new("xdg-open").arg(&url).spawn();

                        #[cfg(target_os = "windows")]
                        let _ = tokio::process::Command::new("cmd")
                            .args(["/C", "start", &url])
                            .spawn();
                    });
                }
                false // Consume action
            }

            Action::PrOpenInIDE => {
                // Get current PR info for IDE opening
                let repo_idx = state.main_view.selected_repository;
                if let Some(repo) = state.main_view.repositories.get(repo_idx).cloned() {
                    if let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) {
                        if let Some(pr) = repo_data.prs.get(repo_data.selected_pr).cloned() {
                            let pr_number = pr.number;

                            log::info!(
                                "Opening PR #{} in IDE for {}/{}",
                                pr_number,
                                repo.org,
                                repo.repo
                            );

                            // Spawn blocking task to open in IDE (matching gh-pr-tui implementation)
                            tokio::task::spawn_blocking(move || {
                                use std::path::PathBuf;
                                use std::process::Command;

                                // Use system temp directory
                                let temp_dir = std::env::temp_dir().join("gh-pr-lander");

                                // Create temp directory if it doesn't exist
                                if let Err(err) = std::fs::create_dir_all(&temp_dir) {
                                    log::error!("Failed to create temp directory: {}", err);
                                    return;
                                }

                                // Create unique directory for this PR
                                let dir_name = format!("{}-{}-pr-{}", repo.org, repo.repo, pr_number);
                                let pr_dir = PathBuf::from(&temp_dir).join(dir_name);

                                // Remove existing directory if present
                                if pr_dir.exists() {
                                    if let Err(err) = std::fs::remove_dir_all(&pr_dir) {
                                        log::error!("Failed to remove existing directory: {}", err);
                                        return;
                                    }
                                }

                                // Clone the repository using gh repo clone
                                log::info!("Cloning {}/{} to {:?}", repo.org, repo.repo, pr_dir);
                                let clone_output = Command::new("gh")
                                    .args([
                                        "repo",
                                        "clone",
                                        &format!("{}/{}", repo.org, repo.repo),
                                        &pr_dir.to_string_lossy(),
                                    ])
                                    .output();

                                match clone_output {
                                    Err(err) => {
                                        log::error!("Failed to run gh repo clone: {}", err);
                                        return;
                                    }
                                    Ok(output) if !output.status.success() => {
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        log::error!("gh repo clone failed: {}", stderr);
                                        return;
                                    }
                                    _ => {}
                                }

                                // Checkout the PR using gh pr checkout
                                log::info!("Checking out PR #{}", pr_number);
                                let checkout_output = Command::new("gh")
                                    .args(["pr", "checkout", &pr_number.to_string()])
                                    .current_dir(&pr_dir)
                                    .output();

                                match checkout_output {
                                    Err(err) => {
                                        log::error!("Failed to run gh pr checkout: {}", err);
                                        return;
                                    }
                                    Ok(output) if !output.status.success() => {
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        log::error!("gh pr checkout failed: {}", stderr);
                                        return;
                                    }
                                    _ => {}
                                }

                                // Set origin URL to SSH (gh checkout doesn't do this)
                                let ssh_url =
                                    format!("git@github.com:{}/{}.git", repo.org, repo.repo);
                                let set_url_output = Command::new("git")
                                    .args(["remote", "set-url", "origin", &ssh_url])
                                    .current_dir(&pr_dir)
                                    .output();

                                if let Err(err) = set_url_output {
                                    log::warn!("Failed to set SSH origin URL: {}", err);
                                    // Continue anyway - HTTPS will still work
                                }

                                // Open in IDE (try common IDE commands)
                                // Priority: code (VS Code), cursor, zed, idea, vim
                                let ide_commands = ["code", "cursor", "zed", "idea", "vim"];
                                let mut opened = false;

                                for ide in ide_commands {
                                    if Command::new(ide).arg(&pr_dir).spawn().is_ok() {
                                        log::info!("Opened PR #{} in {} at {:?}", pr_number, ide, pr_dir);
                                        opened = true;
                                        break;
                                    }
                                }

                                if !opened {
                                    log::error!(
                                        "Failed to open IDE. Tried: {:?}. PR cloned at: {:?}",
                                        ide_commands,
                                        pr_dir
                                    );
                                }
                            });
                        }
                    }
                }
                false // Consume action
            }

            Action::PrRerunFailedJobs => {
                let client = match &self.client {
                    Some(c) => c.clone(),
                    None => {
                        log::error!("GitHub client not available");
                        return false;
                    }
                };

                // Get current PR's CI info
                let (repo_idx, pr_number, owner, repo, head_sha) =
                    match self.get_current_pr_ci_info(state) {
                        Some(info) => info,
                        None => {
                            log::warn!("No PR selected for rerunning jobs");
                            return false;
                        }
                    };

                let dispatcher = dispatcher.clone();
                let client = client.clone();

                // First fetch workflow runs, then rerun failed ones
                self.runtime.spawn(async move {
                    // Fetch workflow runs for this commit
                    match client.fetch_workflow_runs(&owner, &repo, &head_sha).await {
                        Ok(runs) => {
                            // Filter to failed runs and rerun each
                            let failed_runs: Vec<_> = runs
                                .into_iter()
                                .filter(|r| {
                                    r.conclusion.as_ref().map_or(false, |c| {
                                        matches!(
                                            c,
                                            gh_client::WorkflowRunConclusion::Failure
                                                | gh_client::WorkflowRunConclusion::TimedOut
                                        )
                                    })
                                })
                                .collect();

                            if failed_runs.is_empty() {
                                log::info!("No failed workflow runs to rerun for PR #{}", pr_number);
                                return;
                            }

                            for run in failed_runs {
                                dispatcher.dispatch(Action::PrRerunStart(repo_idx, pr_number, run.id));

                                match client.rerun_failed_jobs(&owner, &repo, run.id).await {
                                    Ok(()) => {
                                        log::info!(
                                            "Successfully triggered rerun for workflow {} (PR #{})",
                                            run.name,
                                            pr_number
                                        );
                                        dispatcher.dispatch(Action::PrRerunSuccess(
                                            repo_idx, pr_number, run.id,
                                        ));
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Failed to rerun workflow {} (PR #{}): {}",
                                            run.name,
                                            pr_number,
                                            e
                                        );
                                        dispatcher.dispatch(Action::PrRerunError(
                                            repo_idx,
                                            pr_number,
                                            run.id,
                                            e.to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to fetch workflow runs: {}", e);
                        }
                    }
                });
                false // Consume action
            }

            _ => true, // Pass through other actions
        }
    }
}
