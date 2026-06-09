//! Configuration for Claude Code fix sessions
//!
//! This module defines the configuration structure for the "fix with Claude Code"
//! feature. The configuration is embedded in the main app config under
//! `[fix_with_claude_code]` section.

use serde::{Deserialize, Serialize};

/// A tool permission pattern (e.g., "Bash(git add:*)", "Read", "Edit")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool(pub String);

impl Tool {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Tool {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Permissions for Claude Code sessions
///
/// Controls which tools Claude Code can use automatically vs. with prompting.
/// Empty permissions (all lists empty) means unrestricted (uses --dangerously-skip-permissions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Permissions {
    /// Tools Claude can use without asking (--allowedTools)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<Tool>,

    /// Tools that require user confirmation (default behavior)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ask: Vec<Tool>,

    /// Tools Claude cannot use at all (--disallowedTools)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<Tool>,
}

impl Permissions {
    /// Create unrestricted permissions (empty - uses --dangerously-skip-permissions)
    pub fn unrestricted() -> Self {
        Self::default()
    }

    /// Check if permissions are unrestricted (all lists empty)
    pub fn is_unrestricted(&self) -> bool {
        self.allow.is_empty() && self.ask.is_empty() && self.deny.is_empty()
    }

    /// Get the list of allowed tools
    pub fn tools_allowed(&self) -> &[Tool] {
        &self.allow
    }

    /// Get the list of tools that require user confirmation
    pub fn tools_ask(&self) -> &[Tool] {
        &self.ask
    }

    /// Get the list of denied tools
    pub fn tools_denied(&self) -> &[Tool] {
        &self.deny
    }
}

const DEFAULT_PROMPT: &str = r#"Please analyze and fix the GitHub PR.

Steps:
1. Check the PR description and review comments to understand what needs to be fixed
2. Check CI build logs for errors
3. Implement the fix
4. Commit the changes (don't push)

Notes: PR#: {pr_number} and PR Title: {pr_title}
"#;

/// Terminal multiplexer backend for Claude Code sessions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Multiplexer {
    Tmux,
    #[default]
    Zellij,
}

/// Configuration for "Fix with Claude Code" feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixWithClaudeConfig {
    /// The prompt template sent to Claude Code
    /// Use {pr_number} and {pr_title} as placeholders
    #[serde(default = "default_prompt")]
    pub prompt: String,

    /// Permissions for Claude Code session
    #[serde(default)]
    pub permissions: Permissions,

    /// Terminal multiplexer to use (tmux or zellij)
    #[serde(default)]
    pub multiplexer: Multiplexer,
}

fn default_prompt() -> String {
    DEFAULT_PROMPT.to_string()
}

impl Default for FixWithClaudeConfig {
    fn default() -> Self {
        Self {
            prompt: default_prompt(),
            permissions: Permissions::default(),
            multiplexer: Multiplexer::default(),
        }
    }
}

impl FixWithClaudeConfig {
    /// Build the final prompt with PR context substituted
    pub fn build_prompt(&self, pr_number: usize, pr_title: &str) -> String {
        self.prompt
            .replace("{pr_number}", &pr_number.to_string())
            .replace("{pr_title}", pr_title)
    }

