//! Command palette reducer
//!
//! Handles CommandPalette-specific actions. This reducer only processes
//! actions prefixed with CommandPalette*, so it doesn't need to check
//! if the command palette is active - that's the middleware's job.

use crate::actions::CommandPaletteAction;
use crate::commands::{filter_commands, Command};
use crate::state::CommandPaletteState;

/// Reducer for command palette state.
///
/// Pure state transformation. `all_commands` is the full palette command set
/// (static + dynamic) — single source of truth so selection clamp matches
/// what the view renders. Visible-window math (offset / viewport) is derived
/// in the view model from the rendered area; nothing about layout lives here.
pub fn reduce_command_palette(
    mut state: CommandPaletteState,
    action: &CommandPaletteAction,
    all_commands: &[Command],
) -> CommandPaletteState {
    match action {
        CommandPaletteAction::Char(c) => {
            state.query.push(*c);
            state.selected_index = 0;
        }

        CommandPaletteAction::Backspace => {
            state.query.pop();
            state.selected_index = 0;
        }

        CommandPaletteAction::Clear => {
            state.query.clear();
            state.selected_index = 0;
        }

        CommandPaletteAction::Close | CommandPaletteAction::Execute => {
            state.query.clear();
            state.selected_index = 0;
        }

        CommandPaletteAction::NavigateNext => {
            let filtered_len = filter_commands(all_commands, &state.query).len();
            if filtered_len > 0 {
                state.selected_index = (state.selected_index + 1).min(filtered_len - 1);
            }
        }

        CommandPaletteAction::NavigatePrev => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
        }
    }

    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_id::CommandId;

    fn make_commands(n: usize) -> Vec<Command> {
        (0..n)
            .map(|_| Command::new(CommandId::GlobalClose))
            .collect()
    }

    #[test]
    fn navigate_next_clamps_to_filtered_len_minus_one() {
        let cmds = make_commands(3);
        let mut state = CommandPaletteState::default();
        for _ in 0..10 {
            state = reduce_command_palette(state, &CommandPaletteAction::NavigateNext, &cmds);
        }
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn navigate_prev_does_not_underflow() {
        let cmds = make_commands(3);
        let mut state = CommandPaletteState::default();
        for _ in 0..5 {
            state = reduce_command_palette(state, &CommandPaletteAction::NavigatePrev, &cmds);
        }
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn typing_resets_selection() {
        let cmds = make_commands(20);
        let mut state = CommandPaletteState {
            selected_index: 8,
            ..Default::default()
        };
        state = reduce_command_palette(state, &CommandPaletteAction::Char('x'), &cmds);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.query, "x");
    }

    #[test]
    fn close_clears_query_and_selection() {
        let cmds = make_commands(5);
        let mut state = CommandPaletteState {
            query: "abc".into(),
            selected_index: 3,
        };
        state = reduce_command_palette(state, &CommandPaletteAction::Close, &cmds);
        assert!(state.query.is_empty());
        assert_eq!(state.selected_index, 0);
    }
}
