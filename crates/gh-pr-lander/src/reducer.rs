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
            // View stack logic:
            // - If the new view is floating:
            //   - Check if same view is already on top of stack (prevent duplicates)
            //   - If not duplicate, push it onto the stack
            // - If the new view is not floating, clear the stack and replace with the new view
            if new_view.is_floating() {
                // Check if this floating view is already the top-most view
                let is_duplicate = state
                    .view_stack
                    .last()
                    .map(|top| top.view_id() == new_view.view_id())
                    .unwrap_or(false);

                if is_duplicate {
                    log::debug!(
                        "Ignoring duplicate floating view: {:?}",
                        new_view.view_id()
                    );
                } else {
                    log::debug!("Pushing floating view onto stack: {:?}", new_view.view_id());
                    state.view_stack.push(new_view.clone());
                }
            } else {
                log::debug!(
                    "Replacing view stack with non-floating view: {:?}",
                    new_view.view_id()
                );
                state.view_stack.clear();
                state.view_stack.push(new_view.clone());
            }
        }
        Action::GlobalClose => {
            // Close the top-most view
            // If there's more than one view in the stack, pop the top one
            // If there's only one view left, quit the application
            if state.view_stack.len() > 1 {
                let popped = state.view_stack.pop();
                log::debug!("Closed view: {:?}", popped.map(|v| v.view_id()));
            } else {
                log::debug!("Closing last view - quitting application");
                state.running = false;
            }
        }
        Action::BootstrapEnd => {
            // When bootstrap ends, switch to main view
            state.view_stack.clear();
            state.view_stack.push(Box::new(MainView::new()));
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
