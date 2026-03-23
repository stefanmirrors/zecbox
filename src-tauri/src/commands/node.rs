use tauri::{AppHandle, State};

use crate::process::zebrad;
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
