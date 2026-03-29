//! WireGuard (boringtun) sidecar lifecycle for Proxy Mode.
//! Manages the userspace WireGuard tunnel as a child process.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::proxy_config::ProxyConfig;
use crate::state::{AppState, ProxyState, ProxyStatus};

const PID_FILE: &str = "wireguard.pid";

/// Start the boringtun userspace WireGuard process.
pub async fn start_wireguard(
    app_handle: AppHandle,
    proxy: &ProxyState,
    data_dir: &Path,
    default_data_dir: &Path,
) -> Result<(), String> {
    {
        let status = proxy.status.lock().await;
        match &*status {
            ProxyStatus::Active { .. } | ProxyStatus::Connecting => {
                return Err("WireGuard tunnel is already active or connecting.".into());
            }
            _ => {}
        }
    }

    // Load proxy config to get WireGuard settings
    let config = ProxyConfig::load(default_data_dir)?;

    // Set status to Connecting
    {
        let mut status = proxy.status.lock().await;
        *status = ProxyStatus::Connecting;
    }
    emit_proxy_status(&app_handle, proxy).await;

    // Write WireGuard client config to temp file
    let wg_conf_path = data_dir.join("config").join("wg0.conf");
    let wg_conf = config.generate_home_wg_conf();
    std::fs::create_dir_all(wg_conf_path.parent().unwrap())
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    std::fs::write(&wg_conf_path, &wg_conf)
        .map_err(|e| format!("Failed to write WireGuard config: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&wg_conf_path, std::fs::Permissions::from_mode(0o600));
    }

    // Resolve boringtun binary path
    let binary_path = resolve_wireguard_binary_path(&app_handle);
    if !binary_path.exists() {
        let mut status = proxy.status.lock().await;
        *status = ProxyStatus::Error {
            message: "WireGuard binary not found. Try reinstalling zecbox.".into(),
        };
        emit_proxy_status(&app_handle, proxy).await;
        return Err(format!("boringtun binary not found at {:?}", binary_path));
    }

    // Spawn boringtun
    let mut cmd = tokio::process::Command::new(&binary_path);
    cmd.arg("--config")
        .arg(&wg_conf_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);

    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn WireGuard: {}", e))?;

    let pid = child.id().unwrap_or(0);
    log::info!("WireGuard (boringtun) started with PID {}", pid);

    // Write PID file
    if let Err(e) = super::write_pid_file(data_dir, PID_FILE, pid) {
        log::warn!("Failed to write WireGuard PID file: {}", e);
    }

    // Drain stdout/stderr
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            use tokio::io::AsyncBufReadExt;
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("wireguard stdout: {}", line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = tokio::io::BufReader::new(stderr).lines();
            use tokio::io::AsyncBufReadExt;
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("wireguard stderr: {}", line);
            }
        });
    }

    {
        let mut proc = proxy.process.lock().await;
        *proc = Some(child);
    }

    // Wait briefly for tunnel to establish, then mark active
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Check if process is still alive
    let alive = {
        let mut proc = proxy.process.lock().await;
        if let Some(ref mut child) = *proc {
            child.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    };

    if alive {
        let mut status = proxy.status.lock().await;
        *status = ProxyStatus::Active {
            vps_ip: config.vps_ip.clone(),
            last_handshake_secs: None,
            relay_reachable: None,
        };
        drop(status);
        emit_proxy_status(&app_handle, proxy).await;
    } else {
        let mut status = proxy.status.lock().await;
        *status = ProxyStatus::Error {
            message: "WireGuard process exited unexpectedly during startup.".into(),
        };
        emit_proxy_status(&app_handle, proxy).await;
        return Err("WireGuard process died during startup".into());
    }

    Ok(())
}

/// Stop the WireGuard tunnel.
pub async fn stop_wireguard(
    app_handle: &AppHandle,
    proxy: &ProxyState,
    data_dir: &Path,
) -> Result<(), String> {
    // Abort kill switch
    {
        let mut ks = proxy.kill_switch_task.lock().await;
        if let Some(handle) = ks.take() {
            handle.abort();
        }
    }

    // Abort monitor
    {
        let mut mt = proxy.monitor_task.lock().await;
        if let Some(handle) = mt.take() {
            handle.abort();
        }
    }

    // Gracefully stop process
    {
        let mut proc = proxy.process.lock().await;
        if let Some(ref mut child) = *proc {
            super::platform::graceful_stop(child, Duration::from_secs(5)).await;
        }
        *proc = None;
    }

    // Remove PID file
    let pid_path = data_dir.join(PID_FILE);
    if pid_path.exists() {
        let _ = std::fs::remove_file(&pid_path);
    }

    // Set status
    {
        let mut status = proxy.status.lock().await;
        *status = ProxyStatus::Disabled;
    }
    emit_proxy_status(app_handle, proxy).await;

    log::info!("WireGuard stopped");
    Ok(())
}

