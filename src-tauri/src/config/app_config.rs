use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default = "AppConfig::serde_default")]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub first_run_complete: bool,
    pub shield_mode: bool,
    pub wallet_server: bool,
    pub auto_start: bool,
    pub serve_network: bool,
}

impl AppConfig {
    pub fn default_for(default_data_dir: &Path) -> Self {
        Self {
            data_dir: default_data_dir.to_path_buf(),
            first_run_complete: false,
            shield_mode: false,
            wallet_server: false,
            auto_start: false,
            serve_network: false,
        }
    }

    fn serde_default() -> Self {
        Self {
            data_dir: PathBuf::new(),
            first_run_complete: false,
            shield_mode: false,
            wallet_server: false,
            auto_start: false,
            serve_network: false,
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
        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))
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
}
