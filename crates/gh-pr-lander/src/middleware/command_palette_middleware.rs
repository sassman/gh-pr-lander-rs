//! Command Palette Middleware
//!
//! Executes the selected command when CommandPalette::Execute is dispatched.
//! Text input and navigation are handled via view translation (translate_text_input/translate_navigation).

use crate::actions::{Action, CommandPaletteAction};
use crate::command_id::CommandId;
use crate::commands::{
    filter_commands, get_issue_commands, get_palette_commands_with_hints, Command,
};
use crate::dispatcher::Dispatcher;
use crate::middleware::Middleware;
use crate::state::AppState;
use crate::utils::issue_extractor::RepoContext;
use gh_pr_fix_with_claude::PrId;

/// Middleware that handles command palette command execution
pub struct CommandPaletteMiddleware;

impl CommandPaletteMiddleware {
    pub fn new() -> Self {
        Self
    }

    /// Get PR texts from currently selected/active PRs for issue extraction
    fn get_selected_pr_texts(state: &AppState) -> Vec<String> {
        let repo_idx = state.main_view.selected_repository;
        let Some(repo_data) = state.main_view.repo_data.get(&repo_idx) else {
            return vec![];
        };

        // If PRs are explicitly selected, use those; otherwise use cursor PR
        let pr_numbers: Vec<usize> = if repo_data.selected_pr_numbers.is_empty() {
            // Use cursor PR
            repo_data
                .prs
                .get(repo_data.selected_pr)
                .map(|pr| vec![pr.number])
                .unwrap_or_default()
        } else {
            // Use explicitly selected PRs
            repo_data.selected_pr_numbers.iter().copied().collect()
        };

        // Build text for each PR (title + description)
        pr_numbers
            .iter()
            .filter_map(|&num| repo_data.prs.iter().find(|pr| pr.number == num))
            .map(|pr| format!("{} {}", pr.title, pr.body))
            .collect()
    }

    /// Get the PrId for the cursor PR (if any)
    fn get_cursor_pr_id(state: &AppState) -> Option<PrId> {
        let repo_idx = state.main_view.selected_repository;
        let repo_data = state.main_view.repo_data.get(&repo_idx)?;
        let pr = repo_data.prs.get(repo_data.selected_pr)?;
        Some(PrId::from(pr))
    }

    /// Get repository context for issue extraction
    fn get_repo_context(state: &AppState) -> RepoContext {
        let repo_idx = state.main_view.selected_repository;
        state
            .main_view
            .repositories
            .get(repo_idx)
            .map(|repo| {
                RepoContext::new(
                    &repo.org,
                    &repo.repo,
                    repo.host.as_deref().unwrap_or(gh_client::DEFAULT_HOST),
                )
            })
            .unwrap_or_default()
    }
}

impl Default for CommandPaletteMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CommandPaletteMiddleware {
    fn handle(&mut self, action: &Action, state: &AppState, dispatcher: &Dispatcher) -> bool {
        // Handle command execution - dispatch the selected command's action
        if let Action::CommandPalette(CommandPaletteAction::Execute) = action {
            // Get static commands
            let mut all_commands = get_palette_commands_with_hints(&state.keymap);

            // Add dynamic issue commands based on selected PRs and repo context
            let pr_texts = Self::get_selected_pr_texts(state);
            let repo_ctx = Self::get_repo_context(state);
            let issue_commands =
                get_issue_commands(&state.app_config.issue_tracker, &pr_texts, &repo_ctx);
            all_commands.extend(issue_commands);

            // Add context-aware Claude commands (must match view model)
            let cursor_pr_id = Self::get_cursor_pr_id(state);
            let has_session = cursor_pr_id
                .as_ref()
                .map(|id| state.claude_sessions.has_session(id))
                .unwrap_or(false);
            if has_session {
                all_commands.push(Command::new(CommandId::ClaudeAttachSession));
            } else {
                all_commands.push(Command::new(CommandId::ClaudeFixPr));
            }

            let filtered = filter_commands(&all_commands, &state.command_palette.query);

            if let Some(cmd) = filtered.get(state.command_palette.selected_index) {
                log::debug!("Command palette executing: {}", cmd.title());
                dispatcher.dispatch(cmd.to_action());
            }
            // Let the action continue to the reducer to close the palette
            return true;
        }

        // All other actions pass through
        true
    }
}
