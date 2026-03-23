use std::path::PathBuf;

use tauri::State;

use crate::config::app_config::AppConfig;
use crate::state::{AppState, StorageInfo, VolumeInfo};
use crate::storage;

#[tauri::command]
pub async fn get_volumes() -> Result<Vec<VolumeInfo>, String> {
    Ok(storage::enumerate_volumes())
}

#[tauri::command]
pub async fn get_storage_info(state: State<'_, AppState>) -> Result<StorageInfo, String> {
    let data_dir = state.node.data_dir.lock().await.clone();
    storage::get_data_dir_storage(&data_dir)
}

#[tauri::command]
pub async fn set_data_dir(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let new_path = PathBuf::from(&path);

    if !new_path.exists() || !new_path.is_dir() {
        return Err("Selected path does not exist or is not a directory".into());
    }

    // Node must be stopped to change data directory
    {
        let status = state.node.status.lock().await;
        if !status.is_stopped_or_error() {
            return Err("Stop the node before changing the data directory".into());
        }
    }

    // Check minimum free space
    let info = storage::get_data_dir_storage(&new_path)?;
    if info.available_bytes < 10_000_000_000 {
        return Err("Selected volume needs at least 10GB of free space".into());
    }

    // Create subdirectory structure
    let zecbox_dir = new_path.join("zecbox-data");
    for sub in &["zebra", "zaino", "config", "logs"] {
        std::fs::create_dir_all(zecbox_dir.join(sub))
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Update NodeState data_dir
    {
        let mut data_dir = state.node.data_dir.lock().await;
        *data_dir = zecbox_dir.clone();
    }

    // Update drive_connected status based on new path
    {
        let mut connected = state.storage.drive_connected.lock().await;
        *connected = storage::is_mount_available(&zecbox_dir);
    }

    // Persist to zecbox.json
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.data_dir = zecbox_dir;
    config.save(&state.default_data_dir)?;

    Ok(())
}
