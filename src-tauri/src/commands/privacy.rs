//! Unified privacy mode commands.

use tauri::{AppHandle, Emitter, State};

use crate::config::app_config::AppConfig;
use crate::state::{AppState, PrivacyMode};

#[tauri::command]
pub async fn get_privacy_mode(
    state: State<'_, AppState>,
) -> Result<PrivacyMode, String> {
    let config = AppConfig::load(&state.default_data_dir)?;
    Ok(config.privacy_mode)
}

#[tauri::command]
pub async fn set_privacy_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    mode: PrivacyMode,
) -> Result<(), String> {
    let current = {
        let config = AppConfig::load(&state.default_data_dir)?;
        config.privacy_mode
    };

    if current == mode {
        return Ok(());
    }

    // Persist the new mode
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.privacy_mode = mode.clone();
    config.save(&state.default_data_dir)?;

    let _ = app_handle.emit("privacy_mode_changed", &mode);

    log::info!("Privacy mode changed to {:?}", mode);
    Ok(())
}
