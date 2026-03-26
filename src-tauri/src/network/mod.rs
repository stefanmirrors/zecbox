use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use igd_next::PortMappingProtocol;
use serde_json::json;
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;

use crate::commands::network::NetworkServeStatusInfo;
use crate::state::{NetworkServeState, NetworkServeStatus};

const ZCASH_P2P_PORT: u16 = 8233;
const UPNP_LEASE_SECS: u32 = 3600;
const PEER_POLL_INTERVAL: Duration = Duration::from_secs(10);
const UPNP_RENEW_INTERVAL: Duration = Duration::from_secs(1800);
const REACHABILITY_TIMEOUT: Duration = Duration::from_secs(10);
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Attempt to add a UPnP port mapping for the given port.
/// Returns (public_ip, upnp_succeeded, cgnat_detected).
pub async fn enable_upnp(port: u16) -> Result<(String, bool, bool), String> {
    match igd_next::aio::tokio::search_gateway(Default::default()).await {
        Ok(gateway) => {
            let local_ip = gateway.addr.ip();
            let local_addr: SocketAddrV4 = format!("{}:{}", local_ip, port)
                .parse()
                .map_err(|e| format!("Failed to parse local address: {}", e))?;

            // Get external IP from gateway
            let public_ip = match gateway.get_external_ip().await {
                Ok(ip) => ip.to_string(),
                Err(_) => get_public_ip_fallback().await.unwrap_or_default(),
            };

            // CGNAT detection: compare gateway's external IP with actual public IP
            let real_public_ip = get_public_ip_fallback().await.unwrap_or_default();
            let cgnat = !public_ip.is_empty() && !real_public_ip.is_empty() && public_ip != real_public_ip;
            if cgnat {
                log::warn!("CGNAT detected: gateway reports {} but public IP is {}", public_ip, real_public_ip);
            }
            let effective_ip = if !real_public_ip.is_empty() { real_public_ip } else { public_ip };

            // Add port mapping (still try even if CGNAT — won't hurt)
            match gateway
                .add_port(
                    PortMappingProtocol::TCP,
                    port,
                    SocketAddr::V4(local_addr),
                    UPNP_LEASE_SECS,
                    "TCP Port 8233",
                )
                .await
            {
                Ok(()) => Ok((effective_ip, true, cgnat)),
                Err(e) => {
                    log::warn!("UPnP port mapping failed: {}", e);
                    Ok((effective_ip, false, cgnat))
                }
            }
        }
        Err(e) => {
            log::warn!("UPnP gateway discovery failed: {}", e);
            let public_ip = get_public_ip_fallback().await.unwrap_or_default();
            Ok((public_ip, false, false))
        }
    }
}

/// Remove UPnP port mapping. Swallows errors since the mapping may already be gone.
pub async fn disable_upnp(port: u16) {
    match igd_next::aio::tokio::search_gateway(Default::default()).await {
        Ok(gateway) => {
            let _ = gateway.remove_port(PortMappingProtocol::TCP, port).await;
        }
        Err(_) => {}
    }
}

/// Renew the UPnP port mapping.
async fn renew_upnp(port: u16) {
    if let Ok(gateway) = igd_next::aio::tokio::search_gateway(Default::default()).await {
        let local_ip = gateway.addr.ip();
        if let Ok(local_addr) = format!("{}:{}", local_ip, port).parse::<SocketAddrV4>() {
            let _ = gateway
                .add_port(
                    PortMappingProtocol::TCP,
                    port,
                    SocketAddr::V4(local_addr),
                    UPNP_LEASE_SECS,
                    "TCP Port 8233",
                )
                .await;
        }
    }
}

/// Get public IP via external service as fallback.
async fn get_public_ip_fallback() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.ipify.org")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.text().await.map_err(|e| e.to_string())
}

/// Check if port is reachable from outside using an external port-check service.
/// Returns None if the check itself fails (service unreachable).
pub async fn check_reachability(_ip: &str, port: u16) -> Option<bool> {
    let client = reqwest::Client::builder()
        .timeout(REACHABILITY_TIMEOUT)
        .build()
        .ok()?;

    // Use ifconfig.co port check API (checks from the requester's public IP)
    let url = format!("https://ifconfig.co/port/{}", port);
    match client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                return json.get("reachable").and_then(|v| v.as_bool());
            }
            None
        }
        Err(_) => None,
    }
}

/// Get local IP address for display in manual port forwarding instructions.
pub fn get_local_ip() -> Option<String> {
    // Try to get from UPnP gateway synchronously-ish, or use a simple socket trick
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip().to_string())
}

