//! Claude session reducer

use crate::actions::{Action, ClaudeSessionAction};
use crate::state::AppState;
use gh_pr_fix_with_claude::ClaudeSession;

pub fn reduce(state: &mut AppState, action: &Action) {
    match action {
        Action::ClaudeSession(ClaudeSessionAction::Started {
            pr_id,
            screen_name,
            work_dir,
        }) => {
            state.claude_sessions.add_session(
                pr_id.clone(),
                ClaudeSession {
                    screen_name: screen_name.clone(),
                    work_dir: work_dir.clone(),
                    started_at: chrono::Local::now(),
                },
            );
        }
        Action::ClaudeSession(ClaudeSessionAction::Completed { pr_id }) => {
            state.claude_sessions.remove_session(pr_id);
        }
        _ => {}
    }
}
