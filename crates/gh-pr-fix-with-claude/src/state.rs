//! Session state tracking

use std::collections::HashMap;

use crate::PrId;

/// Tracks active Claude Code sessions across all repos/PRs
#[derive(Debug, Clone, Default)]
pub struct ClaudeSessionsState {
    /// Map of PrId -> session info
    pub sessions: HashMap<PrId, ClaudeSession>,
}

impl ClaudeSessionsState {
    /// Check if a session exists for the given PR
    pub fn has_session(&self, pr_id: &PrId) -> bool {
        self.sessions.contains_key(pr_id)
    }

    /// Get session for the given PR
    pub fn get_session(&self, pr_id: &PrId) -> Option<&ClaudeSession> {
        self.sessions.get(pr_id)
    }

    /// Register a new session
    pub fn add_session(&mut self, pr_id: PrId, session: ClaudeSession) {
        self.sessions.insert(pr_id, session);
    }

    /// Remove a session
    pub fn remove_session(&mut self, pr_id: &PrId) {
        self.sessions.remove(pr_id);
    }
}

/// A single Claude Code background session
#[derive(Debug, Clone)]
pub struct ClaudeSession {
    /// GNU screen session name (for attach/detach)
    pub screen_name: String,
    /// Working directory where the PR is checked out
    pub work_dir: String,
    /// When the session was started
    pub started_at: chrono::DateTime<chrono::Local>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PrId;

    fn pr_id(number: usize) -> PrId {
        PrId::from_parts("github.com", "org", "repo", number)
    }

    #[test]
    fn test_pr_id_from_parts() {
        let id = PrId::from_parts("github.com", "acme", "widgets", 42);
        assert_eq!(id.as_str(), "https://github.com/acme/widgets/pull/42");
    }

    #[test]
    fn test_pr_id_from_url() {
        let id = PrId::from_url("https://github.com/acme/widgets/pull/99");
        assert_eq!(id.as_str(), "https://github.com/acme/widgets/pull/99");
    }

    #[test]
    fn test_pr_id_equality() {
        let a = PrId::from_parts("github.com", "org", "repo", 1);
        let b = PrId::from_url("https://github.com/org/repo/pull/1");
        assert_eq!(a, b);
    }

    #[test]
    fn test_add_and_get_session() {
        let mut state = ClaudeSessionsState::default();
        let id = pr_id(42);
        assert!(!state.has_session(&id));

        state.add_session(
            id.clone(),
            ClaudeSession {
                screen_name: "claude-org-repo-pr-42".to_string(),
                work_dir: "/tmp/test".to_string(),
                started_at: chrono::Local::now(),
            },
        );

        assert!(state.has_session(&id));
        assert_eq!(
            state.get_session(&id).unwrap().screen_name,
            "claude-org-repo-pr-42"
        );
    }

    #[test]
    fn test_remove_session() {
        let mut state = ClaudeSessionsState::default();
        let id = pr_id(42);
        state.add_session(
            id.clone(),
            ClaudeSession {
                screen_name: "test".to_string(),
                work_dir: "/tmp".to_string(),
                started_at: chrono::Local::now(),
            },
        );

        state.remove_session(&id);
        assert!(!state.has_session(&id));
    }

    #[test]
    fn test_multiple_sessions_independent() {
        let mut state = ClaudeSessionsState::default();
        let id1 = PrId::from_parts("github.com", "org", "repo-a", 1);
        let id2 = PrId::from_parts("github.com", "org", "repo-b", 2);
        let id3 = PrId::from_parts("github.com", "org", "repo-a", 2);

        state.add_session(
            id1.clone(),
            ClaudeSession {
                screen_name: "s1".to_string(),
                work_dir: "/tmp/1".to_string(),
                started_at: chrono::Local::now(),
            },
        );
        state.add_session(
            id2.clone(),
            ClaudeSession {
                screen_name: "s2".to_string(),
                work_dir: "/tmp/2".to_string(),
                started_at: chrono::Local::now(),
            },
        );

        assert!(state.has_session(&id1));
        assert!(state.has_session(&id2));
        assert!(!state.has_session(&id3));

        state.remove_session(&id1);
        assert!(!state.has_session(&id1));
        assert!(state.has_session(&id2));
    }

    #[test]
    fn test_ghe_pr_id_distinct_from_github_com() {
        let gh = PrId::from_parts("github.com", "org", "repo", 1);
        let ghe = PrId::from_parts("github.acme.com", "org", "repo", 1);
        assert_ne!(gh, ghe);
    }
}
