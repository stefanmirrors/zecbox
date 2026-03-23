use tauri::{AppHandle, State};

use crate::process::zebrad;
use crate::state::{AppState, NodeStatus};

#[tauri::command]
pub async fn start_node(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
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
