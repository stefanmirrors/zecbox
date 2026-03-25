use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;

use crate::process::zebrad;
use crate::state::{AppState, NodeState, NodeStatus};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONSECUTIVE_FAILURES: u32 = 10;
const RECOVERY_THRESHOLD: u32 = 5;

async fn update_tray_status(app_handle: &AppHandle, status: &NodeStatus) {
    if let Some(state) = app_handle.try_state::<AppState>() {
        if let Some(item) = state.tray_status.lock().await.as_ref() {
            let text = match status {
                NodeStatus::Running { block_height, sync_percentage, .. } => {
                    if let Some(pct) = sync_percentage {
                        if *pct < 99.9 {
                            format!("Syncing: {:.1}% ({})", pct, block_height)
                        } else {
                            format!("Block: {}", block_height)
                        }
                    } else {
                        format!("Block: {}", block_height)
                    }
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
        let client = match reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to initialize health monitor: {}", e);
                return;
            }
        };

        let mut consecutive_failures: u32 = 0;

        // Brief initial delay to let the process start up
        tokio::time::sleep(Duration::from_secs(1)).await;

        let mut startup_polls: u32 = 0;
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

            // Track startup polls to show feedback while waiting for RPC
            let is_starting = {
                let status = node.status.lock().await;
                matches!(*status, NodeStatus::Starting { .. })
            };
            if is_starting {
                startup_polls += 1;
                // Emit progress messages so the user sees something happening
                let msg = match startup_polls {
                    1..=3 => "Initializing node...",
                    4..=8 => "Opening database...",
                    9..=15 => "Connecting to peers...",
                    16..=30 => "Waiting for RPC to become available...",
                    _ => "Still starting up — this can take a few minutes on first launch...",
                };
                let starting_status = NodeStatus::Starting { message: Some(msg.to_string()) };
                {
                    let mut status = node.status.lock().await;
                    *status = starting_status.clone();
                }
                let _ = app_handle.emit("node_status_changed", &starting_status);
            }

            match poll_zebrad(&client).await {
                Ok(poll_result) => {
                    consecutive_failures = 0;
                    startup_polls = 0;

                    let sync_pct = poll_result.estimated_height.map(|est| {
                        if est == 0 { 0.0 } else {
                            ((poll_result.block_height as f64 / est as f64) * 100.0).min(100.0)
                        }
                    });

                    let new_status = NodeStatus::Running {
                        block_height: poll_result.block_height,
                        peer_count: poll_result.peer_count,
                        estimated_height: poll_result.estimated_height,
                        best_block_hash: poll_result.best_block_hash,
                        sync_percentage: sync_pct,
                        chain: poll_result.chain,
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

                    // Update stats
                    {
                        let mut stats = node.stats.lock().await;
                        let mut last_height = node.last_block_height.lock().await;
                        stats.record_uptime_tick(POLL_INTERVAL.as_secs());
                        stats.record_blocks(poll_result.block_height, *last_height);
                        stats.update_streak();
                        *last_height = poll_result.block_height;
                        let data_dir = node.data_dir.lock().await.clone();
                        stats.save(&data_dir);
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

                            // Check if we've hit the recovery threshold
                            let failures = {
                                let backoff = node.backoff.lock().await;
                                backoff.consecutive_failures
                            };
                            if failures >= RECOVERY_THRESHOLD {
                                log::error!(
                                    "zebrad failed {} consecutive restarts — possible DB corruption",
                                    failures
                                );
                                {
                                    let mut status = node.status.lock().await;
                                    *status = NodeStatus::Error {
                                        message: "Node failed to start repeatedly. Database may be corrupted. Consider rebuilding.".into(),
                                    };
                                }
                                let error_status = node.status.lock().await.clone();
                                let _ = app_handle.emit("node_status_changed", &error_status);
                                let _ = app_handle.emit("node_recovery_needed", "Node failed to start after multiple attempts. The database may need to be rebuilt.");
                                update_tray_status(&app_handle, &error_status).await;
                                // Clean up dead process
                                {
                                    let mut proc = node.process.lock().await;
                                    *proc = None;
                                }
                                break;
                            }

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

struct PollResult {
    block_height: u64,
    peer_count: u32,
    estimated_height: Option<u64>,
    best_block_hash: Option<String>,
    chain: Option<String>,
}

async fn poll_zebrad(client: &reqwest::Client) -> Result<PollResult, String> {
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
        .map_err(|_| "Node is not responding".to_string())?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| "Node returned an invalid response".to_string())?;

    if let Some(error) = json.get("error").and_then(|e| e.as_object()) {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(format!("Node reported an issue: {}", msg));
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

    // Try to get blockchain info for sync progress
    let (estimated_height, best_block_hash, chain) =
        match poll_blockchain_info(client).await {
            Ok(info) => info,
            Err(_) => (None, None, None),
        };

    Ok(PollResult {
        block_height,
        peer_count,
        estimated_height,
        best_block_hash,
        chain,
    })
}

async fn poll_blockchain_info(
    client: &reqwest::Client,
) -> Result<(Option<u64>, Option<String>, Option<String>), String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "getblockchaininfo",
        "params": [],
        "id": 2
    });

    let resp = client
        .post("http://127.0.0.1:8232")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let result = json.get("result").ok_or("Missing result")?;

    let estimated_height = result
        .get("estimatedheight")
        .and_then(|v| v.as_u64());

    let best_block_hash = result
        .get("bestblockhash")
        .and_then(|v| v.as_str())
        .map(String::from);

    let chain = result
        .get("chain")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok((estimated_height, best_block_hash, chain))
}
