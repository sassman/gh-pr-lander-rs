//! Application configuration
//!
//! Configuration loaded from gh-pr-tui.toml file.

use gh_pr_fix_with_claude::FixWithClaudeConfig;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// Configuration for an external issue tracker (Jira, Linear, etc.)
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct IssueTrackerConfig {
    /// Display name (e.g., "Jira", "Linear")
    pub name: String,
    /// Regex pattern to match issue references in PR title/description
    pub pattern: String,
    /// URL template with placeholders: $ISSUE_NO, $ORG, $REPO, $HOST
    pub url: String,
    /// Optional: glob patterns to restrict this tracker to specific repos (e.g., ["my-org/*"])
    #[serde(default)]
    pub repos: Vec<String>,
}

/// Application configuration loaded from gh-pr-tui.toml
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    /// Command to open files/directories in IDE (e.g., "code", "zed", "cursor")
    #[serde(default = "default_ide_command")]
    pub ide_command: String,

    /// Temporary directory for cloning PRs
    #[serde(default = "default_temp_dir")]
    pub temp_dir: String,

    /// Default message for PR approvals
    #[serde(default = "default_approval_message")]
    pub approval_message: String,

    /// Default message for PR comments
    #[serde(default = "default_comment_message")]
    pub comment_message: String,

    /// Default message for requesting changes on PRs
    #[serde(default = "default_request_changes_message")]
    pub request_changes_message: String,

    /// Default message for closing PRs
    #[serde(default = "default_close_message")]
    pub close_message: String,

    /// External issue tracker configurations
    #[serde(default)]
    pub issue_tracker: Vec<IssueTrackerConfig>,

    /// Fix with Claude Code session configuration
    #[serde(default)]
    pub fix_with_claude_code: FixWithClaudeConfig,
}

fn default_ide_command() -> String {
    "code".to_string() // Default to VS Code
}

fn default_temp_dir() -> String {
    env::temp_dir()
        .join("gh-pr-lander")
        .to_string_lossy()
        .to_string()
}

fn default_approval_message() -> String {
    ":rocket: thanks for your contribution".to_string()
}

fn default_comment_message() -> String {
    String::new() // Empty default - user must enter comment
}

fn default_request_changes_message() -> String {
    "Please address the following concerns:".to_string()
}

fn default_close_message() -> String {
    "Closing this PR.".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ide_command: default_ide_command(),
            temp_dir: default_temp_dir(),
            approval_message: default_approval_message(),
            comment_message: default_comment_message(),
            request_changes_message: default_request_changes_message(),
            close_message: default_close_message(),
            issue_tracker: Vec::new(),
            fix_with_claude_code: FixWithClaudeConfig::default(),
        }
    }
}

impl AppConfig {
    /// Load config. With `Some(path)`, reads that exact file (no fallback to
    /// the global one); with `None`, reads the global config. Falls back to
    /// `Self::default()` when the file is missing or unparseable.
    pub fn load(override_path: Option<&Path>) -> Self {
        if let Some(content) = crate::load_config_file(override_path) {
            match toml::from_str(&content) {
                Ok(config) => {
                    log::info!("Loaded app config from file");
                    return config;
                }
                Err(e) => {
                    log::warn!("Failed to parse config file: {e}");
                }
            }
        }

        log::debug!("Using default app config");
        Self::default()
    }

