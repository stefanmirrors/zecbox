use tauri::{AppHandle, State};

use crate::process::{zebrad, zaino};
use crate::state::{AppState, NodeStatus};

#[tauri::command]
pub async fn start_node(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Check if paused due to low disk space
    {
        let paused = state.storage.paused_low_space.lock().await;
        if *paused {
            return Err(
                "Node is paused: disk space critically low (<2GB). Free up space to continue."
                    .into(),
            );
        }
    }

    // Check if data drive is disconnected
    {
        let connected = state.storage.drive_connected.lock().await;
        if !*connected {
            return Err("Data drive is disconnected. Reconnect the drive to continue.".into());
        }
    }

    zebrad::start_zebrad(app_handle, &state.node).await
}

#[tauri::command]
pub async fn stop_node(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    zebrad::stop_zebrad(&app_handle, &state.node).await
}

#[tauri::command]
pub async fn get_node_status(
    state: State<'_, AppState>,
) -> Result<NodeStatus, String> {
    let status = state.node.status.lock().await;
    Ok(status.clone())
}

#[tauri::command]
pub async fn rebuild_database(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Stop zebrad if running
    let node_status = state.node.status.lock().await.clone();
    if !matches!(node_status, NodeStatus::Stopped | NodeStatus::Error { .. }) {
        zebrad::stop_zebrad(&app_handle, &state.node).await?;
    }

    // Stop Zaino if running
    let data_dir = state.node.data_dir.lock().await.clone();
    if !state.wallet.status.lock().await.is_stopped_or_error() {
        let _ = zaino::stop_zaino(&app_handle, &state.wallet, &data_dir).await;
    }

    // Delete the zebra chain data directory
    let zebra_dir = data_dir.join("zebra");
    if zebra_dir.exists() {
        std::fs::remove_dir_all(&zebra_dir)
            .map_err(|e| format!("Failed to delete chain data: {}", e))?;
        log::info!("Deleted chain data directory: {:?}", zebra_dir);
    }

    // Reset backoff state so node can start fresh
    {
        let mut backoff = state.node.backoff.lock().await;
        backoff.reset();
    }

    Ok(())
}
