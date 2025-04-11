use anyhow::Result;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::ThemeColors;
use crate::DARK_MODE_COLORS;
use crate::LIGHT_MODE_COLORS;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ThothConfig {
    pub theme: ThemeMode,
}

impl ThothConfig {
    pub fn load() -> Result<Self> {
        let config_path = get_config_path();

        if !config_path.exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: ThothConfig = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = get_config_path();

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let config_str = toml::to_string(self)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }

    pub fn set_theme(&mut self, theme: ThemeMode) -> Result<()> {
        self.theme = theme;
        self.save()?;
        Ok(())
    }

    pub fn get_theme_colors(&self) -> &'static ThemeColors {
        match self.theme {
            ThemeMode::Light => &LIGHT_MODE_COLORS,
            ThemeMode::Dark => &DARK_MODE_COLORS,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    let mut path = home_dir().unwrap_or_default();
    path.push(".config");
    path.push("thoth");
    path.push("config.toml");
    path
}