    /// Generate TOML section with documentation comments
    pub fn generate_toml_section() -> String {
        let mut output = String::new();

        output.push_str("# Fix with Claude Code configuration\n");
        output.push_str("[fix_with_claude_code]\n");
        output.push_str("# Terminal multiplexer: \"zellij\" (default) or \"tmux\"\n");
        output.push_str("multiplexer = \"zellij\"\n");
        output.push_str("# The prompt template sent to Claude Code\n");
        output.push_str("# Use {pr_number} and {pr_title} as placeholders\n");
        output.push_str("prompt = '''\n");
        output.push_str(DEFAULT_PROMPT);
        output.push_str("\n'''\n\n");
        output.push_str("# Permissions control which tools Claude can use\n");
        output.push_str("# Empty = unrestricted (--dangerously-skip-permissions)\n");
        output.push_str("[fix_with_claude_code.permissions]\n");
        output.push_str("# allow = [\"Read\", \"Glob\", \"Grep\", \"Bash(git *)\"]  # Auto-approve these tools\n");
        output.push_str("# ask = [\"Write\", \"Edit\"]  # Require confirmation\n");
        output.push_str("# deny = [\"Bash(rm *)\"]  # Never allow\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FixWithClaudeConfig::default();
        assert!(config.prompt.contains("analyze and fix"));
        assert!(config.permissions.is_unrestricted());
    }

    #[test]
    fn test_build_prompt() {
        let config = FixWithClaudeConfig {
            prompt: "Fix PR #{pr_number}: {pr_title}".to_string(),
            permissions: Permissions::default(),
            multiplexer: Multiplexer::default(),
        };
        let prompt = config.build_prompt(123, "Fix login bug");
        assert_eq!(prompt, "Fix PR #123: Fix login bug");
    }

    #[test]
    fn test_permissions_unrestricted_deserialize() {
        // Empty permissions struct means unrestricted (uses --dangerously-skip-permissions)
        let toml = r#"
prompt = "Fix it"
[permissions]
        "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert!(config.permissions.is_unrestricted());
    }

    #[test]
    fn test_permissions_allow_deserialize() {
        let toml = r#"
prompt = "Fix it"
[permissions]
allow = ["Bash(git *)", "Read", "Write"]
        "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert!(!config.permissions.is_unrestricted());
        assert_eq!(config.permissions.tools_allowed().len(), 3);
        assert_eq!(
            config.permissions.tools_allowed()[0].as_str(),
            "Bash(git *)"
        );
    }

    #[test]
    fn test_permissions_deny_deserialize() {
        let toml = r#"
prompt = "Fix it"
[permissions]
deny = ["Bash(rm *)"]
        "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert!(!config.permissions.is_unrestricted());
        assert_eq!(config.permissions.tools_denied().len(), 1);
        assert_eq!(config.permissions.tools_denied()[0].as_str(), "Bash(rm *)");
    }

    #[test]
    fn test_permissions_mixed_deserialize() {
        let toml = r#"
prompt = "Fix it"
[permissions]
allow = ["Read", "Glob", "Grep"]
ask = ["Bash(git commit*)"]
deny = ["Bash(rm *)"]
        "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert!(!config.permissions.is_unrestricted());
        assert_eq!(config.permissions.tools_allowed().len(), 3);
        assert_eq!(config.permissions.tools_ask().len(), 1);
        assert_eq!(config.permissions.tools_denied().len(), 1);
    }

    #[test]
    fn test_generate_toml_section() {
        let toml = FixWithClaudeConfig::generate_toml_section();
        assert!(toml.contains("[fix_with_claude_code]"));
        assert!(toml.contains("multiplexer"));
        assert!(toml.contains("prompt"));
        assert!(toml.contains("[fix_with_claude_code.permissions]"));
    }

    #[test]
    fn test_multiplexer_default_is_zellij() {
        let config = FixWithClaudeConfig::default();
        assert_eq!(config.multiplexer, Multiplexer::Zellij);
    }

    #[test]
    fn test_multiplexer_tmux_from_toml() {
        let toml = r#"
prompt = "Fix it"
multiplexer = "tmux"
    "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.multiplexer, Multiplexer::Tmux);
    }

    #[test]
    fn test_multiplexer_zellij_from_toml() {
        let toml = r#"
prompt = "Fix it"
multiplexer = "zellij"
    "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.multiplexer, Multiplexer::Zellij);
    }

    #[test]
    fn test_multiplexer_default_when_omitted() {
        let toml = r#"
prompt = "Fix it"
    "#;
        let config: FixWithClaudeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.multiplexer, Multiplexer::Zellij);
    }
}
