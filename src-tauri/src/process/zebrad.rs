use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::zebrad_config;
use crate::health;
use crate::state::{AppState, NodeState, NodeStatus};

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
        *status = NodeStatus::Starting;
    }
    let _ = app_handle.emit("node_status_changed", NodeStatus::Starting);

    let data_dir = node.data_dir.lock().await.clone();

    // Generate config
    let config_path = zebrad_config::write_zebrad_config(&data_dir, None)
        .map_err(|e| format!("Failed to write zebrad config: {}", e))?;

    // Resolve binary path
    let binary_path = resolve_binary_path(&app_handle);
    if !binary_path.exists() {
        let mut status = node.status.lock().await;
        *status = NodeStatus::Error {
            message: "zebrad binary not found".into(),
        };
        let _ = app_handle.emit("node_status_changed", &*status);
        return Err(format!("zebrad binary not found at {:?}", binary_path));
    }

    // Spawn zebrad process
    let mut child = tokio::process::Command::new(&binary_path)
        .arg("--config")
        .arg(&config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| {
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

    if let Some(stdout) = child.stdout.take() {
        let dir = data_dir.clone();
        log_tasks.push(tokio::spawn(async move {
            read_log_stream(stdout, &dir, "stdout").await;
        }));
    }

    if let Some(stderr) = child.stderr.take() {
        let dir = data_dir.clone();
        log_tasks.push(tokio::spawn(async move {
            read_log_stream(stderr, &dir, "stderr").await;
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
    let state = app_handle.state::<AppState>();
    let node_arc = state.node.clone();
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

    // Send SIGTERM, wait, then SIGKILL if needed
    {
        let mut proc = node.process.lock().await;
        if let Some(ref mut child) = *proc {
            if let Some(pid) = child.id() {
                let nix_pid = Pid::from_raw(pid as i32);
                let _ = signal::kill(nix_pid, Signal::SIGTERM);

                // Wait up to 10 seconds
                let wait_result = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    child.wait(),
                )
                .await;

                if wait_result.is_err() {
                    log::warn!("zebrad did not exit after SIGTERM, sending SIGKILL");
                    let _ = child.kill().await;
                }
            } else {
                // No PID available, try kill directly
                let _ = child.kill().await;
            }
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
        let nix_pid = Pid::from_raw(pid as i32);
        // Check if process is alive (signal 0 = check existence)
        if signal::kill(nix_pid, None).is_ok() {
            log::warn!("Found orphaned zebrad process (PID {}), killing it", pid);
            let _ = signal::kill(nix_pid, Signal::SIGTERM);

            // Wait briefly for termination
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            // Force kill if still alive
            if signal::kill(nix_pid, None).is_ok() {
                let _ = signal::kill(nix_pid, Signal::SIGKILL);
            }
        }
        let _ = remove_pid_file(&data_dir);
    }
    Ok(())
}

/// Resolve the path to the zebrad binary.
/// In development: looks relative to the Cargo manifest dir.
/// In production: uses Tauri's resource directory.
pub fn resolve_binary_path(app_handle: &AppHandle) -> PathBuf {
    let target_triple = "aarch64-apple-darwin";
    let binary_name = format!("zebrad-{}", target_triple);

    // In dev mode, look in src-tauri/binaries/
    if cfg!(debug_assertions) {
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&binary_name);
        if dev_path.exists() {
            return dev_path;
        }
    }

    // Production: Tauri resource dir
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let prod_path = resource_dir.join(&binary_name);
        if prod_path.exists() {
            return prod_path;
        }
    }

    // Fallback: relative to executable
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default();
    exe_dir.join(&binary_name)
}

fn write_pid_file(data_dir: &Path, pid: u32) -> Result<(), std::io::Error> {
    let pid_path = data_dir.join("zebrad.pid");
    fs::write(pid_path, pid.to_string())
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

async fn read_log_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    data_dir: &Path,
    label: &str,
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
        let formatted = format!("[{}] {}\n", label, line);
        let _ = tokio::io::AsyncWriteExt::write_all(&mut file, formatted.as_bytes()).await;
    }
}
