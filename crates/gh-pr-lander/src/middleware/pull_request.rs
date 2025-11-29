//! Pull Request Middleware
//!
//! Handles side effects for loading Pull Requests from GitHub:
//! - Initializes octocrab client on BootstrapStart
//! - Triggers PR loading when repositories are added
//! - Makes octocrab API calls to fetch PRs
//! - Dispatches PrLoaded/PrLoadError actions with results

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::domain_models::{MergeableStatus, Pr};
use crate::middleware::Middleware;
use crate::state::AppState;
use octocrab::Octocrab;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Middleware for loading Pull Requests from GitHub
pub struct PullRequestMiddleware {
    /// Tokio runtime for async operations
    runtime: Runtime,
    /// GitHub API client (initialized on BootstrapStart)
    octocrab: Option<Arc<Octocrab>>,
}

impl PullRequestMiddleware {
    pub fn new() -> Self {
        let runtime = Runtime::new().expect("Failed to create tokio runtime");

        Self {
            runtime,
            octocrab: None, // Will be initialized on BootstrapStart
        }
    }

    /// Initialize the octocrab client
    fn initialize_octocrab(&mut self) {
        let result = self.runtime.block_on(async { init_octocrab().await });

        match result {
            Ok(client) => {
                log::info!("PullRequestMiddleware: GitHub client initialized");
                self.octocrab = Some(client);
            }
            Err(e) => {
                log::warn!(
                    "PullRequestMiddleware: GitHub client not initialized: {}",
                    e
                );
            }
        }
    }
}

impl Default for PullRequestMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for PullRequestMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Initialize octocrab on bootstrap
            Action::BootstrapStart => {
                self.initialize_octocrab();
                true // Let action pass through
            }

            // When repositories are added in bulk, start loading PRs for each
            Action::RepositoryAddBulk(repos) => {
                log::info!(
                    "RepositoryAddBulk received with {} repos, octocrab initialized: {}",
                    repos.len(),
                    self.octocrab.is_some()
                );

                if self.octocrab.is_none() {
                    log::warn!("Cannot load PRs: GitHub client not initialized");
                    return true;
                }

                // Calculate starting index (after existing repos)
                let start_idx = state.main_view.repositories.len();
                log::info!(
                    "Current repos in state: {}, will dispatch PrLoadStart for indices {}..{}",
                    start_idx,
                    start_idx,
                    start_idx + repos.len()
                );

                // Dispatch PrLoadStart for each new repository
                for (i, _repo) in repos.iter().enumerate() {
                    let repo_idx = start_idx + i;
                    dispatcher.dispatch(Action::PrLoadStart(repo_idx));
                }

                true // Let action pass through to reducer
            }

            // When a single repository is added via confirm
            Action::AddRepoConfirm => {
                if self.octocrab.is_none() {
                    log::warn!("Cannot load PRs: GitHub client not initialized");
                    return true;
                }

                if state.add_repo_form.is_valid() {
                    // The new repo will be at the end of the list
                    let repo_idx = state.main_view.repositories.len();
                    dispatcher.dispatch(Action::PrLoadStart(repo_idx));
                }

                true // Let action pass through to reducer
            }

            // Handle PR load start - actually fetch the PRs
            Action::PrLoadStart(repo_idx) => {
                log::info!(
                    "PrLoadStart({}) received, repos in state: {}",
                    repo_idx,
                    state.main_view.repositories.len()
                );

                let Some(octocrab) = &self.octocrab else {
                    log::error!("PrLoadStart: octocrab not initialized");
                    dispatcher.dispatch(Action::PrLoadError(
                        *repo_idx,
                        "GitHub client not initialized".to_string(),
                    ));
                    return true;
                };

                // Get the repository at this index
                // Note: For RepositoryAddBulk, the repos aren't in state yet when this runs,
                // so we need to handle this carefully. The reducer will process RepositoryAddBulk
                // before PrLoadStart, so by the time we get here, the repos should be there.
                let Some(repo) = state.main_view.repositories.get(*repo_idx) else {
                    log::warn!(
                        "PrLoadStart: Repository at index {} not found (state has {} repos), will retry",
                        repo_idx,
                        state.main_view.repositories.len()
                    );
                    // This might happen due to action ordering - the reducer might not have
                    // processed RepositoryAddBulk yet. We'll let the action pass through
                    // and the reducer can handle it.
                    return true;
                };

                log::info!(
                    "PrLoadStart: Found repo at index {}: {}/{}",
                    repo_idx,
                    repo.org,
                    repo.repo
                );

                let org = repo.org.clone();
                let repo_name = repo.repo.clone();
                let octocrab = octocrab.clone();
                let dispatcher = dispatcher.clone();
                let repo_idx = *repo_idx;

                // Spawn async task to load PRs
                log::info!("Spawning async task to load PRs for {}/{}", org, repo_name);
                self.runtime.spawn(async move {
                    log::info!("Async task started: Loading PRs for {}/{}", org, repo_name);

                    match load_prs(&octocrab, &org, &repo_name).await {
                        Ok(prs) => {
                            log::info!("Loaded {} PRs for {}/{}", prs.len(), org, repo_name);
                            dispatcher.dispatch(Action::PrLoaded(repo_idx, prs));
                        }
                        Err(e) => {
                            log::error!("Failed to load PRs for {}/{}: {}", org, repo_name, e);
                            dispatcher.dispatch(Action::PrLoadError(repo_idx, e.to_string()));
                        }
                    }
                });

                true // Let action pass through to reducer (to set loading state)
            }

            // Handle PR refresh request
            Action::PrRefresh => {
                if self.octocrab.is_none() {
                    log::warn!("Cannot refresh PRs: GitHub client not initialized");
                    return true;
                }

                let repo_idx = state.main_view.selected_repository;
                dispatcher.dispatch(Action::PrLoadStart(repo_idx));

                true
            }

            _ => true, // Pass through all other actions
        }
    }
}

