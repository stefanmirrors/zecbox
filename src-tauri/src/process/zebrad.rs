use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::zebrad_config;
use crate::health;
use crate::state::{AppState, NodeState, NodeStatus, LOG_BUFFER_CAPACITY};

/// Start zebrad, spawn log readers and health monitor.
pub async fn start_zebrad(
    app_handle: AppHandle,
    node: &NodeState,
) -> Result<(), String> {
    {
        let status = node.status.lock().await;
        if !status.is_stopped_or_error() {
            return Err(format!("Cannot start node: currently {}", status.status_str()));
        }
    }

    // Set status to Starting
    {
        let mut status = node.status.lock().await;
        *status = NodeStatus::Starting { message: None, progress: None };
    }
    let _ = app_handle.emit("node_status_changed", NodeStatus::Starting { message: None, progress: None });

    let data_dir = node.data_dir.lock().await.clone();

    // Check if shield mode (Tor) is active to generate appropriate config
    let shield_active = {
        let state = app_handle.state::<AppState>();
        state.shield.is_active().await
    };

    // Get .onion address if shield mode has one (for external_addr)
    let onion_address = {
        let state = app_handle.state::<AppState>();
        let addr = state.shield.onion_address.lock().await.clone();
        addr
    };

    // Generate config
    let config_path = zebrad_config::write_zebrad_config(&data_dir, shield_active, onion_address.as_deref())
        .map_err(|e| format!("Failed to write zebrad config: {}", e))?;

    // Resolve binary path
    let binary_path = resolve_binary_path(&app_handle);
    if !binary_path.exists() {
        let mut status = node.status.lock().await;
        *status = NodeStatus::Error {
            message: "Node binary not found. Try reinstalling zecbox.".into(),
        };
        let _ = app_handle.emit("node_status_changed", &*status);
        return Err(format!("zebrad binary not found at {:?}", binary_path));
    }

    // Check for port conflicts before spawning
    for port in [8232u16, 8233] {
        if let Err(msg) = check_port_available(port).await {
            let mut status = node.status.lock().await;
            *status = NodeStatus::Error { message: msg.clone() };
            let _ = app_handle.emit("node_status_changed", &*status);
            return Err(msg);
        }
    }

    // Spawn zebrad process (with proxy env vars if shield mode active)
    let mut cmd = tokio::process::Command::new(&binary_path);
    cmd.arg("--config")
        .arg(&config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false);

    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    if shield_active {
        log::info!("Starting zebrad with Shield Mode active (PF firewall enforces Tor routing)");
    }
    if onion_address.is_some() {
        log::info!("Starting zebrad with .onion hidden service (external_addr set)");
    }

    let mut child = cmd.spawn().map_err(|e| {
        format!("Failed to spawn zebrad: {}", e)
    })?;

    // Write PID file
    let pid = child.id().unwrap_or(0);
    if let Err(e) = write_pid_file(&data_dir, pid) {
        log::warn!("Failed to write PID file: {}", e);
    }
    log::info!("zebrad started with PID {}", pid);

    // Spawn log reader tasks for stdout and stderr
    let mut log_tasks = Vec::new();
    let state = app_handle.state::<AppState>();
    let node_arc = state.node.clone();

    if let Some(stdout) = child.stdout.take() {
        let dir = data_dir.clone();
        let node_ref = node_arc.clone();
        let handle = app_handle.clone();
        log_tasks.push(tokio::spawn(async move {
            read_log_stream(stdout, &dir, "stdout", node_ref, handle).await;
        }));
    }

    if let Some(stderr) = child.stderr.take() {
        let dir = data_dir.clone();
        let node_ref = node_arc.clone();
        let handle = app_handle.clone();
        log_tasks.push(tokio::spawn(async move {
            read_log_stream(stderr, &dir, "stderr", node_ref, handle).await;
        }));
    }

    // Store process and log tasks
    {
        let mut proc = node.process.lock().await;
        *proc = Some(child);
    }
    {
        let mut tasks = node.log_reader_tasks.lock().await;
        *tasks = log_tasks;
    }

    // Spawn health monitor
    let health_handle = health::spawn_health_monitor(app_handle.clone(), node_arc);
    {
        let mut ht = node.health_task.lock().await;
        *ht = Some(health_handle);
    }

    Ok(())
}

