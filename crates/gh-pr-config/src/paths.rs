//! Config and data directory paths.
//!
//! Uses `dirs-lite` with `favor-xdg-style`: macOS gets XDG paths
//! (`~/.config`, `~/.cache`) instead of `~/Library/...`.

use anyhow::{Context, Result};
use std::path::PathBuf;

const APP_NAME: &str = "gh-pr-lander";
const LOCAL_SESSION_FILE: &str = ".gh-pr-lander.session.toml";

pub fn config_dir() -> Result<PathBuf> {
    let base = dirs_lite::config_dir().context("Could not determine config directory")?;
    let dir = base.join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs_lite::cache_dir().context("Could not determine cache directory")?;
    let dir = base.join(APP_NAME);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn global_session_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("session.toml"))
}

/// Session file in the current working directory.
pub fn local_session_path() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join(LOCAL_SESSION_FILE))
}

pub fn has_local_session() -> bool {
    local_session_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn recent_repositories_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("recent-repositories.toml"))
}

pub fn api_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("gh-api-cache.json"))
}

pub fn app_config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_exists() {
        let dir = config_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.ends_with(APP_NAME));
    }

    #[test]
    fn test_cache_dir_exists() {
        let dir = cache_dir().unwrap();
        assert!(dir.exists());
        assert!(dir.ends_with(APP_NAME));
    }

    #[test]
    fn test_session_paths() {
        let global = global_session_path().unwrap();
        assert!(global.ends_with("session.toml"));

        let local = local_session_path().unwrap();
        assert!(local.ends_with(LOCAL_SESSION_FILE));
    }
}
