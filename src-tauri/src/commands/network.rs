use tauri::{AppHandle, Emitter, State};

use crate::config::app_config::AppConfig;
use crate::network;
use crate::state::{AppState, NetworkServeStatus, NodeStatus, ShieldStatus};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkServeStatusInfo {
    pub enabled: bool,
    pub status: String,
    pub public_ip: Option<String>,
    pub reachable: Option<bool>,
    pub inbound_peers: Option<u32>,
    pub outbound_peers: Option<u32>,
    pub upnp_active: Option<bool>,
    pub local_ip: Option<String>,
    pub cgnat_detected: Option<bool>,
    pub message: Option<String>,
}

impl From<&NetworkServeStatus> for NetworkServeStatusInfo {
    fn from(status: &NetworkServeStatus) -> Self {
        match status {
            NetworkServeStatus::Disabled => NetworkServeStatusInfo {
                enabled: false,
                status: "disabled".into(),
                public_ip: None,
                reachable: None,
                inbound_peers: None,
                outbound_peers: None,
                upnp_active: None,
                local_ip: None,
                cgnat_detected: None,
                message: None,
            },
            NetworkServeStatus::Enabling => NetworkServeStatusInfo {
                enabled: false,
                status: "enabling".into(),
                public_ip: None,
                reachable: None,
                inbound_peers: None,
                outbound_peers: None,
                upnp_active: None,
                local_ip: None,
                cgnat_detected: None,
                message: None,
            },
            NetworkServeStatus::Active {
                public_ip,
                reachable,
                inbound_peers,
                outbound_peers,
                upnp_active,
                local_ip,
                cgnat_detected,
            } => NetworkServeStatusInfo {
                enabled: true,
                status: "active".into(),
                public_ip: public_ip.clone(),
                reachable: *reachable,
                inbound_peers: *inbound_peers,
                outbound_peers: *outbound_peers,
                upnp_active: Some(*upnp_active),
                local_ip: local_ip.clone(),
                cgnat_detected: Some(*cgnat_detected),
                message: None,
            },
            NetworkServeStatus::Error { message } => NetworkServeStatusInfo {
                enabled: false,
                status: "error".into(),
                public_ip: None,
                reachable: None,
                inbound_peers: None,
                outbound_peers: None,
                upnp_active: None,
                local_ip: None,
                cgnat_detected: None,
                message: Some(message.clone()),
            },
        }
    }
}

#[tauri::command]
pub async fn get_network_serve_status(
    state: State<'_, AppState>,
) -> Result<NetworkServeStatusInfo, String> {
    let status = state.network.status.lock().await;
    Ok(NetworkServeStatusInfo::from(&*status))
}

#[tauri::command]
pub async fn enable_network_serve(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Check Shield Mode not active
    {
        let shield = state.shield.status.lock().await;
        if matches!(*shield, ShieldStatus::Active | ShieldStatus::Bootstrapping { .. }) {
            return Err("Disable Shield Mode first. Accepting inbound connections is not possible while routing through Tor.".into());
        }
    }

    // Check node is running
    {
        let node = state.node.status.lock().await;
        if !matches!(*node, NodeStatus::Running { .. }) {
            return Err("Node must be running to serve the network.".into());
        }
    }

    // Check not already active
    {
        let status = state.network.status.lock().await;
        if matches!(*status, NetworkServeStatus::Active { .. } | NetworkServeStatus::Enabling) {
            return Err("Network serving is already enabled.".into());
        }
    }

    // Set status to Enabling
    {
        let mut status = state.network.status.lock().await;
        *status = NetworkServeStatus::Enabling;
    }
    let _ = app_handle.emit(
        "network_serve_status_changed",
        NetworkServeStatusInfo::from(&NetworkServeStatus::Enabling),
    );

    // Persist setting early so it survives a crash during setup
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.serve_network = true;
    config.save(&state.default_data_dir)?;

    // Spawn the heavy work (UPnP, reachability) in background to avoid blocking IPC
    let network_arc = state.network.clone();
    let default_data_dir = state.default_data_dir.clone();
    let app = app_handle.clone();
    tokio::spawn(async move {
        // Attempt UPnP
        let (public_ip, upnp_active, cgnat) = network::enable_upnp(8233).await.unwrap_or_else(|e| {
            log::warn!("UPnP failed: {}", e);
            (String::new(), false, false)
        });

        let public_ip_opt = if public_ip.is_empty() {
            None
        } else {
            Some(public_ip.clone())
        };

        // Check reachability
        let reachable = if !public_ip.is_empty() {
            network::check_reachability(&public_ip, 8233).await
        } else {
            None
        };

        let local_ip = network::get_local_ip();

        // Set active status
        let active_status = NetworkServeStatus::Active {
            public_ip: public_ip_opt,
            reachable,
            inbound_peers: None,
            outbound_peers: None,
            upnp_active,
            local_ip,
            cgnat_detected: cgnat,
        };
        {
            let mut status = network_arc.status.lock().await;
            *status = active_status.clone();
        }
        let _ = app.emit(
            "network_serve_status_changed",
            NetworkServeStatusInfo::from(&active_status),
        );

        // Spawn monitor task
        let monitor = network::spawn_network_monitor(
            app.clone(),
            network_arc.clone(),
            upnp_active,
            default_data_dir,
        );
        {
            let mut task = network_arc.monitor_task.lock().await;
            *task = Some(monitor);
        }

        log::info!(
            "Network serving enabled (UPnP: {}, reachable: {:?})",
            upnp_active,
            reachable
        );
    });

    Ok(())
}

#[tauri::command]
pub async fn disable_network_serve(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Abort monitor task
    {
        let mut task = state.network.monitor_task.lock().await;
        if let Some(t) = task.take() {
            t.abort();
        }
    }

    // Remove UPnP mapping
    network::disable_upnp(8233).await;

    // Set disabled status
    {
        let mut status = state.network.status.lock().await;
        *status = NetworkServeStatus::Disabled;
    }
    let _ = app_handle.emit(
        "network_serve_status_changed",
        NetworkServeStatusInfo::from(&NetworkServeStatus::Disabled),
    );

    // Persist setting
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.serve_network = false;
    config.save(&state.default_data_dir)?;

    log::info!("Network serving disabled");
    Ok(())
}

#[tauri::command]
pub async fn recheck_reachability(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let public_ip = {
        let status = state.network.status.lock().await;
        if let NetworkServeStatus::Active { ref public_ip, .. } = *status {
            public_ip.clone()
        } else {
            return Err("Network serving is not active.".into());
        }
    };

    let reachable = if let Some(ip) = &public_ip {
        network::check_reachability(ip, 8233).await
    } else {
        None
    };

    // Update status
    {
        let mut status = state.network.status.lock().await;
        if let NetworkServeStatus::Active {
            reachable: ref mut r,
            ..
        } = *status
        {
            *r = reachable;
        }
        let s = status.clone();
        drop(status);
        let _ = app_handle.emit(
            "network_serve_status_changed",
            NetworkServeStatusInfo::from(&s),
        );
    }

    Ok(())
}
