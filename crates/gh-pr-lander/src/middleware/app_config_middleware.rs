//! App Config Middleware
//!
//! Handles loading application configuration on bootstrap.

use crate::actions::{Action, BootstrapAction};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use gh_pr_config::AppConfig;
use std::path::PathBuf;

/// Middleware for loading application configuration
pub struct AppConfigMiddleware {
    config_loaded: bool,
    config_override: Option<PathBuf>,
}

impl AppConfigMiddleware {
    pub fn new(config_override: Option<PathBuf>) -> Self {
        Self {
            config_loaded: false,
            config_override,
        }
    }
}

impl Default for AppConfigMiddleware {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Middleware for AppConfigMiddleware {
    fn handle(&mut self, action: &Action, _state: &AppState, dispatcher: &Dispatcher) -> bool {
        match action {
            Action::Bootstrap(BootstrapAction::Start) => {
                if !self.config_loaded {
                    log::info!("AppConfigMiddleware: Loading application configuration");
                    // This can block - we're on the background thread
                    let config = AppConfig::load(self.config_override.as_deref());
                    log::info!(
                        "AppConfigMiddleware: Loaded config (ide_command: {})",
                        config.ide_command
                    );
                    dispatcher.dispatch(Action::Bootstrap(BootstrapAction::ConfigLoaded(config)));
                    self.config_loaded = true;
                }
                true // Pass through
            }
            _ => true, // All other actions pass through
        }
    }
}