/// Stop zebrad gracefully: SIGTERM → 10s wait → SIGKILL.
pub async fn stop_zebrad(
    app_handle: &AppHandle,
    node: &NodeState,
) -> Result<(), String> {
    {
        let status = node.status.lock().await;
        match *status {
            NodeStatus::Stopped => return Ok(()),
            NodeStatus::Stopping => return Ok(()),
            _ => {}
        }
    }

    {
        let mut status = node.status.lock().await;
        *status = NodeStatus::Stopping;
    }
    let _ = app_handle.emit("node_status_changed", NodeStatus::Stopping);

    // Abort health check task
    {
        let mut ht = node.health_task.lock().await;
        if let Some(handle) = ht.take() {
            handle.abort();
        }
    }

    // Gracefully stop: SIGTERM → wait → force kill
    {
        let mut proc = node.process.lock().await;
        if let Some(ref mut child) = *proc {
            super::platform::graceful_stop(child, std::time::Duration::from_secs(10)).await;
        }
        *proc = None;
    }

    // Abort log reader tasks
    {
        let mut tasks = node.log_reader_tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }
    }

    // Remove PID file
    let data_dir = node.data_dir.lock().await.clone();
    let _ = remove_pid_file(&data_dir);

    // Set status to Stopped
    {
        let mut status = node.status.lock().await;
        *status = NodeStatus::Stopped;
    }
    let _ = app_handle.emit("node_status_changed", NodeStatus::Stopped);

    // Update tray status
    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Some(item) = state.tray_status.lock().await.as_ref() {
            let _ = item.set_text("Status: Stopped");
        }
    }

    // Reset backoff
    {
        let mut backoff = node.backoff.lock().await;
        backoff.reset();
    }

    log::info!("zebrad stopped");
    Ok(())
}

/// Check for and clean up orphaned zebrad process from a prior crash.
pub async fn check_orphan(node: &NodeState) -> Result<(), String> {
    let data_dir = node.data_dir.lock().await.clone();
    if let Some(pid) = read_pid_file(&data_dir) {
        if super::platform::is_process_alive(pid) {
            if !super::is_process_named(pid, "zebrad") {
                log::warn!("PID {} from zebrad.pid is not a zebrad process, removing stale PID file", pid);
                let _ = remove_pid_file(&data_dir);
                return Ok(());
            }

            log::warn!("Found orphaned zebrad process (PID {}), killing it", pid);
            super::platform::send_term(pid);

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            if super::platform::is_process_alive(pid) {
                super::platform::force_kill(pid);
            }
        }
        let _ = remove_pid_file(&data_dir);
    }
    Ok(())
}

/// Resolve the path to the zebrad binary.
pub fn resolve_binary_path(app_handle: &AppHandle) -> PathBuf {
    crate::platform::resolve_sidecar_path(app_handle, "zebrad")
}

fn write_pid_file(data_dir: &Path, pid: u32) -> Result<(), std::io::Error> {
    super::write_pid_file(data_dir, "zebrad.pid", pid)
}