    /// Generate default config as TOML string with documentation comments
    pub fn generate_default_toml() -> String {
        let config = Self::default();
        let mut output = String::new();

        output.push_str("# gh-pr-lander configuration\n");
        output.push_str("# https://github.com/sassman/gh-pr-lander\n\n");

        output.push_str("# IDE command for opening PRs\n");
        output.push_str(&format!("ide_command = \"{}\"\n\n", config.ide_command));

        output.push_str("# Temporary directory for PR checkouts\n");
        output.push_str(&format!("temp_dir = \"{}\"\n\n", config.temp_dir));

        output.push_str("# Default message for PR approvals\n");
        output.push_str(&format!(
            "approval_message = \"{}\"\n\n",
            config.approval_message
        ));

        output.push_str("# Default message for PR comments (empty = prompt user)\n");
        output.push_str(&format!(
            "comment_message = \"{}\"\n\n",
            config.comment_message
        ));

        output.push_str("# Default message for requesting changes\n");
        output.push_str(&format!(
            "request_changes_message = \"{}\"\n\n",
            config.request_changes_message
        ));

        output.push_str("# Default message for closing PRs\n");
        output.push_str(&format!("close_message = \"{}\"\n\n", config.close_message));

        // Append the fix-with-claude section from the crate
        output.push('\n');
        output.push_str(&FixWithClaudeConfig::generate_toml_section());

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.ide_command, "code");
        assert!(!config.temp_dir.is_empty());
        assert!(!config.approval_message.is_empty());
        assert!(config.comment_message.is_empty()); // Empty default
        assert!(!config.request_changes_message.is_empty());
        assert!(!config.close_message.is_empty());
    }

    #[test]
    fn test_config_deserialize() {
        let toml = r#"
            ide_command = "zed"
            approval_message = "LGTM!"
        "#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.ide_command, "zed");
        assert_eq!(config.approval_message, "LGTM!");
        // temp_dir should use default
        assert!(!config.temp_dir.is_empty());
    }

    #[test]
    fn test_config_deserialize_partial() {
        let toml = r#"
            ide_command = "cursor"
        "#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.ide_command, "cursor");
        // Other fields should use defaults
        assert!(!config.temp_dir.is_empty());
        assert_eq!(
            config.approval_message,
            ":rocket: thanks for your contribution"
        );
    }

    #[test]
    fn test_issue_tracker_config_parsing() {
        // In TOML, backslash needs escaping: \d becomes \\d in the file
        // In raw string literals, backslash is literal
        let toml = r##"
ide_command = "zed"

[[issue_tracker]]
name = "GitHub"
pattern = "#(\\d+)"
url = "https://$HOST/$ORG/$REPO/issues/$ISSUE_NO"
        "##;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.issue_tracker.len(), 1);
        assert_eq!(config.issue_tracker[0].name, "GitHub");
        // After TOML parsing, \\d becomes \d
        assert_eq!(config.issue_tracker[0].pattern, r"#(\d+)");
        assert_eq!(
            config.issue_tracker[0].url,
            "https://$HOST/$ORG/$REPO/issues/$ISSUE_NO"
        );
    }

    #[test]
    fn test_issue_tracker_with_repos_filter() {
        let toml = r##"
[[issue_tracker]]
name = "Jira"
pattern = "PROJ-\\d+"
url = "https://jira.example.com/browse/$ISSUE_NO"
repos = ["my-org/*", "other-org/repo"]
        "##;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.issue_tracker.len(), 1);
        assert_eq!(config.issue_tracker[0].repos.len(), 2);
        assert_eq!(config.issue_tracker[0].repos[0], "my-org/*");
    }

    #[test]
    fn test_load_with_override_path_reads_that_file() {
        let path = env::temp_dir().join(format!(
            "gh-pr-config-override-{}-{}.toml",
            std::process::id(),
            line!()
        ));
        std::fs::write(&path, r#"ide_command = "nvim""#).unwrap();

        let config = AppConfig::load(Some(&path));

        let _ = std::fs::remove_file(&path);
        assert_eq!(config.ide_command, "nvim");
    }

    #[test]
    fn test_load_with_missing_override_falls_back_to_defaults() {
        let path = env::temp_dir().join(format!(
            "gh-pr-config-missing-{}-{}.toml",
            std::process::id(),
            line!()
        ));
        assert!(!path.exists());

        let config = AppConfig::load(Some(&path));

        // Defaults — not whatever the global config might happen to contain.
        assert_eq!(config.ide_command, "code");
    }

    #[test]
    fn test_fix_with_claude_config_from_toml() {
        let toml = r#"
ide_command = "zed"

[fix_with_claude_code]
prompt = "Fix PR #{pr_number}: {pr_title}"

[fix_with_claude_code.permissions]
allow = ["Bash(git *)", "Read", "Write"]
deny = ["Bash(rm *)"]
        "#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            config.fix_with_claude_code.prompt,
            "Fix PR #{pr_number}: {pr_title}"
        );
        assert!(!config.fix_with_claude_code.permissions.is_unrestricted());
        assert_eq!(
            config
                .fix_with_claude_code
                .permissions
                .tools_allowed()
                .len(),
            3
        );
        assert_eq!(
            config.fix_with_claude_code.permissions.tools_denied().len(),
            1
        );
    }

    #[test]
    fn test_fix_with_claude_default_when_omitted() {
        let toml = r#"
ide_command = "zed"
        "#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        // Should use defaults
        assert!(config
            .fix_with_claude_code
            .prompt
            .contains("analyze and fix"));
        assert!(config.fix_with_claude_code.permissions.is_unrestricted());
    }

    #[test]
    fn test_generate_default_toml() {
        let toml = AppConfig::generate_default_toml();
        assert!(toml.contains("# gh-pr-lander configuration"));
        assert!(toml.contains("ide_command"));
        assert!(toml.contains("[fix_with_claude_code]"));
        assert!(toml.contains("multiplexer"));
        assert!(toml.contains("prompt"));
        assert!(toml.contains("[fix_with_claude_code.permissions]"));
    }

    #[test]
    fn test_fix_with_claude_multiplexer_from_toml() {
        let toml = r#"
ide_command = "zed"

[fix_with_claude_code]
multiplexer = "tmux"
prompt = "Fix PR #{pr_number}: {pr_title}"
    "#;
        let config: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(
            config.fix_with_claude_code.multiplexer,
            gh_pr_fix_with_claude::Multiplexer::Tmux
        );
    }
}
