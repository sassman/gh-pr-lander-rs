//! Middleware system for Redux architecture
//!
//! Middleware sits between action dispatch and reducer execution, allowing
//! side effects, async operations, logging, and other cross-cutting concerns
//! to be handled in a composable way.
//!
//! ## Design
//!
//! ```text
//! Action → Middleware Chain → Reducer → State
//! ```
//!
//! Each middleware can:
//! - Inspect actions and state
//! - Dispatch new actions
//! - Perform side effects (async operations, logging, etc.)
//! - Block actions from reaching the reducer
//!
//! ## Example
//!
//! ```rust
//! struct LoggingMiddleware;
//!
//! impl Middleware for LoggingMiddleware {
//!     fn handle(
//!         &mut self,
//!         action: &Action,
//!         _state: &AppState,
//!         _dispatcher: &Dispatcher,
//!     ) -> BoxFuture<'_, bool> {
//!         Box::pin(async move {
//!             log::debug!("Action: {:?}", action);
//!             true // Continue to next middleware
//!         })
//!     }
//! }
//! ```

use crate::{actions::Action, state::AppState};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::mpsc;

/// BoxFuture type alias for async middleware handlers
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Middleware trait - handles actions before they reach the reducer
///
/// Middleware is called in order for each action. Each middleware can:
/// - Inspect the action and current state
/// - Dispatch new actions via the Dispatcher
/// - Perform async operations
/// - Block the action from continuing (return false)
///
/// # Example
///
/// ```rust
/// struct MyMiddleware;
///
/// impl Middleware for MyMiddleware {
///     fn handle<'a>(
///         &'a mut self,
///         action: &'a Action,
///         state: &'a AppState,
///         dispatcher: &'a Dispatcher,
///     ) -> BoxFuture<'a, bool> {
///         Box::pin(async move {
///             match action {
///                 Action::SomeAction => {
///                     // Perform side effect
///                     do_something().await;
///                     // Dispatch follow-up action
///                     dispatcher.dispatch(Action::SomeOtherAction);
///                     // Let action continue to reducer
///                     true
///                 }
///                 _ => true, // Pass through other actions
///             }
///         })
///     }
/// }
/// ```
pub trait Middleware: Send + Sync {
    /// Handle an action before it reaches the reducer
    ///
    /// # Parameters
    /// - `action`: The action being dispatched
    /// - `state`: Current application state (read-only)
    /// - `dispatcher`: Can dispatch new actions
    ///
    /// # Returns
    /// - `true`: Continue to next middleware/reducer
    /// - `false`: Block this action from continuing
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool>;
}

/// Dispatcher allows middleware to dispatch new actions
///
/// Actions dispatched through the Dispatcher will be processed
/// in the next event loop iteration, preventing recursion.
#[derive(Clone)]
pub struct Dispatcher {
    tx: mpsc::UnboundedSender<Action>,
}

impl Dispatcher {
    /// Create a new dispatcher
    pub fn new(tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { tx }
    }

    /// Dispatch an action
    ///
    /// The action will be queued and processed in the next iteration
    /// of the event loop.
    pub fn dispatch(&self, action: Action) {
        if let Err(e) = self.tx.send(action) {
            log::error!("Failed to dispatch action: {}", e);
        }
    }

    /// Dispatch an action from an async context
    ///
    /// This is useful when spawning tokio tasks that need to dispatch
    /// actions back to the store.
    pub fn dispatch_async(self, action: Action) {
        tokio::spawn(async move {
            self.dispatch(action);
        });
    }
}

/// LoggingMiddleware - logs all actions that pass through the system
///
/// This is a simple example middleware that demonstrates the pattern.
/// It logs every action for debugging purposes.
pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggingMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        _state: &'a AppState,
        _dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            // Log the action (skip None to reduce noise)
            if !matches!(action, Action::None) {
                log::debug!("Action: {:?}", action);
            }
            // Always continue to next middleware
            true
        })
    }
}

