//! Command registry
//!
//! This module defines commands that can be executed via the command palette
//! or keyboard shortcuts. Commands wrap CommandIds with display metadata.
//!
//! Supports both static commands (from CommandId) and dynamic commands
//! (generated at runtime, e.g., for issue tracker links).

use crate::actions::Action;
use crate::command_id::CommandId;
use crate::keybindings::Keymap;

/// Source of a command - either a static CommandId or a dynamic action
#[derive(Debug, Clone)]
pub enum CommandSource {
    /// Static command from CommandId enum
    Static(CommandId),
    /// Dynamic command with custom action and metadata
    Dynamic {
        action: Box<Action>,
        title: String,
        description: String,
        category: String,
    },
}

/// A command that can be executed via command palette or keybinding
#[derive(Debug, Clone)]
pub struct Command {
    /// The source of this command (static or dynamic)
    pub source: CommandSource,
    /// The keyboard shortcut hint (populated from keybindings)
    pub shortcut_hint: Option<String>,
}

impl Command {
    /// Create a new command from a CommandId
    pub fn new(id: CommandId) -> Self {
        Self {
            source: CommandSource::Static(id),
            shortcut_hint: None,
        }
    }

    /// Create a command with a shortcut hint
    pub fn with_shortcut(id: CommandId, hint: impl Into<String>) -> Self {
        Self {
            source: CommandSource::Static(id),
            shortcut_hint: Some(hint.into()),
        }
    }

    /// Create a dynamic command with custom action and metadata
    pub fn dynamic(
        action: Action,
        title: impl Into<String>,
        description: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            source: CommandSource::Dynamic {
                action: Box::new(action),
                title: title.into(),
                description: description.into(),
                category: category.into(),
            },
            shortcut_hint: None,
        }
    }

    /// Get the title for display
    pub fn title(&self) -> &str {
        match &self.source {
            CommandSource::Static(id) => id.title(),
            CommandSource::Dynamic { title, .. } => title,
        }
    }

    /// Get the description for display
    pub fn description(&self) -> &str {
        match &self.source {
            CommandSource::Static(id) => id.description(),
            CommandSource::Dynamic { description, .. } => description,
        }
    }

    /// Get the category for grouping
    pub fn category(&self) -> &str {
        match &self.source {
            CommandSource::Static(id) => id.category(),
            CommandSource::Dynamic { category, .. } => category,
        }
    }

    /// Get the action to dispatch
    pub fn to_action(&self) -> Action {
        match &self.source {
            CommandSource::Static(id) => id.to_action(),
            CommandSource::Dynamic { action, .. } => (**action).clone(),
        }
    }
}

/// Get all commands with shortcut hints populated from the keymap
///
/// Uses `compact_hint_for_command` to show all keybindings for a command
/// (e.g., "q/Esc" for GlobalClose instead of just "q")
pub fn get_palette_commands_with_hints(keymap: &Keymap) -> Vec<Command> {
    CommandId::palette_command_ids()
        .into_iter()
        .map(|id| {
            if let Some(hint) = keymap.compact_hint_for_command(id) {
                Command::with_shortcut(id, hint)
            } else {
                Command::new(id)
            }
        })
        .collect()
}

/// Filter commands based on a search query
///
/// Performs case-insensitive fuzzy matching on title, description, and category.
pub fn filter_commands(commands: &[Command], query: &str) -> Vec<Command> {
    if query.is_empty() {
        return commands.to_vec();
    }

    let query_lower = query.to_lowercase();
    commands
        .iter()
        .filter(|cmd| {
            cmd.title().to_lowercase().contains(&query_lower)
                || cmd.description().to_lowercase().contains(&query_lower)
                || cmd.category().to_lowercase().contains(&query_lower)
        })
        .cloned()
        .collect()
}

/// Generate dynamic commands for opening related issues
///
/// Extracts issue references from the given PR text (title + description)
/// and creates a command for each matched issue. Uses repository context
/// for URL template variables and repo filtering.
pub fn get_issue_commands(
    config: &[gh_pr_config::IssueTrackerConfig],
    pr_texts: &[String],
    repo_ctx: &crate::utils::issue_extractor::RepoContext,
) -> Vec<Command> {
    use crate::actions::{Action, PullRequestAction};
    use crate::utils::issue_extractor::IssueExtractor;
    use std::collections::HashSet;

    let extractor = IssueExtractor::from_config(config);
    if extractor.is_empty() {
        return vec![];
    }

    // Collect all unique issues from all PR texts
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut commands = vec![];

    for text in pr_texts {
        for issue in extractor.extract_all(text, repo_ctx) {
            let key = (issue.tracker_name.clone(), issue.issue_id.clone());
            if seen.insert(key) {
                commands.push(Command::dynamic(
                    Action::PullRequest(PullRequestAction::OpenRelatedIssue {
                        url: issue.url.clone(),
                    }),
                    format!("Open issue {}", issue.issue_id),
                    format!("Open issue {} on {}", issue.issue_id, issue.tracker_name),
                    "Issue Tracker",
                ));
            }
        }
    }

    commands
}
