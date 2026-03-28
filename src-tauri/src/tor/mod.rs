//! Arti SOCKS5 proxy lifecycle for Shield Mode.
//! Manages Arti as a sidecar process, monitors bootstrap and health,
//! implements kill switch logic (Arti crash while Shield ON → stop zebrad).

pub mod firewall;

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;

use crate::process::zebrad;
use crate::state::{AppState, ShieldState, ShieldStatus};

const ARTI_SOCKS_PORT: u16 = 9150;
const HEALTH_CHECK_INTERVAL_SECS: u64 = 3;

/// Start the Arti SOCKS5 proxy sidecar.
pub async fn start_arti(
    app_handle: AppHandle,
    shield: &ShieldState,
) -> Result<(), String> {
    {
        let status = shield.status.lock().await;
        if !matches!(*status, ShieldStatus::Disabled) {
            let desc = match &*status {
                ShieldStatus::Bootstrapping { .. } => "currently connecting",
                ShieldStatus::Active => "already active",
                ShieldStatus::Error { .. } => "in an error state",
                ShieldStatus::Interrupted => "interrupted",
                _ => "busy",
            };
            return Err(format!("Cannot enable Shield Mode: {}", desc));
        }
    }

    {
        let mut status = shield.status.lock().await;
        *status = ShieldStatus::Bootstrapping { progress: 0 };
    }
    emit_shield_status(&app_handle, &shield).await;

    let binary_path = resolve_arti_binary_path(&app_handle);
    if !binary_path.exists() {
        let mut status = shield.status.lock().await;
        *status = ShieldStatus::Error {
            message: "Tor proxy binary not found. Try reinstalling zecbox.".into(),
        };
        emit_shield_status(&app_handle, &shield).await;
        return Err(format!("Arti binary not found at {:?}", binary_path));
    }

    // Check for port conflict before spawning
    if let Err(_) = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", ARTI_SOCKS_PORT)).await {
        let mut status = shield.status.lock().await;
        *status = ShieldStatus::Error {
            message: format!("Port {} is already in use. Another Tor instance may be running.", ARTI_SOCKS_PORT),
        };
        emit_shield_status(&app_handle, shield).await;
        return Err(format!("Port {} is already in use", ARTI_SOCKS_PORT));
    }

    let mut child = tokio::process::Command::new(&binary_path)
        .arg("--socks-port")
        .arg(ARTI_SOCKS_PORT.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| format!("Failed to spawn Arti: {}", e))?;

    let pid = child.id().unwrap_or(0);
    log::info!("Arti started with PID {}", pid);

    // Write PID file
    {
        let state = app_handle.state::<AppState>();
        let data_dir = state.node.data_dir.lock().await.clone();
        if let Err(e) = write_pid_file(&data_dir, pid) {
            log::warn!("Failed to write Arti PID file: {}", e);
        }
    }

    // Monitor stderr for bootstrap progress
    if let Some(stderr) = child.stderr.take() {
        let app = app_handle.clone();
        let state = app_handle.state::<AppState>();
        let shield_arc = state.shield.clone();
        let bootstrap_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("arti: {}", line);
                if let Some(pct) = parse_bootstrap_progress(&line) {
                    let mut status = shield_arc.status.lock().await;
                    if matches!(*status, ShieldStatus::Bootstrapping { .. }) {
                        *status = ShieldStatus::Bootstrapping { progress: pct };
                        drop(status);
                        let _ = app.emit("shield_status_changed", get_status_payload(&shield_arc).await);
                        if pct >= 100 {
                            let mut status = shield_arc.status.lock().await;
                            *status = ShieldStatus::Active;
                            drop(status);
                            let _ = app.emit("shield_status_changed", get_status_payload(&shield_arc).await);
                        }
                    }
                }
            }
        });
        let mut task = shield.bootstrap_task.lock().await;
        *task = Some(bootstrap_task);
    }

    // Drain stdout
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::debug!("arti stdout: {}", line);
            }
        });
    }

    {
        let mut proc = shield.process.lock().await;
        *proc = Some(child);
    }

    // Wait for bootstrap completion (up to 60s)
    let state = app_handle.state::<AppState>();
    let shield_arc = state.shield.clone();
    let bootstrapped = wait_for_bootstrap(&shield_arc, 60).await;
    if !bootstrapped {
        // If bootstrap didn't complete, check if process is still alive
        let alive = {
            let mut proc = shield.process.lock().await;
            if let Some(ref mut child) = *proc {
                child.try_wait().ok().flatten().is_none()
            } else {
                false
            }
        };
        if alive {
            // Process is running but bootstrap didn't complete — report error, don't assume Active
            let mut status = shield.status.lock().await;
            *status = ShieldStatus::Error {
                message: "Tor bootstrap timed out. Check your network connection.".into(),
            };
            emit_shield_status(&app_handle, shield).await;
            return Err("Arti bootstrap timed out after 60s".into());
        } else {
            let mut status = shield.status.lock().await;
            *status = ShieldStatus::Error {
                message: "Arti failed to bootstrap".into(),
            };
            emit_shield_status(&app_handle, &shield).await;
            return Err("Arti failed to bootstrap within timeout".into());
        }
    }

    emit_shield_status(&app_handle, &shield).await;

    // Spawn kill switch monitor
    let kill_switch_handle = spawn_kill_switch(app_handle.clone(), shield_arc);
    {
        let mut ks = shield.kill_switch_task.lock().await;
        *ks = Some(kill_switch_handle);
    }

    Ok(())
}

