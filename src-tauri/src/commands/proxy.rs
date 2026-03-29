//! Commands for Proxy Mode (WireGuard VPS relay).

use tauri::{AppHandle, State};

use crate::config::app_config::AppConfig;
use crate::config::proxy_config::{self, ProxyConfig};
use crate::process::{wireguard, zebrad};
use crate::state::{AppState, NetworkServeStatus, PrivacyMode, ProxyStatus};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatusInfo {
    pub enabled: bool,
    pub status: String,
    pub vps_ip: Option<String>,
    pub last_handshake_secs: Option<u64>,
    pub relay_reachable: Option<bool>,
    pub message: Option<String>,
    pub step: Option<String>,
}

impl From<&ProxyStatus> for ProxyStatusInfo {
    fn from(status: &ProxyStatus) -> Self {
        match status {
            ProxyStatus::Disabled => ProxyStatusInfo {
                enabled: false,
                status: "disabled".into(),
                vps_ip: None,
                last_handshake_secs: None,
                relay_reachable: None,
                message: None,
                step: None,
            },
            ProxyStatus::Setup { step } => ProxyStatusInfo {
                enabled: false,
                status: "setup".into(),
                vps_ip: None,
                last_handshake_secs: None,
                relay_reachable: None,
                message: None,
                step: Some(step.clone()),
            },
            ProxyStatus::Connecting => ProxyStatusInfo {
                enabled: false,
                status: "connecting".into(),
                vps_ip: None,
                last_handshake_secs: None,
                relay_reachable: None,
                message: None,
                step: None,
            },
            ProxyStatus::Active { vps_ip, last_handshake_secs, relay_reachable } => ProxyStatusInfo {
                enabled: true,
                status: "active".into(),
                vps_ip: Some(vps_ip.clone()),
                last_handshake_secs: *last_handshake_secs,
                relay_reachable: *relay_reachable,
                message: None,
                step: None,
            },
            ProxyStatus::Error { message } => ProxyStatusInfo {
                enabled: false,
                status: "error".into(),
                vps_ip: None,
                last_handshake_secs: None,
                relay_reachable: None,
                message: Some(message.clone()),
                step: None,
            },
            ProxyStatus::Interrupted => ProxyStatusInfo {
                enabled: false,
                status: "interrupted".into(),
                vps_ip: None,
                last_handshake_secs: None,
                relay_reachable: None,
                message: Some("WireGuard tunnel stopped unexpectedly. Node stopped to prevent unproxied connections.".into()),
                step: None,
            },
        }
    }
}

#[tauri::command]
pub async fn get_proxy_status(
    state: State<'_, AppState>,
) -> Result<ProxyStatusInfo, String> {
    let status = state.proxy.status.lock().await;
    Ok(ProxyStatusInfo::from(&*status))
}

#[tauri::command]
pub async fn start_proxy_setup(
    state: State<'_, AppState>,
    vps_ip: String,
    vps_wg_port: Option<u16>,
) -> Result<(), String> {
    // Validate IP
    proxy_config::validate_public_ip(&vps_ip)?;

    // Generate keys and config
    let config = ProxyConfig::generate(&vps_ip, vps_wg_port);
    config.save(&state.default_data_dir)?;

    // Set status to setup
    {
        let mut status = state.proxy.status.lock().await;
        *status = ProxyStatus::Setup { step: "config_generated".into() };
    }

    log::info!("Proxy setup started for VPS {}", vps_ip);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySetupConfig {
    pub vps_wg_conf: String,
    pub docker_compose: String,
    pub install_command: String,
    pub home_wg_conf: String,
}

#[tauri::command]
pub async fn get_proxy_setup_config(
    state: State<'_, AppState>,
) -> Result<ProxySetupConfig, String> {
    let config = ProxyConfig::load(&state.default_data_dir)?;

    Ok(ProxySetupConfig {
        vps_wg_conf: config.generate_vps_wg_conf(),
        docker_compose: config.generate_docker_compose(),
        install_command: config.generate_install_command(),
        home_wg_conf: config.generate_home_wg_conf(),
    })
}

#[tauri::command]
pub async fn enable_proxy_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Check Network Serve not active
    {
        let net_status = state.network.status.lock().await;
        if matches!(*net_status, NetworkServeStatus::Active { .. }) {
            return Err("Disable Serve the Network first. Proxy Mode and Network Serve are mutually exclusive.".into());
        }
    }

    // Check proxy config exists
    if !ProxyConfig::exists(&state.default_data_dir) {
        return Err("Proxy not set up. Run proxy setup first.".into());
    }

    let data_dir = state.node.data_dir.lock().await.clone();

    // Start WireGuard tunnel
    wireguard::start_wireguard(
        app_handle.clone(),
        &state.proxy,
        &data_dir,
        &state.default_data_dir,
    ).await?;

    // Spawn kill switch
    let proxy_arc = state.proxy.clone();
    let ks_handle = wireguard::spawn_kill_switch(app_handle.clone(), proxy_arc);
    {
        let mut ks = state.proxy.kill_switch_task.lock().await;
        *ks = Some(ks_handle);
    }

    // Restart zebrad with external_addr if it was running
    let node_was_running = {
        let status = state.node.status.lock().await;
        matches!(*status, crate::state::NodeStatus::Running { .. })
    };
    if node_was_running {
        zebrad::stop_zebrad(&app_handle, &state.node).await?;
        zebrad::start_zebrad(app_handle.clone(), &state.node).await?;
    }

    // Persist privacy mode
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.privacy_mode = PrivacyMode::Proxy;
    config.save(&state.default_data_dir)?;

    // Mark setup complete
    if let Ok(mut proxy_config) = ProxyConfig::load(&state.default_data_dir) {
        proxy_config.setup_complete = true;
        let _ = proxy_config.save(&state.default_data_dir);
    }

    log::info!("Proxy Mode enabled");
    Ok(())
}