/// Poll zebrad's getpeerinfo RPC for inbound/outbound peer counts.
/// Falls back to getinfo total connections if getpeerinfo is unsupported.
/// Returns (Option<inbound>, Option<outbound>) — None means unknown split.
pub async fn get_peer_info(client: &reqwest::Client) -> Result<(Option<u32>, Option<u32>), String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "getpeerinfo",
        "params": [],
        "id": 3
    });

    let resp = client
        .post("http://127.0.0.1:8232")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("RPC failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid response: {}", e))?;

    // Check if method is supported
    if json.get("error").is_some() {
        // Fallback: use getinfo connections as all outbound
        return get_peer_count_fallback(client).await;
    }

    if let Some(peers) = json.get("result").and_then(|r| r.as_array()) {
        let mut inbound = 0u32;
        let mut outbound = 0u32;
        for peer in peers {
            if peer.get("inbound").and_then(|v| v.as_bool()).unwrap_or(false) {
                inbound += 1;
            } else {
                outbound += 1;
            }
        }
        Ok((Some(inbound), Some(outbound)))
    } else {
        get_peer_count_fallback(client).await
    }
}

async fn get_peer_count_fallback(client: &reqwest::Client) -> Result<(Option<u32>, Option<u32>), String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "getinfo",
        "params": [],
        "id": 1
    });
    let resp = client
        .post("http://127.0.0.1:8232")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("RPC failed: {}", e))?;
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid response: {}", e))?;
    let total = json
        .get("result")
        .and_then(|r| r.get("connections"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;
    // We don't know the inbound/outbound split from getinfo
    Ok((None, Some(total)))
}

const MAX_CONSECUTIVE_POLL_FAILURES: u32 = 5;

/// Spawn the network serve monitor task.
/// Polls peer info every 10s and renews UPnP every 30 min.
/// Auto-disables if the node goes down (consecutive RPC failures).
pub fn spawn_network_monitor(
    app_handle: AppHandle,
    network: Arc<NetworkServeState>,
    upnp_active: bool,
    default_data_dir: std::path::PathBuf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(RPC_TIMEOUT)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to create HTTP client for network monitor: {}", e);
                return;
            }
        };

        let mut peer_interval = tokio::time::interval(PEER_POLL_INTERVAL);
        let mut renew_interval = tokio::time::interval(UPNP_RENEW_INTERVAL);
        // Skip the immediate first tick of renew
        renew_interval.tick().await;
        let mut consecutive_failures: u32 = 0;

        loop {
            tokio::select! {
                _ = peer_interval.tick() => {
                    // Check if still active
                    {
                        let status = network.status.lock().await;
                        if !matches!(*status, NetworkServeStatus::Active { .. }) {
                            break;
                        }
                    }

                    match get_peer_info(&client).await {
                        Ok((inbound, outbound)) => {
                            consecutive_failures = 0;
                            let mut status = network.status.lock().await;
                            if let NetworkServeStatus::Active {
                                ref mut inbound_peers,
                                ref mut outbound_peers,
                                ..
                            } = *status
                            {
                                *inbound_peers = inbound;
                                *outbound_peers = outbound;
                            }
                            let info = NetworkServeStatusInfo::from(&*status);
                            drop(status);
                            let _ = app_handle.emit("network_serve_status_changed", &info);
                        }
                        Err(_) => {
                            consecutive_failures += 1;
                            if consecutive_failures >= MAX_CONSECUTIVE_POLL_FAILURES {
                                log::warn!("Node appears down ({} consecutive RPC failures), disabling network serve", consecutive_failures);
                                // Auto-disable: clean up UPnP
                                if upnp_active {
                                    disable_upnp(ZCASH_P2P_PORT).await;
                                }
                                // Set status to Disabled
                                {
                                    let mut status = network.status.lock().await;
                                    *status = NetworkServeStatus::Disabled;
                                }
                                let _ = app_handle.emit(
                                    "network_serve_status_changed",
                                    NetworkServeStatusInfo::from(&NetworkServeStatus::Disabled),
                                );
                                // Persist
                                if let Ok(mut config) = crate::config::app_config::AppConfig::load(&default_data_dir) {
                                    config.serve_network = false;
                                    let _ = config.save(&default_data_dir);
                                }
                                break;
                            }
                        }
                    }
                }
                _ = renew_interval.tick() => {
                    if upnp_active {
                        renew_upnp(ZCASH_P2P_PORT).await;
                    }
                }
            }
        }
    })
}
