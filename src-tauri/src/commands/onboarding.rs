use tauri::{AppHandle, State};

use crate::config::app_config::AppConfig;
use crate::process::{zebrad, zaino};
use crate::state::AppState;
use crate::tor;

use super::storage::apply_data_dir;

#[tauri::command]
pub async fn get_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    AppConfig::load(&state.default_data_dir)
}

#[tauri::command]
pub async fn complete_onboarding(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    path: String,
    shield_mode: bool,
) -> Result<(), String> {
    // Set up data directory (validates, creates subdirs, updates state, persists config)
    apply_data_dir(&state.node, &state.storage, &state.default_data_dir, &path).await?;

    // Mark onboarding complete and persist shield choice
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.first_run_complete = true;
    config.shield_mode = shield_mode;
    config.save(&state.default_data_dir)?;

    // If shield mode chosen, start Arti + enable firewall before starting node
    // Skip if already active (e.g. restored on startup)
    if shield_mode && !state.shield.is_active().await {
        tor::start_arti(app_handle.clone(), &state.shield).await?;

        tor::firewall::enable_firewall()
            .map_err(|e| format!("Failed to enable firewall: {}", e))?;

        // Verify traffic actually routes through Tor before starting the node
        if let Err(e) = tor::verify_tor_path().await {
            log::error!("Traffic verification failed during onboarding: {}", e);
            let _ = tor::firewall::disable_firewall();
            let _ = tor::stop_arti(&app_handle, &state.shield).await;
            return Err(format!("Shield Mode failed traffic verification: {}. Try again or select Standard.", e));
        }

        log::info!("Shield Mode enabled during onboarding");
    }

    // Start zebrad (reads shield status to generate correct config)
    zebrad::start_zebrad(app_handle, &state.node).await?;

    Ok(())
}

#[tauri::command]
pub async fn reset_onboarding(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Stop all running processes
    let data_dir = state.node.data_dir.lock().await.clone();
    let _ = zaino::stop_zaino(&app_handle, &state.wallet, &data_dir).await;
    let _ = zebrad::stop_zebrad(&app_handle, &state.node).await;
    let _ = tor::stop_arti(&app_handle, &state.shield).await;

    // Reset config to first-run state
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.first_run_complete = false;
    config.save(&state.default_data_dir)?;

    Ok(())
}