/// TaskMiddleware - handles async operations like loading repos, merging PRs, etc.
///
/// This middleware replaces the old Effect/BackgroundTask system by handling
/// async operations directly in response to actions.
///
/// # Example Operations
/// - Bootstrap → load .env, init Octocrab, load repos → dispatch BootstrapComplete
/// - RefreshCurrentRepo → fetch PR data from GitHub → dispatch RepoDataLoaded
/// - MergeSelectedPrs → call GitHub API → dispatch MergeComplete
/// - Rebase → call GitHub API → dispatch RebaseComplete
///
/// # Design
/// The middleware spawns tokio tasks for async operations and dispatches
/// result actions when complete. This eliminates the need for:
/// - Effect enum
/// - BackgroundTask enum
/// - TaskResult enum
/// - result_to_action() conversion
pub struct TaskMiddleware {
    /// GitHub client (set after initialization)
    octocrab: Option<octocrab::Octocrab>,
    /// API response cache
    cache: std::sync::Arc<std::sync::Mutex<gh_api_cache::ApiCache>>,
    /// Task channel for background operations (legacy - used during migration)
    task_tx: tokio::sync::mpsc::UnboundedSender<crate::task::BackgroundTask>,
}

impl TaskMiddleware {
    pub fn new(
        cache: std::sync::Arc<std::sync::Mutex<gh_api_cache::ApiCache>>,
        task_tx: tokio::sync::mpsc::UnboundedSender<crate::task::BackgroundTask>,
    ) -> Self {
        Self {
            octocrab: None,
            cache,
            task_tx,
        }
    }

    /// Get octocrab client (returns error if not initialized)
    fn octocrab(&self) -> Result<octocrab::Octocrab, String> {
        self.octocrab
            .clone()
            .ok_or_else(|| "Octocrab not initialized".to_string())
    }
}

impl Middleware for TaskMiddleware {
    fn handle<'a>(
        &'a mut self,
        action: &'a Action,
        state: &'a AppState,
        dispatcher: &'a Dispatcher,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            use crate::actions::{Action, BootstrapResult};
            use crate::state::{TaskStatus, TaskStatusType};
            use crate::task::BackgroundTask;

