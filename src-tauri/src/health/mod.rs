use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;

use crate::process::zebrad;
use crate::state::{AppState, NodeState, NodeStatus};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

async fn update_tray_status(app_handle: &AppHandle, status: &NodeStatus) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Some(item) = state.tray_status.lock().await.as_ref() {
            let text = match status {
                NodeStatus::Running { block_height, .. } => {
                    format!("Block: {}", block_height)
                }
                _ => format!("Status: {}", capitalize(status.status_str())),
            };
            let _ = item.set_text(text);
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().chain(c).collect(),
    }
}

/// Spawn the health monitor task. Returns a handle that can be aborted to stop monitoring.
pub fn spawn_health_monitor(
    app_handle: AppHandle,
    node: Arc<NodeState>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .expect("failed to build HTTP client");

        let mut consecutive_failures: u32 = 0;

        // Brief initial delay to let the process start up
        tokio::time::sleep(Duration::from_secs(1)).await;

        let mut interval = tokio::time::interval(POLL_INTERVAL);
        loop {
            interval.tick().await;

            // Check if we should still be running
            {
                let status = node.status.lock().await;
                match *status {
                    NodeStatus::Stopped | NodeStatus::Stopping => break,
                    _ => {}
                }
            }

            match poll_zebrad(&client).await {
                Ok((block_height, peer_count)) => {
                    consecutive_failures = 0;

                    let new_status = NodeStatus::Running {
                        block_height,
                        peer_count,
                    };

                    {
                        let mut status = node.status.lock().await;
                        *status = new_status.clone();
                    }
                    let _ = app_handle.emit("node_status_changed", &new_status);
                    update_tray_status(&app_handle, &new_status).await;

                    // Track healthy duration for backoff reset
                    {
                        let mut backoff = node.backoff.lock().await;
                        backoff.mark_healthy();
                    }
                }
                Err(e) => {
                    consecutive_failures += 1;
                    log::warn!(
                        "Health check failed ({}/{}): {}",
                        consecutive_failures,
                        MAX_CONSECUTIVE_FAILURES,
                        e
                    );

                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        // Check if process is actually dead
                        let process_dead = {
                            let mut proc = node.process.lock().await;
                            if let Some(ref mut child) = *proc {
                                child.try_wait().ok().flatten().is_some()
                            } else {
                                true
                            }
                        };

                        if process_dead {
                            log::error!("zebrad process has died, attempting restart");

                            let delay_secs = {
                                let mut backoff = node.backoff.lock().await;
                                backoff.next_delay()
                            };

                            {
                                let mut status = node.status.lock().await;
                                *status = NodeStatus::Error {
                                    message: format!(
                                        "Node crashed. Restarting in {}s...",
                                        delay_secs
                                    ),
                                };
                            }
                            let error_status = {
                                let status = node.status.lock().await;
                                status.clone()
                            };
                            let _ = app_handle.emit("node_status_changed", &error_status);
                            update_tray_status(&app_handle, &error_status).await;

                            // Clean up dead process
                            {
                                let mut proc = node.process.lock().await;
                                *proc = None;
                            }

                            // Wait for backoff
                            tokio::time::sleep(Duration::from_secs(delay_secs)).await;

                            // Attempt restart
                            match zebrad::start_zebrad(app_handle.clone(), &node).await {
                                Ok(()) => {
                                    log::info!("zebrad restarted successfully");
                                }
                                Err(e) => {
                                    log::error!("Failed to restart zebrad: {}", e);
                                    let mut status = node.status.lock().await;
                                    *status = NodeStatus::Error {
                                        message: format!("Restart failed: {}", e),
                                    };
                                    let _ = app_handle.emit(
                                        "node_status_changed",
                                        &*status,
                                    );
                                    update_tray_status(&app_handle, &*status).await;
                                    break;
                                }
                            }
                            // After successful restart, a new health monitor is spawned
                            // by start_zebrad, so this one should exit
                            break;
                        }
                    }
                }
            }
        }
    })
}

async fn poll_zebrad(client: &reqwest::Client) -> Result<(u64, u32), String> {
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
        .map_err(|e| format!("Connection failed: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid response: {}", e))?;

    if let Some(error) = json.get("error").and_then(|e| e.as_object()) {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("RPC error: {}", msg));
    }

    let result = json
        .get("result")
        .ok_or("Missing 'result' field")?;

    let block_height = result
        .get("blocks")
        .and_then(|b| b.as_u64())
        .unwrap_or(0);

    let peer_count = result
        .get("connections")
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u32;

    Ok((block_height, peer_count))
}
