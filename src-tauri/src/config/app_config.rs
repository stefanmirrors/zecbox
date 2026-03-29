use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state::PrivacyMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default = "AppConfig::serde_default")]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub first_run_complete: bool,
    pub privacy_mode: PrivacyMode,
    pub wallet_server: bool,
    pub auto_start: bool,
    pub serve_network: bool,
    /// Legacy field for backward compatibility with configs that have shield_mode: true.
    /// Migrated to privacy_mode on load.
    #[serde(default)]
    pub shield_mode: bool,
}

impl AppConfig {
    pub fn default_for(default_data_dir: &Path) -> Self {
        Self {
            data_dir: default_data_dir.to_path_buf(),
            first_run_complete: false,
            privacy_mode: PrivacyMode::Standard,
            wallet_server: false,
            auto_start: false,
            serve_network: false,
            shield_mode: false,
        }
    }

    fn serde_default() -> Self {
        Self {
            data_dir: PathBuf::new(),
            first_run_complete: false,
            privacy_mode: PrivacyMode::Standard,
            wallet_server: false,
            auto_start: false,
            serve_network: false,
            shield_mode: false,
        }
    }

    pub fn config_path(default_data_dir: &Path) -> PathBuf {
        default_data_dir.join("config").join("zecbox.json")
    }

    pub fn load(default_data_dir: &Path) -> Result<Self, String> {
        let path = Self::config_path(default_data_dir);
        if !path.exists() {
            return Ok(Self::default_for(default_data_dir));
        }
        let contents =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {}", e))?;
        let mut config: Self =
            serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))?;

        // Migrate legacy shield_mode boolean to privacy_mode enum
        if config.shield_mode && config.privacy_mode == PrivacyMode::Standard {
            config.privacy_mode = PrivacyMode::Stealth;
            config.shield_mode = false;
        }

        Ok(config)
    }

    pub fn save(&self, default_data_dir: &Path) -> Result<(), String> {
        let path = Self::config_path(default_data_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &contents)
            .map_err(|e| format!("Failed to write config: {}", e))?;
        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename config: {}", e))?;
        Ok(())
    }

    /// Returns true if stealth (Tor) is active in the current privacy mode.
    pub fn is_stealth_active(&self) -> bool {
        matches!(self.privacy_mode, PrivacyMode::Stealth | PrivacyMode::Shield)
    }

    /// Returns true if proxy (VPS relay) is active in the current privacy mode.
    pub fn is_proxy_active(&self) -> bool {
        matches!(self.privacy_mode, PrivacyMode::Proxy | PrivacyMode::Shield)
    }
}
