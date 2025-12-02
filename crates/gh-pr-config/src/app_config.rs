//! Application configuration
//!
//! Configuration loaded from gh-pr-tui.toml file.

use serde::{Deserialize, Serialize};
use std::env;

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

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ide_command: default_ide_command(),
            temp_dir: default_temp_dir(),
            approval_message: default_approval_message(),
        }
    }
}

impl AppConfig {
    /// Load config from CWD first, then home directory, or use defaults
    pub fn load() -> Self {
        if let Some(content) = crate::load_config_file() {
            match toml::from_str(&content) {
                Ok(config) => {
                    log::info!("Loaded app config from file");
                    return config;
                }
                Err(e) => {
                    log::warn!("Failed to parse config file: {}", e);
                }
            }
        }

        log::debug!("Using default app config");
        Self::default()
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
}