/// Stop the Arti SOCKS5 proxy.
pub async fn stop_arti(
    app_handle: &AppHandle,
    shield: &ShieldState,
) -> Result<(), String> {
    // Abort kill switch
    {
        let mut ks = shield.kill_switch_task.lock().await;
        if let Some(handle) = ks.take() {
            handle.abort();
        }
    }

    // Abort bootstrap task
    {
        let mut bt = shield.bootstrap_task.lock().await;
        if let Some(handle) = bt.take() {
            handle.abort();
        }
    }

    // Gracefully stop: SIGTERM → wait → force kill
    {
        let mut proc = shield.process.lock().await;
        if let Some(ref mut child) = *proc {
            crate::process::platform::graceful_stop(child, std::time::Duration::from_secs(5)).await;
        }
        *proc = None;
    }

    // Remove PID file
    if let Some(state) = app_handle.try_state::<AppState>() {
        let data_dir = state.node.data_dir.lock().await.clone();
        let _ = remove_pid_file(&data_dir);
    }

    {
        let mut status = shield.status.lock().await;
        *status = ShieldStatus::Disabled;
    }

    emit_shield_status(app_handle, shield).await;
    log::info!("Arti stopped");
    Ok(())
}

/// Kill switch: monitors Arti health and PF firewall status.
/// If either Arti dies or PF firewall is disabled while Shield is ON,
/// immediately stop zebrad to prevent clearnet fallback.
fn spawn_kill_switch(
    app_handle: AppHandle,
    shield: Arc<ShieldState>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS));

        loop {
            interval.tick().await;

            let status = shield.status.lock().await.clone();
            match status {
                ShieldStatus::Active | ShieldStatus::Bootstrapping { .. } => {}
                _ => break,
            }

            // Check if Arti process is alive
            let arti_dead = {
                let mut proc = shield.process.lock().await;
                if let Some(ref mut child) = *proc {
                    child.try_wait().ok().flatten().is_some()
                } else {
                    true
                }
            };

            // Check if PF firewall is still active (during both Active and Bootstrapping)
            let firewall_down = if !arti_dead {
                match firewall::firewall_status() {
                    Ok((enabled, _)) => !enabled,
                    Err(_) => false, // Don't trip kill switch on transient query failures
                }
            } else {
                false
            };

            let reason = if arti_dead {
                Some("Tor proxy stopped unexpectedly. Node stopped to prevent clearnet exposure.")
            } else if firewall_down {
                Some("Firewall rules removed unexpectedly. Node stopped to prevent clearnet exposure.")
            } else {
                None
            };

            if let Some(msg) = reason {
                log::error!("KILL SWITCH: {} — stopping zebrad", msg);

                if arti_dead {
                    let mut proc = shield.process.lock().await;
                    *proc = None;
                }
                {
                    let mut status = shield.status.lock().await;
                    *status = ShieldStatus::Interrupted;
                }

                let _ = app_handle.emit("shield_interrupted", msg);

                // Stop zebrad immediately
                let state = app_handle.state::<AppState>();
                let _ = zebrad::stop_zebrad(&app_handle, &state.node).await;

                let _ = app_handle.emit(
                    "shield_status_changed",
                    get_status_payload(&shield).await,
                );

                break;
            }
        }
    })
}