fn read_pid_file(data_dir: &Path) -> Option<u32> {
    let pid_path = data_dir.join("zebrad.pid");
    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn remove_pid_file(data_dir: &Path) -> Result<(), std::io::Error> {
    let pid_path = data_dir.join("zebrad.pid");
    if pid_path.exists() {
        fs::remove_file(pid_path)?;
    }
    Ok(())
}

struct StartupInfo {
    message: String,
    progress: Option<f64>,
}

/// Parse zebrad log lines into user-friendly startup messages.
fn parse_startup_message(line: &str) -> Option<StartupInfo> {
    if line.contains("Thank you for running") || line.contains("Starting zebrad") {
        Some(StartupInfo { message: "Preparing your node...".into(), progress: None })
    } else if line.contains("opening database") || line.contains("creating new database") {
        Some(StartupInfo { message: "Setting up local storage...".into(), progress: None })
    } else if line.contains("initializing network") {
        Some(StartupInfo { message: "Connecting to the Zcash network...".into(), progress: None })
    } else if line.contains("connecting to initial peer set") {
        Some(StartupInfo { message: "Finding other nodes to connect to...".into(), progress: None })
    } else if line.contains("active_initial_peer_count=") {
        let count = line.split("active_initial_peer_count=").nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.parse::<u32>().ok());
        let msg = match count {
            Some(c) => format!("Connected to {} other nodes around the world", c),
            None => "Connected to the network".into(),
        };
        Some(StartupInfo { message: msg, progress: None })
    } else if line.contains("initializing verifiers") || line.contains("starting state checkpoint validation") {
        Some(StartupInfo { message: "Getting ready to verify transactions...".into(), progress: None })
    } else if line.contains("checkpoint") && line.contains("verified") {
        // Extract block height from checkpoint logs for progress
        let height = line.split("Included(Height(").nth(1)
            .and_then(|s| s.split(')').next())
            .and_then(|s| s.parse::<u64>().ok());
        let pct = height.map(|h| (h as f64 / 3_300_000.0 * 100.0).min(99.0));
        let msg = match height {
            Some(h) if h > 1000 => format!("Verifying blockchain history — block {}", format_number(h)),
            _ => "Verifying blockchain history...".into(),
        };
        Some(StartupInfo { message: msg, progress: pct })
    } else if line.contains("sync_percent=") {
        // Only extract progress number — checkpoint lines provide the user-facing message
        let pct = line.split("sync_percent=").nth(1)
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.strip_suffix('%').or(Some(s)))
            .and_then(|s| s.parse::<f64>().ok());
        pct.map(|p| StartupInfo { message: String::new(), progress: Some(p) })
    } else if line.contains("Opened RPC endpoint") {
        Some(StartupInfo { message: "Almost ready...".into(), progress: None })
    } else {
        None
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

async fn check_port_available(port: u16) -> Result<(), String> {
    match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await {
        Ok(_) => Ok(()),
        Err(_) => Err(format!(
            "Port {} is already in use. Another instance may be running.",
            port
        )),
    }
}

async fn read_log_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    data_dir: &Path,
    label: &str,
    node: Arc<NodeState>,
    app_handle: AppHandle,
) {
    let log_path = data_dir.join("logs").join("zebrad.log");
    let mut file = match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to open log file: {}", e);
            return;
        }
    };

    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let formatted = format!("[{}] {}", label, line);
        let file_line = format!("{}\n", formatted);
        let _ = tokio::io::AsyncWriteExt::write_all(&mut file, file_line.as_bytes()).await;

        // Push to in-memory log buffer
        {
            let mut buffer = node.log_buffer.lock().await;
            if buffer.len() >= LOG_BUFFER_CAPACITY {
                buffer.pop_front();
            }
            buffer.push_back(formatted.clone());
        }

        // Emit to frontend
        let _ = app_handle.emit("log_line", &formatted);

        // Parse real status from zebrad output and update Starting message.
        // Only update if status is still Starting (health monitor may have set Running).
        {
            let is_starting = matches!(*node.status.lock().await, NodeStatus::Starting { .. });
            if is_starting {
                if let Some(info) = parse_startup_message(&line) {
                    // Re-acquire lock and re-check before writing (avoids TOCTOU race
                    // where health monitor sets Running between our check and write).
                    let mut status = node.status.lock().await;
                    if let NodeStatus::Starting { ref message, .. } = *status {
                        // If message is empty (progress-only update), keep previous message
                        let new_message = if info.message.is_empty() {
                            message.clone()
                        } else {
                            Some(info.message)
                        };
                        let new_status = NodeStatus::Starting { message: new_message, progress: info.progress };
                        *status = new_status.clone();
                        drop(status);
                        let _ = app_handle.emit("node_status_changed", &new_status);
                    }
                }
            }
        }
    }
}
