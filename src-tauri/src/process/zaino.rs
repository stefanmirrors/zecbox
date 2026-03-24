use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task::JoinHandle;

use crate::config::zaino_config;
use crate::state::{AppState, WalletState, WalletStatus, LOG_BUFFER_CAPACITY};

const GRPC_PORT: u16 = 9067;
const HEALTH_POLL_INTERVAL: Duration = Duration::from_secs(2);
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// Start Zaino, spawn log readers and health monitor.
pub async fn start_zaino(
    app_handle: AppHandle,
    wallet: &WalletState,
    data_dir: &Path,
) -> Result<(), String> {
    {
        let status = wallet.status.lock().await;
        if !status.is_stopped_or_error() {
            return Err(format!(
                "Cannot start Zaino: currently {}",
                status.status_str()
            ));
        }
    }

    {
        let mut status = wallet.status.lock().await;
        *status = WalletStatus::Starting;
    }
    emit_wallet_status(&app_handle, wallet).await;

    // Generate config
    let config_path = zaino_config::write_zaino_config(data_dir)
        .map_err(|e| format!("Failed to write zaino config: {}", e))?;

    // Resolve binary path
    let binary_path = resolve_zaino_binary_path(&app_handle);
    if !binary_path.exists() {
        let mut status = wallet.status.lock().await;
        *status = WalletStatus::Error {
            message: "Wallet server binary not found. Try reinstalling ZecBox.".into(),
        };
        emit_wallet_status(&app_handle, wallet).await;
        return Err(format!("Zaino binary not found at {:?}", binary_path));
    }

    // Check for port conflict before spawning
    if let Err(msg) = check_port_available(GRPC_PORT).await {
        let mut status = wallet.status.lock().await;
        *status = WalletStatus::Error { message: msg.clone() };
        emit_wallet_status(&app_handle, wallet).await;
        return Err(msg);
    }

    let mut child = tokio::process::Command::new(&binary_path)
        .arg("--config")
        .arg(&config_path)
        .arg("--grpc-port")
        .arg(GRPC_PORT.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(false)
        .spawn()
        .map_err(|e| format!("Failed to spawn Zaino: {}", e))?;

    // Write PID file
    let pid = child.id().unwrap_or(0);
    if let Err(e) = write_pid_file(data_dir, pid) {
        log::warn!("Failed to write Zaino PID file: {}", e);
    }
    log::info!("Zaino started with PID {}", pid);

    // Spawn log reader tasks
    let mut log_tasks = Vec::new();
    let state = app_handle.state::<AppState>();
    let wallet_arc = state.wallet.clone();

    if let Some(stdout) = child.stdout.take() {
        let dir = data_dir.to_path_buf();
        let wal = wallet_arc.clone();
        let handle = app_handle.clone();
        log_tasks.push(tokio::spawn(async move {
            read_log_stream(stdout, &dir, "stdout", wal, handle).await;
        }));
    }

    if let Some(stderr) = child.stderr.take() {
        let dir = data_dir.to_path_buf();
        let wal = wallet_arc.clone();
        let handle = app_handle.clone();
        log_tasks.push(tokio::spawn(async move {
            read_zaino_stderr(stderr, &dir, wal, handle).await;
        }));
    }

    // Store process and log tasks
    {
        let mut proc = wallet.process.lock().await;
        *proc = Some(child);
    }
    {
        let mut tasks = wallet.log_reader_tasks.lock().await;
        *tasks = log_tasks;
    }

    // Spawn health monitor
    let health_handle = spawn_zaino_health_monitor(
        app_handle.clone(),
        wallet_arc,
        data_dir.to_path_buf(),
    );
    {
        let mut ht = wallet.health_task.lock().await;
        *ht = Some(health_handle);
    }

    Ok(())
}

/// Stop Zaino gracefully: SIGTERM -> 5s wait -> SIGKILL.
pub async fn stop_zaino(
    app_handle: &AppHandle,
    wallet: &WalletState,
    data_dir: &Path,
) -> Result<(), String> {
    {
        let status = wallet.status.lock().await;
        match *status {
            WalletStatus::Stopped => return Ok(()),
            WalletStatus::Stopping => return Ok(()),
            _ => {}
        }
    }

    {
        let mut status = wallet.status.lock().await;
        *status = WalletStatus::Stopping;
    }
    emit_wallet_status(app_handle, wallet).await;

    // Abort health check task
    {
        let mut ht = wallet.health_task.lock().await;
        if let Some(handle) = ht.take() {
            handle.abort();
        }
    }

    // Send SIGTERM, wait, then SIGKILL
    {
        let mut proc = wallet.process.lock().await;
        if let Some(ref mut child) = *proc {
            if let Some(pid) = child.id() {
                let nix_pid = Pid::from_raw(pid as i32);
                let _ = signal::kill(nix_pid, Signal::SIGTERM);

                let wait_result = tokio::time::timeout(
                    Duration::from_secs(5),
                    child.wait(),
                )
                .await;

                if wait_result.is_err() {
                    log::warn!("Zaino did not exit after SIGTERM, sending SIGKILL");
                    let _ = child.kill().await;
                }
            } else {
                let _ = child.kill().await;
            }
        }
        *proc = None;
    }

    // Abort log reader tasks
    {
        let mut tasks = wallet.log_reader_tasks.lock().await;
        for task in tasks.drain(..) {
            task.abort();
        }
    }

    // Remove PID file
    let _ = remove_pid_file(data_dir);

    // Set status to Stopped
    {
        let mut status = wallet.status.lock().await;
        *status = WalletStatus::Stopped;
    }
    emit_wallet_status(app_handle, wallet).await;

    // Reset backoff
    {
        let mut backoff = wallet.backoff.lock().await;
        backoff.reset();
    }

    log::info!("Zaino stopped");
    Ok(())
}

/// Check for and clean up orphaned Zaino process from a prior crash.
pub async fn check_zaino_orphan(data_dir: &Path) -> Result<(), String> {
    if let Some(pid) = read_pid_file(data_dir) {
        let nix_pid = Pid::from_raw(pid as i32);
        if signal::kill(nix_pid, None).is_ok() {
            log::warn!("Found orphaned Zaino process (PID {}), killing it", pid);
            let _ = signal::kill(nix_pid, Signal::SIGTERM);

            tokio::time::sleep(Duration::from_secs(3)).await;

            if signal::kill(nix_pid, None).is_ok() {
                let _ = signal::kill(nix_pid, Signal::SIGKILL);
            }
        }
        let _ = remove_pid_file(data_dir);
    }
    Ok(())
}

/// Resolve the path to the Zaino binary.
pub fn resolve_zaino_binary_path(app_handle: &AppHandle) -> PathBuf {
    let target_triple = "aarch64-apple-darwin";
    let binary_name_with_triple = format!("zaino-{}", target_triple);
    let binary_name = "zaino";

    if cfg!(debug_assertions) {
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&binary_name_with_triple);
        if dev_path.exists() {
            return dev_path;
        }
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        let prod_path = dir.join(binary_name);
        if prod_path.exists() {
            return prod_path;
        }
        let prod_path = dir.join(&binary_name_with_triple);
        if prod_path.exists() {
            return prod_path;
        }
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let prod_path = resource_dir.join(binary_name);
        if prod_path.exists() {
            return prod_path;
        }
    }

    exe_dir.unwrap_or_default().join(binary_name)
}

