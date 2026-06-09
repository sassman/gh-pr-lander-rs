use crate::paths;

/// Load `config.toml` from the app config dir, if present.
pub fn load_config_file() -> Option<String> {
    let config_path = paths::app_config_path().ok()?;
    match std::fs::read_to_string(&config_path) {
        Ok(content) => {
            log::debug!("Loaded config from {}", config_path.display());
            Some(content)
        }
        Err(_) => None,
    }
}
