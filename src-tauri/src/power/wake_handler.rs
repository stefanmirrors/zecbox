//! Portable wake event handler: health-checks zebrad, Arti, and Zaino after system wake.

use std::time::Duration;

use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::process::{zebrad, zaino};
use crate::state::{AppState, NodeStatus};
use crate::tor;

pub enum PowerEvent {
    Wake,
}

/// Handle wake events: wait for network, then health-check and restart if needed.
pub async fn handle_wake_events(
    app_handle: AppHandle,
    mut rx: mpsc::UnboundedReceiver<PowerEvent>,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_default();

    while let Some(PowerEvent::Wake) = rx.recv().await {
        log::info!("Processing wake event: waiting 5s for network recovery");
        tokio::time::sleep(Duration::from_secs(5)).await;

        let state = app_handle.state::<AppState>();

        // Check zebrad health if it was running
        let node_was_running = {
            let status = state.node.status.lock().await;
            matches!(*status, NodeStatus::Running { .. } | NodeStatus::Starting { .. })
        };

        if node_was_running {
            let healthy = check_zebrad_health(&client, Duration::from_secs(15)).await;
            if !healthy {
                log::warn!("zebrad unresponsive after wake, restarting");
                let _ = zebrad::stop_zebrad(&app_handle, &state.node).await;
                let _ = zebrad::start_zebrad(app_handle.clone(), &state.node).await;
            } else {
                log::info!("zebrad healthy after wake");
            }
        }

        // Check Arti if stealth mode was active
        if state.stealth.is_active().await {
            let arti_alive = {
                let mut proc = state.stealth.process.lock().await;
                if let Some(ref mut child) = *proc {
                    child.try_wait().ok().flatten().is_none()
                } else {
                    false
                }
            };
            if !arti_alive {
                log::warn!("Arti unresponsive after wake, restarting stealth mode");
                let _ = tor::stop_arti(&app_handle, &state.stealth).await;
                let _ = tor::start_arti(app_handle.clone(), &state.stealth).await;
            }
        }

        // Check Zaino if wallet server was running
        let wallet_was_running = {
            let status = state.wallet.status.lock().await;
            matches!(*status, crate::state::WalletStatus::Running { .. })
        };
        if wallet_was_running {
            let zaino_alive = {
                let mut proc = state.wallet.process.lock().await;
                if let Some(ref mut child) = *proc {
                    child.try_wait().ok().flatten().is_none()
                } else {
                    false
                }
            };
            if !zaino_alive {
                log::warn!("Zaino unresponsive after wake, restarting");
                let data_dir = state.node.data_dir.lock().await.clone();
                let _ = zaino::stop_zaino(&app_handle, &state.wallet, &data_dir).await;
                let _ = zaino::start_zaino(app_handle.clone(), &state.wallet, &data_dir).await;
            }
        }
    }
}

/// Poll zebrad health over a duration, returning true if it responds.
pub async fn check_zebrad_health(client: &reqwest::Client, timeout: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + timeout;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "getinfo",
        "params": [],
        "id": 1
    });

    while tokio::time::Instant::now() < deadline {
        let result = client
            .post("http://127.0.0.1:8232")
            .json(&body)
            .send()
            .await;
        if result.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    false
}