/// Initialize octocrab client from environment or gh CLI
async fn init_octocrab() -> anyhow::Result<Arc<Octocrab>> {
    // Try environment variables first
    let token = std::env::var("GITHUB_TOKEN")
        .or_else(|_| std::env::var("GH_TOKEN"))
        .or_else(|_| {
            // Fallback: try to get token from gh CLI
            log::debug!("No GITHUB_TOKEN/GH_TOKEN found, trying gh auth token");
            std::process::Command::new("gh")
                .args(["auth", "token"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout)
                            .ok()
                            .map(|s| s.trim().to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| std::env::VarError::NotPresent)
        })
        .map_err(|_| {
            anyhow::anyhow!(
                "GitHub token not found. Set GITHUB_TOKEN, GH_TOKEN, or run 'gh auth login'"
            )
        })?;

    let octocrab = Octocrab::builder().personal_token(token).build()?;

    Ok(Arc::new(octocrab))
}

/// Load PRs for a repository
async fn load_prs(octocrab: &Octocrab, org: &str, repo: &str) -> anyhow::Result<Vec<Pr>> {
    let pulls = octocrab
        .pulls(org, repo)
        .list()
        .state(octocrab::params::State::Open)
        .per_page(50)
        .send()
        .await?;

    let prs: Vec<Pr> = pulls
        .items
        .into_iter()
        .map(|pr| {
            let mergeable = match pr.mergeable_state {
                Some(octocrab::models::pulls::MergeableState::Clean) => MergeableStatus::Ready,
                Some(octocrab::models::pulls::MergeableState::Behind) => {
                    MergeableStatus::NeedsRebase
                }
                Some(octocrab::models::pulls::MergeableState::Dirty) => MergeableStatus::Conflicted,
                Some(octocrab::models::pulls::MergeableState::Blocked) => MergeableStatus::Blocked,
                Some(octocrab::models::pulls::MergeableState::Unstable) => {
                    MergeableStatus::BuildFailed
                }
                _ => MergeableStatus::Unknown,
            };

            Pr {
                number: pr.number as usize,
                title: pr.title.clone().unwrap_or_default(),
                body: pr.body.clone().unwrap_or_default(),
                author: pr.user.map(|u| u.login).unwrap_or_default(),
                comments: pr.comments.unwrap_or_default() as usize,
                mergeable,
                needs_rebase: matches!(mergeable, MergeableStatus::NeedsRebase),
                head_sha: pr.head.sha.clone(),
                created_at: pr.created_at.unwrap_or_else(chrono::Utc::now),
                updated_at: pr.updated_at.unwrap_or_else(chrono::Utc::now),
            }
        })
        .collect();

    Ok(prs)
}
