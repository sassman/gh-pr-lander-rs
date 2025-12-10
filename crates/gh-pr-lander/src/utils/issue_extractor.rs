//! Issue extraction from PR titles and descriptions
//!
//! Extracts issue references from PR text using configurable regex patterns
//! and generates URLs to external issue trackers (Jira, Linear, GitHub, etc.).
//!
//! Supports repository context for:
//! - URL template variables: `$ISSUE_NO`, `$ORG`, `$REPO`, `$HOST`
//! - Scoping trackers to specific repos via glob patterns

use gh_pr_config::IssueTrackerConfig;
use regex::Regex;

/// Repository context for issue extraction
#[derive(Debug, Clone, Default)]
pub struct RepoContext {
    /// Organization/owner name
    pub org: String,
    /// Repository name
    pub repo: String,
    /// GitHub host (e.g., "github.com" or GHE hostname)
    pub host: String,
}

impl RepoContext {
    pub fn new(org: impl Into<String>, repo: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            org: org.into(),
            repo: repo.into(),
            host: host.into(),
        }
    }
}

/// A matched issue with its tracker info and URL
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchedIssue {
    /// Display name of the tracker (e.g., "Jira", "Linear")
    pub tracker_name: String,
    /// The matched issue ID (e.g., "BAR-123")
    pub issue_id: String,
    /// The full URL to open the issue
    pub url: String,
}

/// Single issue tracker with compiled regex
struct IssueTracker {
    name: String,
    pattern: Regex,
    url_template: String,
    repo_patterns: Vec<String>,
}

impl IssueTracker {
    fn new(config: &IssueTrackerConfig) -> Result<Self, regex::Error> {
        Ok(Self {
            name: config.name.clone(),
            pattern: Regex::new(&config.pattern)?,
            url_template: config.url.clone(),
            repo_patterns: config.repos.clone(),
        })
    }

    /// Check if this tracker applies to the given repository
    fn matches_repo(&self, ctx: &RepoContext) -> bool {
        // If no repo patterns specified, tracker applies to all repos
        if self.repo_patterns.is_empty() {
            return true;
        }

        let repo_path = format!("{}/{}", ctx.org, ctx.repo);
        self.repo_patterns
            .iter()
            .any(|pattern| glob_match(pattern, &repo_path))
    }

    /// Extract issue from text and build URL with context
    ///
    /// If the regex has a capture group, uses the first group as $ISSUE_NO.
    /// Otherwise uses the full match. This allows patterns like `#(\d+)` to
    /// extract just the number for URLs while displaying the full match.
    fn extract(&self, text: &str, ctx: &RepoContext) -> Option<MatchedIssue> {
        if !self.matches_repo(ctx) {
            return None;
        }

        self.pattern.captures(text).map(|caps| {
            let full_match = caps.get(0).unwrap().as_str().to_string();
            // Use first capture group if present, otherwise full match
            let issue_no = caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| full_match.clone());

            let url = self
                .url_template
                .replace("$ISSUE_NO", &issue_no)
                .replace("$ORG", &ctx.org)
                .replace("$REPO", &ctx.repo)
                .replace("$HOST", &ctx.host);
            MatchedIssue {
                tracker_name: self.name.clone(),
                issue_id: full_match, // Display the full match in command palette
                url,
            }
        })
    }
}

/// Simple glob matching supporting `*` wildcard
///
/// Patterns:
/// - `org/*` matches any repo in org
/// - `org/repo` matches exact repo
/// - `*` matches anything
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 1 {
        // No wildcard - exact match
        return pattern == text;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if i == 0 {
            // First part must be prefix
            if !text.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else if i == parts.len() - 1 {
            // Last part must be suffix
            if !text[pos..].ends_with(part) {
                return false;
            }
        } else {
            // Middle parts must exist somewhere
            if let Some(found_pos) = text[pos..].find(part) {
                pos += found_pos + part.len();
            } else {
                return false;
            }
        }
    }

    true
}

/// Collection of issue trackers for extracting issues from PR text
#[derive(Default)]
pub struct IssueExtractor {
    trackers: Vec<IssueTracker>,
}

impl IssueExtractor {
    /// Create an IssueExtractor from configuration
    ///
    /// Invalid regex patterns are logged and skipped.
    pub fn from_config(configs: &[IssueTrackerConfig]) -> Self {
        log::debug!("IssueExtractor: creating from {} configs", configs.len());
        let trackers = configs
            .iter()
            .filter_map(|c| {
                log::debug!(
                    "IssueExtractor: loading tracker '{}' with pattern '{}'",
                    c.name,
                    c.pattern
                );
                IssueTracker::new(c)
                    .map_err(|e| {
                        log::warn!(
                            "Invalid regex pattern for issue tracker '{}': {}",
                            c.name,
                            e
                        )
                    })
                    .ok()
            })
            .collect();
        Self { trackers }
    }

