//! Commands for Shield Mode (Tor) toggle.

use tauri::{AppHandle, State};

use crate::config::app_config::AppConfig;
use crate::process::zebrad;
use crate::state::{AppState, NetworkServeStatus, ShieldStatus};
use crate::tor;
use crate::tor::firewall;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShieldStatusInfo {
    pub enabled: bool,
    pub status: String,
    pub bootstrap_progress: Option<u8>,
    pub message: Option<String>,
}

impl From<&ShieldStatus> for ShieldStatusInfo {
    fn from(status: &ShieldStatus) -> Self {
        match status {
            ShieldStatus::Disabled => ShieldStatusInfo {
                enabled: false,
                status: "disabled".into(),
                bootstrap_progress: None,
                message: None,
            },
            ShieldStatus::Bootstrapping { progress } => ShieldStatusInfo {
                enabled: false,
                status: "bootstrapping".into(),
                bootstrap_progress: Some(*progress),
                message: None,
            },
            ShieldStatus::Active => ShieldStatusInfo {
                enabled: true,
                status: "active".into(),
                bootstrap_progress: None,
                message: None,
            },
            ShieldStatus::Error { message } => ShieldStatusInfo {
                enabled: false,
                status: "error".into(),
                bootstrap_progress: None,
                message: Some(message.clone()),
            },
            ShieldStatus::Interrupted => ShieldStatusInfo {
                enabled: false,
                status: "interrupted".into(),
                bootstrap_progress: None,
                message: Some(
                    "Tor proxy stopped unexpectedly. Node stopped to prevent clearnet exposure."
                        .into(),
                ),
            },
        }
    }
}

#[tauri::command]
pub async fn get_shield_status(
    state: State<'_, AppState>,
) -> Result<ShieldStatusInfo, String> {
    let status = state.shield.status.lock().await;
    Ok(ShieldStatusInfo::from(&*status))
}

#[tauri::command]
pub async fn enable_shield_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Check if network serving is active
    {
        let net_status = state.network.status.lock().await;
        if matches!(*net_status, NetworkServeStatus::Active { .. }) {
            return Err("Disable Serve the Network first. Shield Mode cannot be enabled while accepting inbound connections.".into());
        }
    }

    // Check if firewall helper is installed
    if !firewall::is_helper_installed() {
        return Err("Firewall helper not installed. Install it first to enable Shield Mode.".into());
    }

    let node_was_running = {
        let status = state.node.status.lock().await;
        matches!(
            *status,
            crate::state::NodeStatus::Running { .. }
        )
    };

    // Start Arti SOCKS proxy
    tor::start_arti(app_handle.clone(), &state.shield).await?;

    // Enable PF firewall rules + transparent redirector
    firewall::enable_firewall()
        .map_err(|e| format!("Failed to enable firewall: {}", e))?;

    // If node was running, restart it with shield config
    if node_was_running {
        zebrad::stop_zebrad(&app_handle, &state.node).await?;
        zebrad::start_zebrad(app_handle.clone(), &state.node).await?;
    }

    // Persist shield_mode setting
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.shield_mode = true;
    config.save(&state.default_data_dir)?;

    log::info!("Shield Mode enabled (PF firewall active)");
    Ok(())
}

#[tauri::command]
pub async fn disable_shield_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let node_was_running = {
        let status = state.node.status.lock().await;
        matches!(
            *status,
            crate::state::NodeStatus::Running { .. }
        )
    };

    // Stop node first (before disabling Arti) to avoid any clearnet window
    if node_was_running {
        zebrad::stop_zebrad(&app_handle, &state.node).await?;
    }

    // Disable PF firewall rules + stop redirector
    if let Err(e) = firewall::disable_firewall() {
        log::error!("Failed to disable firewall: {}", e);
    }

    // Stop Arti
    tor::stop_arti(&app_handle, &state.shield).await?;

    // Restart node with clearnet config if it was running
    if node_was_running {
        zebrad::start_zebrad(app_handle.clone(), &state.node).await?;
    }

    // Persist shield_mode setting
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.shield_mode = false;
    config.save(&state.default_data_dir)?;

    log::info!("Shield Mode disabled (PF firewall removed)");
    Ok(())
}

#[tauri::command]
pub async fn install_firewall_helper(
    app_handle: AppHandle,
) -> Result<(), String> {
    firewall::install_helper(&app_handle)
}

#[tauri::command]
pub async fn is_firewall_helper_installed() -> Result<bool, String> {
    Ok(firewall::is_helper_installed())
}
