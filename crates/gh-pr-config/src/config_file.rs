use crate::paths;
use std::path::Path;

/// Load config TOML. With `Some(path)`, reads that exact file (no fallback);
/// with `None`, reads the global `app_config_path()`.
pub fn load_config_file(override_path: Option<&Path>) -> Option<String> {
    let config_path = match override_path {
        Some(p) => p.to_path_buf(),
        None => paths::app_config_path().ok()?,
    };
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            log::debug!("Loaded config from {}", config_path.display());
            Some(content)
        }
        Err(e) => {
            if override_path.is_some() {
                log::warn!(
                    "Failed to read config file {}: {} \u{2014} falling back to defaults",
                    config_path.display(),
                    e
                );
            }
            None
        }
    }
}
