//! Key Bindings Panel Reducer
//!
//! Handles state updates for the key bindings help panel.

use crate::actions::Action;
use crate::state::KeyBindingsPanelState;

/// Reducer for key bindings panel state
pub fn reduce(mut state: KeyBindingsPanelState, action: &Action) -> KeyBindingsPanelState {
    match action {
        Action::NavigateNext => {
            state.scroll_offset = state.scroll_offset.saturating_add(1);
        }
        Action::NavigatePrevious => {
            // Note: max_scroll should be enforced by view model
            state.scroll_offset = state.scroll_offset.saturating_sub(1);
        }
        Action::NavigateToTop => {
            state.scroll_offset = 0;
        }
        Action::KeyBindingsViewClose | Action::GlobalClose => {
            // Reset scroll when closing
            state.scroll_offset = 0;
        }
        _ => {}
    }
    state
}
