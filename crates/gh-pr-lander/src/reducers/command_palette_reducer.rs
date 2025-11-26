use crate::actions::Action;
use crate::commands::{filter_commands, get_all_commands};
use crate::state::CommandPaletteState;

/// Reducer for command palette state
pub fn reduce(mut state: CommandPaletteState, action: &Action) -> CommandPaletteState {
    match action {
        Action::CommandPaletteUpdateQuery(query) => {
            state.query = query.clone();
            state.selected_index = 0; // Reset selection when query changes
        }
        Action::CommandPaletteClear => {
            state.query.clear();
            state.selected_index = 0;
        }
        Action::NavigateNext => {
            // Move to next command
            let all_commands = get_all_commands();
            let filtered = filter_commands(&all_commands, &state.query);
            if !filtered.is_empty() {
                state.selected_index = (state.selected_index + 1).min(filtered.len() - 1);
            }
        }
        Action::NavigatePrevious => {
            // Move to previous command
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
        _ => {}
    }
    state
}