#[tauri::command]
pub async fn disable_proxy_mode(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.node.data_dir.lock().await.clone();

    // Stop node first (before dropping tunnel)
    let node_was_running = {
        let status = state.node.status.lock().await;
        matches!(*status, crate::state::NodeStatus::Running { .. })
    };
    if node_was_running {
        zebrad::stop_zebrad(&app_handle, &state.node).await?;
    }

    // Stop WireGuard
    wireguard::stop_wireguard(&app_handle, &state.proxy, &data_dir).await?;

    // Restart node with default config
    if node_was_running {
        zebrad::start_zebrad(app_handle.clone(), &state.node).await?;
    }

    // Persist privacy mode
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.privacy_mode = PrivacyMode::Standard;
    config.save(&state.default_data_dir)?;

    log::info!("Proxy Mode disabled");
    Ok(())
}

#[tauri::command]
pub async fn verify_proxy_connection(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    // Try TCP connect to VPS end of tunnel (10.13.37.1:8233)
    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::net::TcpStream::connect("10.13.37.1:8233"),
    ).await {
        Ok(Ok(_)) => {
            // Update relay_reachable
            let mut status = state.proxy.status.lock().await;
            if let ProxyStatus::Active { relay_reachable, .. } = &mut *status {
                *relay_reachable = Some(true);
            }
            Ok(true)
        }
        _ => {
            let mut status = state.proxy.status.lock().await;
            if let ProxyStatus::Active { relay_reachable, .. } = &mut *status {
                *relay_reachable = Some(false);
            }
            Ok(false)
        }
    }
}

#[tauri::command]
pub async fn reset_proxy_config(
    state: State<'_, AppState>,
) -> Result<(), String> {
    ProxyConfig::delete(&state.default_data_dir)?;

    let mut status = state.proxy.status.lock().await;
    *status = ProxyStatus::Disabled;

    log::info!("Proxy config reset");
    Ok(())
}

/// Get the list of VPS providers for the given tier.
#[tauri::command]
pub async fn get_vps_providers(
    tier: String,
) -> Result<serde_json::Value, String> {
    let providers = include_str!("../providers.json");
    let all: serde_json::Value = serde_json::from_str(providers)
        .map_err(|e| format!("Failed to parse providers: {}", e))?;

    // Filter by tier if specified
    if let Some(arr) = all.as_array() {
        let filtered: Vec<&serde_json::Value> = arr.iter()
            .filter(|p| {
                if tier.is_empty() { return true; }
                p.get("tiers").and_then(|t| t.as_array()).map_or(false, |tiers| {
                    tiers.iter().any(|t| t.get("useCase").and_then(|u| u.as_str()) == Some(&tier))
                })
            })
            .collect();
        Ok(serde_json::json!(filtered))
    } else {
        Ok(all)
    }
}
