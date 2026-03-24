use std::path::{Path, PathBuf};

use tauri::State;

use crate::config::app_config::AppConfig;
use crate::state::{AppState, NodeState, StorageInfo, StorageState, VolumeInfo};
use crate::storage;

/// Shared logic for setting up a data directory: validates, creates subdirs, updates state.
/// Returns the resolved zecbox-data path.
pub async fn apply_data_dir(
    node: &NodeState,
    storage_state: &StorageState,
    default_data_dir: &Path,
    path: &str,
) -> Result<PathBuf, String> {
    let new_path = PathBuf::from(path);

    if !new_path.exists() || !new_path.is_dir() {
        return Err("Selected path does not exist or is not a directory".into());
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
        let mut data_dir = node.data_dir.lock().await;
        *data_dir = zecbox_dir.clone();
    }

    // Update drive_connected status based on new path
    {
        let mut connected = storage_state.drive_connected.lock().await;
        *connected = storage::is_mount_available(&zecbox_dir);
    }

    // Persist to zecbox.json
    let mut config = AppConfig::load(default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(default_data_dir));
    config.data_dir = zecbox_dir.clone();
    config.save(default_data_dir)?;

    Ok(zecbox_dir)
}

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
    // Node must be stopped to change data directory
    {
        let status = state.node.status.lock().await;
        if !status.is_stopped_or_error() {
            return Err("Stop the node before changing the data directory".into());
        }
    }

    apply_data_dir(&state.node, &state.storage, &state.default_data_dir, &path).await?;
    Ok(())
}
