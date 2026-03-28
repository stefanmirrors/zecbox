//! Windows power management (no-op sleep/wake) and Registry autostart management.

use tauri::AppHandle;
use tokio::task::JoinHandle;

/// Spawn the power monitor — no-op on Windows.
/// The periodic 2s health checks will detect unresponsive zebrad after wake.
pub fn spawn_power_monitor(
    _app_handle: AppHandle,
) -> (std::thread::JoinHandle<()>, JoinHandle<()>) {
    let thread_handle = std::thread::Builder::new()
        .name("power-monitor".into())
        .spawn(|| {
            log::info!("Power monitor: no-op on Windows (relying on health checks)");
            std::thread::park();
        })
        .expect("failed to spawn power monitor thread");

    let wake_task = tokio::spawn(async {
        let (_tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        rx.recv().await;
    });

    (thread_handle, wake_task)
}

pub fn stop_power_monitor() {
    // No-op on Windows
}

// --- Windows Registry autostart management ---

const REG_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const REG_VALUE: &str = "ZecBox";

pub fn install_launch_agent() -> Result<(), String> {
    let app_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get app path: {}", e))?;

    let output = std::process::Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\{}", REG_KEY),
            "/v",
            REG_VALUE,
            "/t",
            "REG_SZ",
            "/d",
            &app_path.display().to_string(),
            "/f",
        ])
        .output()
        .map_err(|e| format!("Failed to set registry key: {}", e))?;

    if !output.status.success() {
        return Err("Failed to add auto-start registry entry".into());
    }

    log::info!("Installed autostart registry entry for ZecBox");
    Ok(())
}

pub fn remove_launch_agent() -> Result<(), String> {
    let _ = std::process::Command::new("reg")
        .args([
            "delete",
            &format!("HKCU\\{}", REG_KEY),
            "/v",
            REG_VALUE,
            "/f",
        ])
        .output();
    log::info!("Removed autostart registry entry for ZecBox");
    Ok(())
}

pub fn is_launch_agent_installed() -> bool {
    std::process::Command::new("reg")
        .args([
            "query",
            &format!("HKCU\\{}", REG_KEY),
            "/v",
            REG_VALUE,
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
