//! Add Repository Middleware
//!
//! Translates generic TextInput actions to AddRepo-specific actions
//! when the add repository view is active.

use crate::actions::Action;
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::views::ViewId;

/// Middleware that handles add repository form interactions
pub struct AddRepositoryMiddleware;

impl AddRepositoryMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Check if the add repository view is the active view
    fn is_active(state: &AppState) -> bool {
        state.active_view().view_id() == ViewId::AddRepository
    }
}

impl Default for AddRepositoryMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for AddRepositoryMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Only process when add repository view is active
        if !Self::is_active(state) {
            return true; // Pass through
        }

        match action {
            // Translate generic TextInput actions to AddRepo-specific actions
            Action::TextInputChar(c) => {
                dispatcher.dispatch(Action::AddRepoChar(*c));
                false // Consume the original action
            }

            Action::TextInputBackspace => {
                dispatcher.dispatch(Action::AddRepoBackspace);
                false
            }

            Action::TextInputClearLine => {
                dispatcher.dispatch(Action::AddRepoClearField);
                false
            }

            Action::TextInputEscape => {
                dispatcher.dispatch(Action::AddRepoClose);
                false
            }

            Action::TextInputConfirm => {
                dispatcher.dispatch(Action::AddRepoConfirm);
                false
            }

            // Tab navigation between fields
            Action::NavigateNext => {
                dispatcher.dispatch(Action::AddRepoNextField);
                false
            }

            Action::NavigatePrevious => {
                dispatcher.dispatch(Action::AddRepoPrevField);
                false
            }

            // All other actions pass through
            _ => true,
        }
    }
}
