use crate::actions::Action;
use crate::reducers::{debug_console_reducer, splash_reducer};
use crate::state::AppState;
use crate::views::MainView;

/// Reducer - pure function that produces new state from current state + action
/// This is the root reducer that orchestrates all sub-reducers
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    // Handle global actions first
    match action {
        Action::GlobalQuit => {
            // Quit from any view
            state.running = false;
            return state;
        }
        Action::GlobalActivateView(new_view) => {
            state.active_view = new_view.clone();
        }
        Action::BootstrapEnd => {
            // When bootstrap ends, switch to main view
            state.active_view = Box::new(MainView::new());
        }
        _ => {}
    }

    // Run sub-reducers for component-specific actions
    state.splash = splash_reducer::reduce(state.splash, action);
    state.debug_console = debug_console_reducer::reduce(state.debug_console, action);

    // Note: Capabilities are now computed on-demand via the View trait
    // instead of being stored in state

    state
}
