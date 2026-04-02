//! Commands for Shield Mode (Tor + hidden service) toggle.

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
    pub onion_address: Option<String>,
}

impl ShieldStatusInfo {
    pub async fn from_state(state: &crate::state::ShieldState) -> Self {
        let status = state.status.lock().await;
        let onion = state.onion_address.lock().await.clone();
        match &*status {
            ShieldStatus::Disabled => ShieldStatusInfo {
                enabled: false,
                status: "disabled".into(),
                bootstrap_progress: None,
                message: None,
                onion_address: None,
            },
            ShieldStatus::Bootstrapping { progress } => ShieldStatusInfo {
                enabled: false,
                status: "bootstrapping".into(),
                bootstrap_progress: Some(*progress),
                message: None,
                onion_address: None,
            },
            ShieldStatus::Active => ShieldStatusInfo {
                enabled: true,
                status: "active".into(),
                bootstrap_progress: None,
                message: None,
                onion_address: onion,
            },
            ShieldStatus::Error { message } => ShieldStatusInfo {
                enabled: false,
                status: "error".into(),
                bootstrap_progress: None,
                message: Some(message.clone()),
                onion_address: None,
            },
            ShieldStatus::Interrupted => ShieldStatusInfo {
                enabled: false,
                status: "interrupted".into(),
                bootstrap_progress: None,
                message: Some(
                    "Tor proxy stopped unexpectedly. Node stopped to prevent clearnet exposure."
                        .into(),
                ),
                onion_address: None,
            },
        }
    }
}

#[tauri::command]
pub async fn get_shield_status(
    state: State<'_, AppState>,
) -> Result<ShieldStatusInfo, String> {
    Ok(ShieldStatusInfo::from_state(&state.shield).await)
}

#[tauri::command]
pub async fn get_onion_address(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let addr = state.shield.onion_address.lock().await;
    Ok(addr.clone())
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
            return Err("Disable Serve the Network first. Shield Mode cannot be enabled while accepting inbound connections via UPnP.".into());
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

    // Start Arti SOCKS proxy + hidden service
    tor::start_arti(app_handle.clone(), &state.shield).await?;

    // Enable PF firewall rules + transparent redirector (10s timeout)
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking(|| firewall::enable_firewall())
    )
    .await
    .map_err(|_| "Firewall helper did not respond within 10 seconds. Try reinstalling the helper.".to_string())?
    .map_err(|e| format!("Firewall task failed: {}", e))?
    .map_err(|e| format!("Failed to enable firewall: {}", e))?;

    // Verify traffic actually routes through Tor (15s timeout)
    if let Err(e) = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        tor::verify_tor_path()
    ).await.map_err(|_| "Tor path verification timed out".to_string())? {
        log::error!("Traffic verification failed: {}", e);
        let _ = firewall::disable_firewall();
        let _ = tor::stop_arti(&app_handle, &state.shield).await;
        return Err(format!("Shield Mode failed traffic verification: {}. Disabled for safety.", e));
    }

    // Resolve Zcash DNS seeders through Tor to prevent DNS leaks (45s timeout).
    // zebrad.toml will contain IPs only — no DNS hostnames that could leak to ISP.
    let resolved_peers = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        tor::dns::resolve_seeders_via_tor()
    )
    .await
    .map_err(|_| {
        log::error!("DNS resolution through Tor timed out after 45s");
        let app = app_handle.clone();
        let shield = state.shield.clone();
        tokio::spawn(async move {
            let _ = firewall::disable_firewall();
            let _ = tor::stop_arti(&app, &shield).await;
        });
        "DNS resolution through Tor timed out. Check your network connection.".to_string()
    })?
    .map_err(|e| {
        log::error!("DNS resolution through Tor failed: {}", e);
        let app = app_handle.clone();
        let shield = state.shield.clone();
        tokio::spawn(async move {
            let _ = firewall::disable_firewall();
            let _ = tor::stop_arti(&app, &shield).await;
        });
        format!("Shield Mode failed: {}. Disabled for safety.", e)
    })?;

    // Store resolved peers for zebrad config generation
    {
        let mut peers = state.shield.resolved_peers.lock().await;
        *peers = Some(resolved_peers);
    }

    // If node was running, restart it with shield config (resolved IPs + onion external_addr)
    if node_was_running {
        zebrad::stop_zebrad(&app_handle, &state.node).await?;
        zebrad::start_zebrad(app_handle.clone(), &state.node).await?;
    }

    // Persist shield_mode setting
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.shield_mode = true;
    config.save(&state.default_data_dir)?;

    log::info!("Shield Mode enabled (PF firewall + hidden service active)");
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

    // Clear onion address and resolved peers
    {
        let mut addr = state.shield.onion_address.lock().await;
        *addr = None;
    }
    {
        let mut peers = state.shield.resolved_peers.lock().await;
        *peers = None;
    }

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

#[tauri::command]
pub async fn is_shield_supported() -> Result<bool, String> {
    Ok(cfg!(unix))
}
