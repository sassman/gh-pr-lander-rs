use crate::actions::Action;
use crate::state::AppState;

/// Reducer - pure function that produces new state from current state + action
pub fn reduce(mut state: AppState, action: &Action) -> AppState {
    match action {
        Action::GlobalQuit => {
            state.running = false;
        }
        Action::GlobalClose => {
            // For now, close means quit (later will have panel stack)
            state.running = false;
        }
        _ => {
            // Unhandled actions - no state change
        }
    }

    state
}
