use tauri::State;

use crate::config::app_config::AppConfig;
use crate::power;
use crate::state::AppState;

#[tauri::command]
pub async fn get_auto_start_enabled() -> Result<bool, String> {
    Ok(power::is_launch_agent_installed())
}

#[tauri::command]
pub async fn set_auto_start(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if enabled {
        power::install_launch_agent()?;
    } else {
        power::remove_launch_agent()?;
    }

    // Persist to app config
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.auto_start = enabled;
    config.save(&state.default_data_dir)?;

    Ok(())
}
