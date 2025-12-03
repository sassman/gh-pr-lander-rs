//! Bootstrap actions
//!
//! Actions for application initialization and configuration loading.

use crate::domain_models::Repository;

/// Actions for application bootstrap/initialization
#[derive(Debug, Clone)]
pub enum BootstrapAction {
    /// Bootstrap process started
    Start,
    /// Bootstrap process completed
    End,
    /// Application configuration loaded
    ConfigLoaded(gh_pr_config::AppConfig),
    /// Request to load recent repositories from config
    LoadRecentRepositories,
    /// Recent repositories loaded
    LoadRecentRepositoriesDone,
    /// Add multiple repositories at once (from config)
    RepositoryAddBulk(Vec<Repository>),
}
