//! Stable Pull Request identifier
//!
//! Uses the GitHub PR URL as a stable, unique key that encodes
//! host, org, repo, and PR number. Unlike positional indices,
//! this survives repo reordering, additions, and removals.

/// Stable identifier for a Pull Request.
///
/// Backed by the PR's GitHub URL (e.g., `https://github.com/org/repo/pull/42`).
/// This is stable across repo list changes, unlike positional indices.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PrId(String);

impl PrId {
    /// Create a PrId from a PR's HTML URL directly
    pub fn from_url(url: impl Into<String>) -> Self {
        Self(url.into())
    }

    /// Construct a PrId from component parts (for cases without a full Pr object)
    pub fn from_parts(host: &str, org: &str, repo: &str, pr_number: usize) -> Self {
        Self(format!(
            "https://{}/{}/{}/pull/{}",
            host, org, repo, pr_number
        ))
    }

    /// Get the underlying URL string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PrId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
