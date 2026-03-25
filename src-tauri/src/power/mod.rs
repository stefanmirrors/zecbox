//! macOS sleep/wake handling via IOKit power notifications,
//! and launchd agent management for auto-start at login.

use std::ffi::c_void;
use std::time::Duration;

use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::process::{zebrad, zaino};
use crate::state::{AppState, NodeStatus};
use crate::tor;

// --- IOKit FFI bindings ---

type IONotificationPortRef = *mut c_void;
type IOReturn = i32;
type IOObject = u32;

#[allow(non_upper_case_globals)]
const kIOMessageSystemHasPoweredOn: u32 = 0xe000_0300;
#[allow(non_upper_case_globals)]
const kIOMessageSystemWillSleep: u32 = 0xe000_0280;
#[allow(non_upper_case_globals)]
const kIOMessageCanSystemSleep: u32 = 0xe000_0270;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IORegisterForSystemPower(
        refcon: *mut c_void,
        the_port_ref: *mut IONotificationPortRef,
        callback: extern "C" fn(
            refcon: *mut c_void,
            service: IOObject,
            message_type: u32,
            message_argument: *mut c_void,
        ),
        notifier: *mut IOObject,
    ) -> IOObject;

    fn IODeregisterForSystemPower(notifier: *mut IOObject) -> IOReturn;

    fn IONotificationPortGetRunLoopSource(notify_port: IONotificationPortRef)
        -> core_foundation::runloop::CFRunLoopSourceRef;

    fn IOAllowPowerChange(kernel_port: IOObject, notification_id: isize) -> IOReturn;
}

// Shared sender for the power callback to communicate with the async handler
static mut POWER_SENDER: Option<mpsc::UnboundedSender<PowerEvent>> = None;
static mut ROOT_PORT: IOObject = 0;

enum PowerEvent {
    Wake,
}

extern "C" fn power_callback(
    _refcon: *mut c_void,
    _service: IOObject,
    message_type: u32,
    message_argument: *mut c_void,
) {
    #[allow(non_upper_case_globals)]
    match message_type {
        kIOMessageSystemHasPoweredOn => {
            log::info!("System woke from sleep");
            unsafe {
                if let Some(ref sender) = POWER_SENDER {
                    let _ = sender.send(PowerEvent::Wake);
                }
            }
        }
        kIOMessageSystemWillSleep => {
            log::info!("System going to sleep");
            unsafe {
                IOAllowPowerChange(ROOT_PORT, message_argument as isize);
            }
        }
        kIOMessageCanSystemSleep => {
            unsafe {
                IOAllowPowerChange(ROOT_PORT, message_argument as isize);
            }
        }
        _ => {}
    }
}

/// Spawn the power monitor: a std::thread for the CFRunLoop + a tokio task for wake handling.
/// Returns the thread handle and tokio task handle.
pub fn spawn_power_monitor(
    app_handle: AppHandle,
) -> (std::thread::JoinHandle<()>, JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();

    // Store sender in static for use by the C callback
    unsafe {
        POWER_SENDER = Some(tx);
    }

    // Spawn dedicated thread for CFRunLoop (IOKit requirement)
    let thread_handle = std::thread::Builder::new()
        .name("power-monitor".into())
        .spawn(move || {
            let mut notify_port: IONotificationPortRef = std::ptr::null_mut();
            let mut notifier: IOObject = 0;

            unsafe {
                ROOT_PORT = IORegisterForSystemPower(
                    std::ptr::null_mut(),
                    &mut notify_port,
                    power_callback,
                    &mut notifier,
                );

                if ROOT_PORT == 0 {
                    log::error!("Failed to register for system power notifications");
                    return;
                }

                let source = IONotificationPortGetRunLoopSource(notify_port);
                let run_loop = CFRunLoop::get_current();
                let cf_source = core_foundation::runloop::CFRunLoopSource::wrap_under_get_rule(source);
                run_loop.add_source(&cf_source, kCFRunLoopDefaultMode);

                log::info!("Power monitor registered, running CFRunLoop");
                CFRunLoop::run_current();

                // Cleanup on exit
                IODeregisterForSystemPower(&mut notifier);
            }
        })
        .expect("failed to spawn power monitor thread");

    // Spawn async wake handler
    let wake_task = tokio::spawn(handle_wake_events(app_handle, rx));

    (thread_handle, wake_task)
}

/// Handle wake events: wait for network, then health-check and restart if needed.
async fn handle_wake_events(
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

        // Check Arti if shield mode was active
        if state.shield.is_active().await {
            let arti_alive = {
                let mut proc = state.shield.process.lock().await;
                if let Some(ref mut child) = *proc {
                    child.try_wait().ok().flatten().is_none()
                } else {
                    false
                }
            };
            if !arti_alive {
                log::warn!("Arti unresponsive after wake, restarting shield mode");
                let _ = tor::stop_arti(&app_handle, &state.shield).await;
                let _ = tor::start_arti(app_handle.clone(), &state.shield).await;
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
async fn check_zebrad_health(client: &reqwest::Client, timeout: Duration) -> bool {
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

/// Stop the power monitor CFRunLoop from another thread.
pub fn stop_power_monitor() {
    unsafe {
        POWER_SENDER = None;
    }
    // CFRunLoop will exit when no more sources remain or when explicitly stopped.
    // We'll stop it by dropping the sender which causes the rx loop to end.
}

// --- launchd agent management ---

const LAUNCH_AGENT_LABEL: &str = "com.zecbox.app";

fn launch_agent_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", LAUNCH_AGENT_LABEL))
}

pub fn install_launch_agent() -> Result<(), String> {
    let app_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get app path: {}", e))?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>"#,
        LAUNCH_AGENT_LABEL,
        app_path.display()
    );

    let path = launch_agent_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;
    }
    std::fs::write(&path, plist)
        .map_err(|e| format!("Failed to write launch agent plist: {}", e))?;

    log::info!("Installed launch agent at {:?}", path);
    Ok(())
}

pub fn remove_launch_agent() -> Result<(), String> {
    let path = launch_agent_path();
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove launch agent: {}", e))?;
        log::info!("Removed launch agent at {:?}", path);
    }
    Ok(())
}

pub fn is_launch_agent_installed() -> bool {
    launch_agent_path().exists()
}