            match action {
                //
                // BOOTSTRAP FLOW
                //

                Action::Bootstrap => {
                    log::debug!("TaskMiddleware: Handling Bootstrap");

                    // Step 1: Load .env file if GITHUB_TOKEN not set
                    if std::env::var("GITHUB_TOKEN").is_err() {
                        match dotenvy::dotenv() {
                            Ok(path) => {
                                log::debug!("Loaded .env file from: {:?}", path);
                            }
                            Err(_) => {
                                log::debug!(".env file not found, will rely on environment variables");
                            }
                        }
                    }

                    // Step 2: Initialize Octocrab
                    match std::env::var("GITHUB_TOKEN") {
                        Ok(token) => match octocrab::Octocrab::builder()
                            .personal_token(token)
                            .build()
                        {
                            Ok(client) => {
                                log::debug!("Octocrab client initialized successfully");
                                dispatcher.dispatch(Action::OctocrabInitialized(client));
                            }
                            Err(e) => {
                                log::error!("Failed to initialize octocrab: {}", e);
                                dispatcher.dispatch(Action::BootstrapComplete(Err(format!(
                                    "Failed to initialize GitHub client: {}",
                                    e
                                ))));
                                return true; // Stop bootstrap flow
                            }
                        },
                        Err(_) => {
                            dispatcher.dispatch(Action::BootstrapComplete(Err(
                                "GITHUB_TOKEN environment variable not set. Please set it or create a .env file.".to_string()
                            )));
                            return true; // Stop bootstrap flow
                        }
                    }
                }

                Action::OctocrabInitialized(client) => {
                    log::debug!("TaskMiddleware: Storing Octocrab client");
                    // Store the client for future use
                    self.octocrab = Some(client.clone());

                    // Step 3: Load repositories from config
                    match crate::loading_recent_repos() {
                        Ok(repos) => {
                            if repos.is_empty() {
                                dispatcher.dispatch(Action::BootstrapComplete(Err(
                                    "No repositories configured. Add repositories to .recent-repositories.json".to_string()
                                )));
                                return true;
                            }

                            // Restore session
                            let selected_repo: usize =
                                if let Ok(persisted_state) = crate::load_persisted_state() {
                                    repos
                                        .iter()
                                        .position(|r| r == &persisted_state.selected_repo)
                                        .unwrap_or_default()
                                } else {
                                    0
                                };

                            // Dispatch bootstrap complete
                            let result = BootstrapResult {
                                repos,
                                selected_repo,
                            };
                            dispatcher.dispatch(Action::BootstrapComplete(Ok(result)));
                        }
                        Err(err) => {
                            dispatcher.dispatch(Action::BootstrapComplete(Err(err.to_string())));
                        }
                    }
                }

                //
                // REPO LOADING OPERATIONS
                //

                Action::RefreshCurrentRepo => {
                    log::debug!("TaskMiddleware: Handling RefreshCurrentRepo");

                    // Get current repo info
                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        let filter = state.repos.filter.clone();

                        // Dispatch loading status
                        dispatcher.dispatch(Action::SetReposLoading(vec![repo_index]));
                        dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                            message: "Refreshing...".to_string(),
                            status_type: TaskStatusType::Running,
                        })));

                        // Trigger background task (using legacy system for now)
                        if let Ok(octocrab) = self.octocrab() {
                            let _ = self.task_tx.send(BackgroundTask::LoadSingleRepo {
                                repo_index,
                                repo,
                                filter,
                                octocrab,
                                cache: self.cache.clone(),
                                bypass_cache: true, // Refresh always bypasses cache
                            });
                        }
                    }
                }

                Action::ReloadRepo(repo_index) => {
                    log::debug!("TaskMiddleware: Handling ReloadRepo {}", repo_index);

                    if let Some(repo) = state.repos.recent_repos.get(*repo_index).cloned() {
                        let filter = state.repos.filter.clone();

                        // Dispatch loading status
                        dispatcher.dispatch(Action::SetReposLoading(vec![*repo_index]));

                        // Trigger background task
                        if let Ok(octocrab) = self.octocrab() {
                            let _ = self.task_tx.send(BackgroundTask::LoadSingleRepo {
                                repo_index: *repo_index,
                                repo,
                                filter,
                                octocrab,
                                cache: self.cache.clone(),
                                bypass_cache: false, // Normal reload uses cache
                            });
                        }
                    }
                }

                //
                // SIMPLE OPERATIONS
                //

                Action::OpenCurrentPrInBrowser => {
                    log::debug!("TaskMiddleware: Handling OpenCurrentPrInBrowser");

                    // Get current repo and selected PRs
                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index) {
                        // Check if there are selected PRs
                        let has_selection = if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            !repo_data.selected_pr_numbers.is_empty()
                        } else {
                            false
                        };

                        // Get PR numbers to open
                        let prs_to_open: Vec<usize> = if has_selection {
                            if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                                repo_data
                                    .prs
                                    .iter()
                                    .filter(|pr| {
                                        repo_data.selected_pr_numbers.contains(&crate::state::PrNumber::from_pr(pr))
                                    })
                                    .map(|pr| pr.number)
                                    .collect()
                            } else {
                                Vec::new()
                            }
                        } else if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            // No selection, open just the currently focused PR
                            if let Some(selected_idx) = repo_data.table_state.selected() {
                                repo_data
                                    .prs
                                    .get(selected_idx)
                                    .map(|pr| vec![pr.number])
                                    .unwrap_or_default()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        };

                        // Open each PR in browser
                        for pr_number in prs_to_open {
                            let url = format!(
                                "https://github.com/{}/{}/pull/{}",
                                repo.org, repo.repo, pr_number
                            );
                            log::debug!("Opening in browser: {}", url);

                            // Spawn async task to open URL (platform-specific)
                            tokio::spawn(async move {
                                #[cfg(target_os = "macos")]
                                let _ = tokio::process::Command::new("open")
                                    .arg(&url)
                                    .spawn();

                                #[cfg(target_os = "linux")]
                                let _ = tokio::process::Command::new("xdg-open")
                                    .arg(&url)
                                    .spawn();

                                #[cfg(target_os = "windows")]
                                let _ = tokio::process::Command::new("cmd")
                                    .args(["/C", "start", &url])
                                    .spawn();
                            });
                        }
                    }
                }

                Action::OpenInIDE => {
                    log::debug!("TaskMiddleware: Handling OpenInIDE");

                    // Get current repo and selected PR
                    let repo_index = state.repos.selected_repo;
                    if let Some(repo) = state.repos.recent_repos.get(repo_index).cloned() {
                        let config = state.config.clone();

                        if let Some(repo_data) = state.repos.repo_data.get(&repo_index) {
                            // Check if a PR is selected
                            let pr_number = if let Some(selected_idx) = repo_data.table_state.selected() {
                                repo_data.prs.get(selected_idx).map(|pr| pr.number).unwrap_or(0)
                            } else {
                                // No PR selected (empty list) - open main branch
                                0
                            };

                            // Set status message
                            let message = if pr_number == 0 {
                                "Opening main branch in IDE...".to_string()
                            } else {
                                format!("Opening PR #{} in IDE...", pr_number)
                            };
                            dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                message,
                                status_type: TaskStatusType::Running,
                            })));

                            // Send background task to open in IDE
                            let _ = self.task_tx.send(BackgroundTask::OpenPRInIDE {
                                repo,
                                pr_number,
                                ide_command: config.ide_command,
                                temp_dir: config.temp_dir,
                            });
                        }
                    }
                }

                Action::AddRepoFormSubmit => {
                    log::debug!("TaskMiddleware: Handling AddRepoFormSubmit");

                    // Build the new repo from form data
                    let branch = if state.ui.add_repo_form.branch.is_empty() {
                        "main".to_string()
                    } else {
                        state.ui.add_repo_form.branch.clone()
                    };

                    let new_repo = crate::state::Repo {
                        org: state.ui.add_repo_form.org.clone(),
                        repo: state.ui.add_repo_form.repo.clone(),
                        branch,
                    };

                    // Check if repository already exists
                    let repo_exists = state
                        .repos
                        .recent_repos
                        .iter()
                        .any(|r| {
                            r.org == new_repo.org
                                && r.repo == new_repo.repo
                                && r.branch == new_repo.branch
                        });

                    if !repo_exists {
                        // Calculate new repo index
                        let repo_index = state.repos.recent_repos.len();

                        // Build new repos list for saving
                        let mut new_repos = state.repos.recent_repos.clone();
                        new_repos.push(new_repo.clone());

                        // Save to file asynchronously
                        let dispatcher = dispatcher.clone();
                        let new_repo_for_action = new_repo.clone();
                        tokio::spawn(async move {
                            match crate::store_recent_repos(&new_repos) {
                                Ok(_) => {
                                    // Dispatch success actions
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: format!(
                                            "Repository {}/{} added",
                                            new_repo.org, new_repo.repo
                                        ),
                                        status_type: TaskStatusType::Success,
                                    })));
                                    dispatcher.dispatch(Action::RepositoryAdded {
                                        repo_index,
                                        repo: new_repo_for_action.clone(),
                                    });
                                    dispatcher.dispatch(Action::SelectRepoByIndex(repo_index));
                                    dispatcher.dispatch(Action::ReloadRepo(repo_index));
                                }
                                Err(e) => {
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: format!("Failed to save repository: {}", e),
                                        status_type: TaskStatusType::Error,
                                    })));
                                }
                            }
                        });
                    } else {
                        // Repository already exists
                        dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                            message: format!(
                                "Repository {}/{} already exists",
                                new_repo.org, new_repo.repo
                            ),
                            status_type: TaskStatusType::Error,
                        })));
                    }
                }

                Action::DeleteCurrentRepo => {
                    log::debug!("TaskMiddleware: Handling DeleteCurrentRepo");

                    // Build updated repos list without current repo
                    let repo_index = state.repos.selected_repo;
                    let mut new_repos = state.repos.recent_repos.clone();

                    if repo_index < new_repos.len() {
                        new_repos.remove(repo_index);

                        // Save to file asynchronously
                        let dispatcher = dispatcher.clone();
                        tokio::spawn(async move {
                            match crate::store_recent_repos(&new_repos) {
                                Ok(_) => {
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: "Repository deleted".to_string(),
                                        status_type: TaskStatusType::Success,
                                    })));
                                }
                                Err(e) => {
                                    dispatcher.dispatch(Action::SetTaskStatus(Some(TaskStatus {
                                        message: format!("Failed to save repositories: {}", e),
                                        status_type: TaskStatusType::Error,
                                    })));
                                }
                            }
                        });
                    }
                }

                // All other actions pass through unchanged
                _ => {}
            }

            // Always continue to next middleware/reducer
            true
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMiddleware {
        called: bool,
    }

    impl Middleware for TestMiddleware {
        fn handle<'a>(
            &'a mut self,
            _action: &'a Action,
            _state: &'a AppState,
            _dispatcher: &'a Dispatcher,
        ) -> BoxFuture<'a, bool> {
            Box::pin(async move {
                self.called = true;
                true
            })
        }
    }

    #[tokio::test]
    async fn test_middleware_trait() {
        let mut middleware = TestMiddleware { called: false };
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);
        let state = AppState::default();

        let should_continue = middleware
            .handle(&Action::None, &state, &dispatcher)
            .await;

        assert!(should_continue);
        assert!(middleware.called);
    }

    #[test]
    fn test_dispatcher() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);

        dispatcher.dispatch(Action::None);

        let received = rx.try_recv();
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_logging_middleware() {
        let mut middleware = LoggingMiddleware;
        let (tx, _rx) = mpsc::unbounded_channel();
        let dispatcher = Dispatcher::new(tx);
        let state = AppState::default();

        let should_continue = middleware
            .handle(&Action::Quit, &state, &dispatcher)
            .await;

        assert!(should_continue);
    }
}