/// gRPC endpoint address for display.
pub fn grpc_endpoint() -> String {
    format!("127.0.0.1:{}", GRPC_PORT)
}

fn write_pid_file(data_dir: &Path, pid: u32) -> Result<(), std::io::Error> {
    let pid_path = data_dir.join("zaino.pid");
    fs::write(pid_path, pid.to_string())
}

fn read_pid_file(data_dir: &Path) -> Option<u32> {
    let pid_path = data_dir.join("zaino.pid");
    fs::read_to_string(pid_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

fn remove_pid_file(data_dir: &Path) -> Result<(), std::io::Error> {
    let pid_path = data_dir.join("zaino.pid");
    if pid_path.exists() {
        fs::remove_file(pid_path)?;
    }
    Ok(())
}

/// Read stderr from Zaino, parse "ZAINO READY" to transition to Running.
async fn read_zaino_stderr<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    data_dir: &Path,
    wallet: Arc<WalletState>,
    app_handle: AppHandle,
) {
    let log_path = data_dir.join("logs").join("zaino.log");
    let mut file = match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to open zaino log file: {}", e);
            return;
        }
    };

    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let formatted = format!("[zaino/stderr] {}", line);
        let file_line = format!("{}\n", formatted);
        let _ = tokio::io::AsyncWriteExt::write_all(&mut file, file_line.as_bytes()).await;

        // Push to log buffer
        {
            let mut buffer = wallet.log_buffer.lock().await;
            if buffer.len() >= LOG_BUFFER_CAPACITY {
                buffer.pop_front();
            }
            buffer.push_back(formatted.clone());
        }
        let _ = app_handle.emit("log_line", &formatted);

        // Check for ready signal
        if line.contains("ZAINO READY") {
            let endpoint = grpc_endpoint();
            let mut status = wallet.status.lock().await;
            if matches!(*status, WalletStatus::Starting) {
                *status = WalletStatus::Running {
                    endpoint: endpoint.clone(),
                };
                drop(status);
                emit_wallet_status(&app_handle, &wallet).await;
                log::info!("Zaino is ready, gRPC endpoint: {}", endpoint);
            }
        }
    }
}