async fn wait_for_bootstrap(shield: &ShieldState, timeout_secs: u64) -> bool {
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_secs(timeout_secs);

    loop {
        let status = shield.status.lock().await.clone();
        match status {
            ShieldStatus::Active => return true,
            ShieldStatus::Error { .. } | ShieldStatus::Disabled | ShieldStatus::Interrupted => {
                return false;
            }
            ShieldStatus::Bootstrapping { .. } => {}
        }

        if tokio::time::Instant::now() >= deadline {
            return false;
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

fn parse_bootstrap_progress(line: &str) -> Option<u8> {
    // Expected format: "BOOTSTRAP PROGRESS=XX"
    if let Some(idx) = line.find("BOOTSTRAP PROGRESS=") {
        let after = &line[idx + "BOOTSTRAP PROGRESS=".len()..];
        let num_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        num_str.parse().ok()
    } else {
        None
    }
}

fn resolve_arti_binary_path(app_handle: &AppHandle) -> PathBuf {
    crate::platform::resolve_sidecar_path(app_handle, "arti")
}

async fn emit_shield_status(app_handle: &AppHandle, shield: &ShieldState) {
    let payload = get_status_payload(shield).await;
    let _ = app_handle.emit("shield_status_changed", payload);
}

async fn get_status_payload(shield: &ShieldState) -> serde_json::Value {
    let status = shield.status.lock().await;
    match &*status {
        ShieldStatus::Disabled => serde_json::json!({
            "status": "disabled",
            "enabled": false,
        }),
        ShieldStatus::Bootstrapping { progress } => serde_json::json!({
            "status": "bootstrapping",
            "enabled": false,
            "bootstrapProgress": progress,
        }),
        ShieldStatus::Active => serde_json::json!({
            "status": "active",
            "enabled": true,
        }),
        ShieldStatus::Error { message } => serde_json::json!({
            "status": "error",
            "enabled": false,
            "message": message,
        }),
        ShieldStatus::Interrupted => serde_json::json!({
            "status": "interrupted",
            "enabled": false,
            "message": "Tor proxy stopped unexpectedly. Node stopped to prevent clearnet exposure.",
        }),
    }
}

// --- PID file management ---

fn write_pid_file(data_dir: &Path, pid: u32) -> Result<(), std::io::Error> {
    let pid_path = data_dir.join("arti.pid");
    std::fs::write(&pid_path, pid.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&pid_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn read_pid_file(data_dir: &Path) -> Option<u32> {
    std::fs::read_to_string(data_dir.join("arti.pid"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn remove_pid_file(data_dir: &Path) -> Result<(), std::io::Error> {
    let path = data_dir.join("arti.pid");
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Verify the Tor routing path is functional by performing a SOCKS5 handshake
/// through the redirector to Arti. This confirms: PF rules → redirector → Arti are all working.
pub async fn verify_tor_path() -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let arti_addr = "127.0.0.1:9150";

    let mut stream = tokio::time::timeout(
        Duration::from_secs(10),
        TcpStream::connect(arti_addr),
    )
    .await
    .map_err(|_| "Tor path verification timed out connecting to Arti".to_string())?
    .map_err(|e| format!("Cannot connect to Arti SOCKS at {}: {}", arti_addr, e))?;

    // SOCKS5 greeting: version 5, 1 auth method (no auth)
    stream
        .write_all(&[0x05, 0x01, 0x00])
        .await
        .map_err(|e| format!("SOCKS5 handshake write failed: {}", e))?;

    let mut resp = [0u8; 2];
    tokio::time::timeout(Duration::from_secs(5), stream.read_exact(&mut resp))
        .await
        .map_err(|_| "SOCKS5 handshake read timed out".to_string())?
        .map_err(|e| format!("SOCKS5 handshake read failed: {}", e))?;

    if resp[0] != 0x05 || resp[1] != 0x00 {
        return Err(format!(
            "Arti SOCKS5 handshake failed: version={}, method={}",
            resp[0], resp[1]
        ));
    }

    log::info!("Tor path verification passed: SOCKS5 handshake with Arti successful");
    Ok(())
}

/// Check for and clean up orphaned Arti process from a prior crash.
pub async fn check_arti_orphan(data_dir: &Path) -> Result<(), String> {
    if let Some(pid) = read_pid_file(data_dir) {
        if crate::process::platform::is_process_alive(pid) {
            if !crate::process::is_process_named(pid, "arti") {
                log::warn!("PID {} from arti.pid is not an arti process, removing stale PID file", pid);
                let _ = remove_pid_file(data_dir);
                return Ok(());
            }

            log::warn!("Found orphaned Arti process (PID {}), killing it", pid);
            crate::process::platform::send_term(pid);

            tokio::time::sleep(Duration::from_secs(3)).await;

            if crate::process::platform::is_process_alive(pid) {
                crate::process::platform::force_kill(pid);
            }
        }
        let _ = remove_pid_file(data_dir);
    }
    Ok(())
}
