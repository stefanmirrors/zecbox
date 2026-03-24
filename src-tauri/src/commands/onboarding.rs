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
) -> Result<(), String> {
    // Set up data directory (validates, creates subdirs, updates state, persists config)
    apply_data_dir(&state.node, &state.storage, &state.default_data_dir, &path).await?;

    // Mark onboarding complete
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.first_run_complete = true;
    config.save(&state.default_data_dir)?;

    // Start zebrad
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