/// Check for and clean up orphaned WireGuard process from a prior crash.
pub async fn check_wireguard_orphan(data_dir: &Path) -> Result<(), String> {
    let pid_path = data_dir.join(PID_FILE);
    if let Ok(contents) = std::fs::read_to_string(&pid_path) {
        if let Ok(pid) = contents.trim().parse::<u32>() {
            if super::platform::is_process_alive(pid) {
                if !super::is_process_named(pid, "boringtun") {
                    log::warn!("PID {} from wireguard.pid is not boringtun, removing stale PID file", pid);
                    let _ = std::fs::remove_file(&pid_path);
                    return Ok(());
                }

                log::warn!("Found orphaned WireGuard process (PID {}), killing it", pid);
                super::platform::send_term(pid);
                tokio::time::sleep(Duration::from_secs(3)).await;

                if super::platform::is_process_alive(pid) {
                    super::platform::force_kill(pid);
                }
            }
            let _ = std::fs::remove_file(&pid_path);
        }
    }
    Ok(())
}

/// Spawn kill switch: monitors WireGuard process health.
/// If boringtun dies while Proxy/Shield mode is active, stop zebrad immediately.
pub fn spawn_kill_switch(
    app_handle: AppHandle,
    proxy: std::sync::Arc<ProxyState>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3));

        loop {
            interval.tick().await;

            let status = proxy.status.lock().await.clone();
            match status {
                ProxyStatus::Active { .. } | ProxyStatus::Connecting => {}
                _ => break,
            }

            // Check if boringtun process is alive
            let dead = {
                let mut proc = proxy.process.lock().await;
                if let Some(ref mut child) = *proc {
                    child.try_wait().ok().flatten().is_some()
                } else {
                    true
                }
            };

            if dead {
                log::error!("PROXY KILL SWITCH: WireGuard tunnel died — stopping zebrad");

                {
                    let mut proc = proxy.process.lock().await;
                    *proc = None;
                }
                {
                    let mut status = proxy.status.lock().await;
                    *status = ProxyStatus::Interrupted;
                }

                let _ = app_handle.emit("proxy_interrupted", "WireGuard tunnel stopped unexpectedly. Node stopped to prevent unproxied connections.");

                // Stop zebrad
                let state = app_handle.state::<AppState>();
                let _ = crate::process::zebrad::stop_zebrad(&app_handle, &state.node).await;

                emit_proxy_status(&app_handle, &proxy).await;
                break;
            }
        }
    })
}

fn resolve_wireguard_binary_path(app_handle: &AppHandle) -> PathBuf {
    crate::platform::resolve_sidecar_path(app_handle, "boringtun")
}

pub async fn emit_proxy_status(app_handle: &AppHandle, proxy: &ProxyState) {
    let status = proxy.status.lock().await;
    let payload = match &*status {
        ProxyStatus::Disabled => serde_json::json!({
            "status": "disabled",
            "enabled": false,
        }),
        ProxyStatus::Setup { step } => serde_json::json!({
            "status": "setup",
            "enabled": false,
            "step": step,
        }),
        ProxyStatus::Connecting => serde_json::json!({
            "status": "connecting",
            "enabled": false,
        }),
        ProxyStatus::Active { vps_ip, last_handshake_secs, relay_reachable } => serde_json::json!({
            "status": "active",
            "enabled": true,
            "vpsIp": vps_ip,
            "lastHandshakeSecs": last_handshake_secs,
            "relayReachable": relay_reachable,
        }),
        ProxyStatus::Error { message } => serde_json::json!({
            "status": "error",
            "enabled": false,
            "message": message,
        }),
        ProxyStatus::Interrupted => serde_json::json!({
            "status": "interrupted",
            "enabled": false,
            "message": "WireGuard tunnel stopped unexpectedly. Node stopped to prevent unproxied connections.",
        }),
    };
    let _ = app_handle.emit("proxy_status_changed", payload);
}