    /// Find all matching issues from all configured trackers
    ///
    /// Each tracker returns at most one match (first occurrence).
    /// Trackers are filtered by repository context if they have repo patterns.
    pub fn extract_all(&self, text: &str, ctx: &RepoContext) -> Vec<MatchedIssue> {
        self.trackers
            .iter()
            .filter_map(|t| t.extract(text, ctx))
            .collect()
    }

    /// Check if any trackers are configured
    pub fn is_empty(&self) -> bool {
        self.trackers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(name: &str, pattern: &str, url: &str) -> IssueTrackerConfig {
        IssueTrackerConfig {
            name: name.to_string(),
            pattern: pattern.to_string(),
            url: url.to_string(),
            repos: vec![],
        }
    }

    fn make_config_with_repos(
        name: &str,
        pattern: &str,
        url: &str,
        repos: Vec<&str>,
    ) -> IssueTrackerConfig {
        IssueTrackerConfig {
            name: name.to_string(),
            pattern: pattern.to_string(),
            url: url.to_string(),
            repos: repos.into_iter().map(String::from).collect(),
        }
    }

    fn default_ctx() -> RepoContext {
        RepoContext::new("my-org", "my-repo", "github.com")
    }

    #[test]
    fn test_single_tracker_match() {
        let configs = vec![make_config(
            "Jira",
            r"BAR-\d+",
            "https://jira.example.com/browse/$ISSUE_NO",
        )];
        let extractor = IssueExtractor::from_config(&configs);

        let matches = extractor.extract_all("feat: BAR-123 implement login", &default_ctx());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].tracker_name, "Jira");
        assert_eq!(matches[0].issue_id, "BAR-123");
        assert_eq!(matches[0].url, "https://jira.example.com/browse/BAR-123");
    }

    #[test]
    fn test_url_template_variables() {
        // Use capture group to extract just the number for URL
        let configs = vec![make_config(
            "GitHub",
            r"#(\d+)",
            "https://$HOST/$ORG/$REPO/issues/$ISSUE_NO",
        )];
        let extractor = IssueExtractor::from_config(&configs);
        let ctx = RepoContext::new("acme", "widgets", "github.com");

        let matches = extractor.extract_all("Fixes #42", &ctx);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].issue_id, "#42"); // Full match for display
        assert_eq!(matches[0].url, "https://github.com/acme/widgets/issues/42");
        // Capture group in URL
    }

    #[test]
    fn test_ghe_host_variable() {
        let configs = vec![make_config(
            "GitHub",
            r"#(\d+)",
            "https://$HOST/$ORG/$REPO/issues/$ISSUE_NO",
        )];
        let extractor = IssueExtractor::from_config(&configs);
        let ctx = RepoContext::new("corp", "internal", "github.corp.com");

        let matches = extractor.extract_all("Fixes #99", &ctx);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].issue_id, "#99");
        assert_eq!(
            matches[0].url,
            "https://github.corp.com/corp/internal/issues/99"
        );
    }

    #[test]
    fn test_repo_filter_matches() {
        let configs = vec![make_config_with_repos(
            "Jira",
            r"PROJ-\d+",
            "https://jira.example.com/browse/$ISSUE_NO",
            vec!["my-org/*"],
        )];
        let extractor = IssueExtractor::from_config(&configs);
        let ctx = RepoContext::new("my-org", "any-repo", "github.com");

        let matches = extractor.extract_all("PROJ-123", &ctx);
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_repo_filter_no_match() {
        let configs = vec![make_config_with_repos(
            "Jira",
            r"PROJ-\d+",
            "https://jira.example.com/browse/$ISSUE_NO",
            vec!["other-org/*"],
        )];
        let extractor = IssueExtractor::from_config(&configs);
        let ctx = RepoContext::new("my-org", "my-repo", "github.com");

        let matches = extractor.extract_all("PROJ-123", &ctx);
        assert!(matches.is_empty());
    }

    #[test]
    fn test_repo_filter_exact_match() {
        let configs = vec![make_config_with_repos(
            "Jira",
            r"PROJ-\d+",
            "https://jira.example.com/browse/$ISSUE_NO",
            vec!["my-org/specific-repo"],
        )];
        let extractor = IssueExtractor::from_config(&configs);

        // Exact match
        let ctx1 = RepoContext::new("my-org", "specific-repo", "github.com");
        assert_eq!(extractor.extract_all("PROJ-1", &ctx1).len(), 1);

        // Different repo - no match
        let ctx2 = RepoContext::new("my-org", "other-repo", "github.com");
        assert!(extractor.extract_all("PROJ-1", &ctx2).is_empty());
    }

    #[test]
    fn test_multiple_trackers() {
        let configs = vec![
            make_config(
                "Jira",
                r"BAR-\d+",
                "https://jira.example.com/browse/$ISSUE_NO",
            ),
            make_config(
                "Linear",
                r"LIN-\d+",
                "https://linear.app/team/issue/$ISSUE_NO",
            ),
        ];
        let extractor = IssueExtractor::from_config(&configs);

        let matches = extractor.extract_all("feat: BAR-123 also fixes LIN-456", &default_ctx());
        assert_eq!(matches.len(), 2);
        assert!(matches.iter().any(|m| m.issue_id == "BAR-123"));
        assert!(matches.iter().any(|m| m.issue_id == "LIN-456"));
    }

    #[test]
    fn test_no_match() {
        let configs = vec![make_config(
            "Jira",
            r"BAR-\d+",
            "https://jira.example.com/browse/$ISSUE_NO",
        )];
        let extractor = IssueExtractor::from_config(&configs);

        let matches =
            extractor.extract_all("feat: implement login without issue ref", &default_ctx());
        assert!(matches.is_empty());
    }

    #[test]
    fn test_invalid_regex_skipped() {
        let configs = vec![
            make_config("Invalid", r"[invalid", "https://example.com/$ISSUE_NO"),
            make_config("Valid", r"FOO-\d+", "https://example.com/$ISSUE_NO"),
        ];
        let extractor = IssueExtractor::from_config(&configs);

        // Invalid tracker should be skipped, valid one should work
        let matches = extractor.extract_all("FOO-999", &default_ctx());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].issue_id, "FOO-999");
    }

    #[test]
    fn test_first_match_per_tracker() {
        let configs = vec![make_config(
            "Jira",
            r"BAR-\d+",
            "https://jira.example.com/browse/$ISSUE_NO",
        )];
        let extractor = IssueExtractor::from_config(&configs);

        // Only first match is returned
        let matches = extractor.extract_all("BAR-1 and BAR-2 and BAR-3", &default_ctx());
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].issue_id, "BAR-1");
    }

    #[test]
    fn test_empty_config() {
        let extractor = IssueExtractor::from_config(&[]);
        assert!(extractor.is_empty());
        assert!(extractor.extract_all("anything", &default_ctx()).is_empty());
    }

    #[test]
    fn test_github_issue_real_world() {
        // Real-world test case from PR sassman/t-rec-rs#248
        let configs = vec![make_config(
            "GitHub",
            r"#(\d+)",
            "https://$HOST/$ORG/$REPO/issues/$ISSUE_NO",
        )];
        let extractor = IssueExtractor::from_config(&configs);
        let ctx = RepoContext::new("sassman", "t-rec-rs", "github.com");

        let pr_text = "feat: wayland api impl (see #27) WIP relates to #27";
        let matches = extractor.extract_all(pr_text, &ctx);

        assert_eq!(matches.len(), 1, "Should find one issue reference");
        assert_eq!(matches[0].issue_id, "#27");
        assert_eq!(
            matches[0].url,
            "https://github.com/sassman/t-rec-rs/issues/27"
        );
    }

    #[test]
    fn test_end_to_end_simulating_toml_config() {
        // Test with the exact pattern that comes from TOML parsing
        // In TOML file: pattern = "#(\\d+)" -> after parsing: #(\d+)
        let config = IssueTrackerConfig {
            name: "GitHub".to_string(),
            pattern: r"#(\d+)".to_string(), // This is what TOML parsing produces
            url: "https://$HOST/$ORG/$REPO/issues/$ISSUE_NO".to_string(),
            repos: vec![],
        };

        // Create extractor
        let extractor = IssueExtractor::from_config(&[config]);
        assert!(!extractor.is_empty(), "Extractor should have trackers");

        let ctx = RepoContext::new("sassman", "t-rec-rs", "github.com");
        let pr_text = "feat: wayland api impl (see #27) WIP relates to #27";
        let matches = extractor.extract_all(pr_text, &ctx);

        assert_eq!(matches.len(), 1, "Should find issue #27");
        assert_eq!(matches[0].issue_id, "#27");
        assert_eq!(
            matches[0].url,
            "https://github.com/sassman/t-rec-rs/issues/27"
        );
    }

    #[test]
    fn test_glob_match() {
        // Wildcard patterns
        assert!(glob_match("org/*", "org/repo"));
        assert!(glob_match("org/*", "org/any-repo"));
        assert!(!glob_match("org/*", "other/repo"));

        // Exact match
        assert!(glob_match("org/repo", "org/repo"));
        assert!(!glob_match("org/repo", "org/other"));

        // Star only
        assert!(glob_match("*", "anything"));
        assert!(glob_match("*", "org/repo"));
    }
}
