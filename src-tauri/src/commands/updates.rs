//! Commands for app and binary update management.

use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

use crate::state::{AppState, BinaryUpdateInfo, UpdateStatus, VersionInfo};
use crate::updates::{self, BinaryVersions};

#[tauri::command]
pub async fn get_versions(state: State<'_, AppState>) -> Result<VersionInfo, String> {
    let data_dir = state.node.data_dir.lock().await.clone();
    let versions = BinaryVersions::load(&data_dir);

    Ok(VersionInfo {
        app: env!("CARGO_PKG_VERSION").to_string(),
        zebrad: versions.zebrad,
        zaino: versions.zaino,
        arti: versions.arti,
    })
}

#[tauri::command]
pub async fn get_update_status(
    state: State<'_, AppState>,
) -> Result<UpdateStatus, String> {
    let status = state.update.status.lock().await;
    Ok(status.clone())
}

#[tauri::command]
pub async fn check_for_updates(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<BinaryUpdateInfo>, String> {
    let data_dir = state.node.data_dir.lock().await.clone();

    updates::emit_update_status(&app_handle, &state.update, UpdateStatus::Checking).await;

    match updates::check_manifest(&data_dir, &app_handle).await {
        Ok(available) => {
            let has_updates = !available.is_empty();
            let mut stored = state.update.available_updates.lock().await;
            *stored = available.clone();
            drop(stored);

            let new_status = if has_updates {
                UpdateStatus::UpdateAvailable
            } else {
                UpdateStatus::Idle
            };
            updates::emit_update_status(&app_handle, &state.update, new_status).await;

            Ok(available)
        }
        Err(e) => {
            updates::emit_update_status(
                &app_handle,
                &state.update,
                UpdateStatus::Error {
                    message: e.clone(),
                },
            )
            .await;
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn apply_update(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let update_info = {
        let available = state.update.available_updates.lock().await;
        available
            .iter()
            .find(|u| u.name == name)
            .cloned()
            .ok_or_else(|| format!("No update found for {}", name))?
    };

    updates::apply_binary_update(app_handle.clone(), &update_info, &state).await?;

    // Remove applied update from available list
    {
        let mut available = state.update.available_updates.lock().await;
        available.retain(|u| u.name != name);
        if available.is_empty() {
            updates::emit_update_status(&app_handle, &state.update, UpdateStatus::Complete).await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn apply_all_updates(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let all_updates: Vec<BinaryUpdateInfo> = {
        let available = state.update.available_updates.lock().await;
        available.clone()
    };

    if all_updates.is_empty() {
        return Err("No updates available".into());
    }

    for update_info in &all_updates {
        updates::apply_binary_update(app_handle.clone(), update_info, &state).await?;
    }

    {
        let mut available = state.update.available_updates.lock().await;
        available.clear();
    }

    updates::emit_update_status(&app_handle, &state.update, UpdateStatus::Complete).await;

    Ok(())
}

#[tauri::command]
pub async fn dismiss_updates(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut available = state.update.available_updates.lock().await;
    available.clear();
    drop(available);

    updates::emit_update_status(&app_handle, &state.update, UpdateStatus::Idle).await;

    Ok(())
}

#[tauri::command]
pub async fn check_app_update(app_handle: AppHandle) -> Result<bool, String> {
    let updater = app_handle
        .updater_builder()
        .build()
        .map_err(|e| format!("Failed to initialize updater: {}", e))?;

    match updater.check().await {
        Ok(Some(update)) => {
            log::info!(
                "App update available: {} -> {}",
                env!("CARGO_PKG_VERSION"),
                update.version
            );
            Ok(true)
        }
        Ok(None) => {
            log::debug!("No app update available");
            Ok(false)
        }
        Err(e) => {
            log::warn!("App update check failed: {}", e);
            Err(format!("Failed to check for app updates: {}", e))
        }
    }
}
