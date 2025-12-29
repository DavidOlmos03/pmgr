use crate::ui::Theme;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: Theme,
    // Future: keybindings, layout preferences, etc.
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Default,
        }
    }
}

/// Get the path to the settings file
fn settings_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("pmgr");

    // Create config directory if it doesn't exist
    fs::create_dir_all(&config_dir)?;

    Ok(config_dir.join("settings.json"))
}

/// Load settings from disk
/// Falls back to default settings if file doesn't exist or is invalid
pub fn load_settings() -> Settings {
    match settings_path() {
        Ok(path) => {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(settings) = serde_json::from_str(&content) {
                        return settings;
                    }
                }
            }
        }
        Err(_) => {}
    }

    // Return default settings if anything fails
    Settings::default()
}

/// Save settings to disk
pub fn save_settings(settings: &Settings) -> Result<()> {
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(settings)?;
    fs::write(path, json)?;
    Ok(())
}
