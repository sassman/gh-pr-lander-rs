//! Repository Middleware
//!
//! Handles repository-related side effects:
//! - Loading recent repositories from config on LoadRecentRepositories
//! - Managing the add repository form view
//! - Translating generic TextInput actions to AddRepository-specific actions

use crate::actions::{Action, AddRepositoryAction, BootstrapAction, GlobalAction};
use crate::dispatcher::Dispatcher;
use crate::domain_models::Repository;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::ViewId;
use gh_pr_config::load_recent_repositories;

/// Repository middleware - handles repository loading and add repository form
pub struct RepositoryMiddleware;

impl RepositoryMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Check if the add repository view is the active view
    fn is_add_repo_active(state: &AppState) -> bool {
        state.active_view().view_id() == ViewId::AddRepository
    }
}

impl Default for RepositoryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for RepositoryMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            // Handle loading recent repositories from config
            Action::Bootstrap(BootstrapAction::LoadRecentRepositories) => {
                log::info!("RepositoryMiddleware: Loading recent repositories from config");

                let recent_repos = load_recent_repositories();
                if !recent_repos.is_empty() {
                    let repositories: Vec<Repository> = recent_repos
                        .into_iter()
                        .map(|r| Repository::new(r.org, r.repo, r.branch))
                        .collect();
                    log::info!(
                        "RepositoryMiddleware: Found {} recent repositories",
                        repositories.len()
                    );
                    dispatcher.dispatch(Action::Bootstrap(BootstrapAction::RepositoryAddBulk(
                        repositories,
                    )));
                } else {
                    log::info!("RepositoryMiddleware: No recent repositories found");
                    // Even if no repos, signal that loading is done
                    dispatcher.dispatch(Action::Bootstrap(
                        BootstrapAction::LoadRecentRepositoriesDone,
                    ));
                }

                true // Let action pass through
            }

            // Handle closing the add repository view
            Action::AddRepository(AddRepositoryAction::Close) => {
                if Self::is_add_repo_active(state) && state.view_stack.len() > 1 {
                    log::debug!("Closing add repository form");
                    dispatcher.dispatch(Action::Global(GlobalAction::Close));
                }
                true // Let action pass through to reducer to reset form
            }

            // Handle confirm - close view if form is valid
            Action::AddRepository(AddRepositoryAction::Confirm) => {
                if Self::is_add_repo_active(state) && state.add_repo_form.is_valid() {
                    // Close the view after successful add
                    if state.view_stack.len() > 1 {
                        dispatcher.dispatch(Action::Global(GlobalAction::Close));
                    }
                }
                true // Let action pass through to reducer to add repository
            }

            // All other actions pass through
            _ => true,
        }
    }
}
