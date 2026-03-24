use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn get_logs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let buffer = state.node.log_buffer.lock().await;
    Ok(buffer.iter().cloned().collect())
}