/// Read stdout from Zaino and stream to logs.
async fn read_log_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: R,
    data_dir: &Path,
    label: &str,
    wallet: Arc<WalletState>,
    app_handle: AppHandle,
) {
    let log_path = data_dir.join("logs").join("zaino.log");
    let mut file = match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            log::error!("Failed to open zaino log file: {}", e);
            return;
        }
    };

    let mut lines = BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let formatted = format!("[zaino/{}] {}", label, line);
        let file_line = format!("{}\n", formatted);
        let _ = tokio::io::AsyncWriteExt::write_all(&mut file, file_line.as_bytes()).await;

        {
            let mut buffer = wallet.log_buffer.lock().await;
            if buffer.len() >= LOG_BUFFER_CAPACITY {
                buffer.pop_front();
            }
            buffer.push_back(formatted.clone());
        }
        let _ = app_handle.emit("log_line", &formatted);
    }
}

/// Health monitor for Zaino: TCP connect to gRPC port every 2s.
/// Auto-restarts on crash with exponential backoff.
fn spawn_zaino_health_monitor(
    app_handle: AppHandle,
    wallet: Arc<WalletState>,
    data_dir: PathBuf,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Brief initial delay
        tokio::time::sleep(Duration::from_secs(1)).await;

        let mut consecutive_failures: u32 = 0;
        let mut interval = tokio::time::interval(HEALTH_POLL_INTERVAL);

        loop {
            interval.tick().await;

            {
                let status = wallet.status.lock().await;
                match *status {
                    WalletStatus::Stopped | WalletStatus::Stopping => break,
                    _ => {}
                }
            }

            match tokio::net::TcpStream::connect(grpc_endpoint()).await {
                Ok(_) => {
                    consecutive_failures = 0;

                    // If still Starting (ready signal not yet parsed), transition to Running
                    {
                        let mut status = wallet.status.lock().await;
                        if matches!(*status, WalletStatus::Starting) {
                            *status = WalletStatus::Running {
                                endpoint: grpc_endpoint(),
                            };
                            drop(status);
                            emit_wallet_status(&app_handle, &wallet).await;
                        }
                    }

                    {
                        let mut backoff = wallet.backoff.lock().await;
                        backoff.mark_healthy();
                    }
                }
                Err(_) => {
                    consecutive_failures += 1;
                    log::warn!(
                        "Zaino health check failed ({}/{})",
                        consecutive_failures,
                        MAX_CONSECUTIVE_FAILURES
                    );

                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        let process_dead = {
                            let mut proc = wallet.process.lock().await;
                            if let Some(ref mut child) = *proc {
                                child.try_wait().ok().flatten().is_some()
                            } else {
                                true
                            }
                        };

                        if process_dead {
                            log::error!("Zaino process has died, attempting restart");

                            let delay_secs = {
                                let mut backoff = wallet.backoff.lock().await;
                                backoff.next_delay()
                            };

                            {
                                let mut status = wallet.status.lock().await;
                                *status = WalletStatus::Error {
                                    message: format!(
                                        "Wallet server crashed. Restarting in {}s...",
                                        delay_secs
                                    ),
                                };
                            }
                            emit_wallet_status(&app_handle, &wallet).await;

                            // Clean up dead process
                            {
                                let mut proc = wallet.process.lock().await;
                                *proc = None;
                            }

                            tokio::time::sleep(Duration::from_secs(delay_secs)).await;

                            // Attempt restart
                            match start_zaino(
                                app_handle.clone(),
                                &wallet,
                                &data_dir,
                            )
                            .await
                            {
                                Ok(()) => {
                                    log::info!("Zaino restarted successfully");
                                }
                                Err(e) => {
                                    log::error!("Failed to restart Zaino: {}", e);
                                    let mut status = wallet.status.lock().await;
                                    *status = WalletStatus::Error {
                                        message: format!("Restart failed: {}", e),
                                    };
                                    emit_wallet_status(&app_handle, &wallet).await;
                                }
                            }
                            // New health monitor spawned by start_zaino
                            break;
                        }
                    }
                }
            }
        }
    })
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

async fn emit_wallet_status(app_handle: &AppHandle, wallet: &WalletState) {
    let status = wallet.status.lock().await;
    let payload = match &*status {
        WalletStatus::Stopped => serde_json::json!({
            "enabled": false,
            "status": "stopped",
        }),
        WalletStatus::Starting => serde_json::json!({
            "enabled": false,
            "status": "starting",
        }),
        WalletStatus::Running { endpoint } => serde_json::json!({
            "enabled": true,
            "status": "running",
            "endpoint": endpoint,
        }),
        WalletStatus::Stopping => serde_json::json!({
            "enabled": false,
            "status": "stopping",
        }),
        WalletStatus::Error { message } => serde_json::json!({
            "enabled": false,
            "status": "error",
            "message": message,
        }),
    };
    let _ = app_handle.emit("wallet_status_changed", payload);
}
