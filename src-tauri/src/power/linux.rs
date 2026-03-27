//! Linux power management (no-op sleep/wake) and XDG autostart management.

use tauri::AppHandle;
use tokio::task::JoinHandle;

/// Spawn the power monitor — no-op on Linux.
/// The periodic 2s health checks will detect unresponsive zebrad after wake.
pub fn spawn_power_monitor(
    _app_handle: AppHandle,
) -> (std::thread::JoinHandle<()>, JoinHandle<()>) {
    let thread_handle = std::thread::Builder::new()
        .name("power-monitor".into())
        .spawn(|| {
            // No-op: Linux sleep/wake is handled by health checks
            log::info!("Power monitor: no-op on Linux (relying on health checks)");
            // Park the thread so it doesn't exit immediately
            // (the caller expects the handle to remain valid)
            std::thread::park();
        })
        .expect("failed to spawn power monitor thread");

    let wake_task = tokio::spawn(async {
        // No-op: never receives events. Use an unbounded channel that never sends.
        let (_tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        rx.recv().await;
    });

    (thread_handle, wake_task)
}

pub fn stop_power_monitor() {
    // No-op on Linux
}

// --- XDG autostart management ---

const DESKTOP_ENTRY_NAME: &str = "com.zecbox.app.desktop";

fn autostart_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".config")
        })
        .join("autostart")
        .join(DESKTOP_ENTRY_NAME)
}

pub fn install_launch_agent() -> Result<(), String> {
    let app_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get app path: {}", e))?;

    let desktop_entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=ZecBox\n\
         Comment=One-click Zcash full node\n\
         Exec={}\n\
         Terminal=false\n\
         Hidden=false\n\
         X-GNOME-Autostart-enabled=true\n",
        app_path.display()
    );

    let path = autostart_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create autostart dir: {}", e))?;
    }
    std::fs::write(&path, desktop_entry)
        .map_err(|e| format!("Failed to write autostart desktop entry: {}", e))?;

    log::info!("Installed autostart entry at {:?}", path);
    Ok(())
}

pub fn remove_launch_agent() -> Result<(), String> {
    let path = autostart_path();
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove autostart entry: {}", e))?;
        log::info!("Removed autostart entry at {:?}", path);
    }
    Ok(())
}

pub fn is_launch_agent_installed() -> bool {
    autostart_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostart_path_not_empty() {
        let path = autostart_path();
        assert!(path.to_string_lossy().contains("autostart"));
        assert!(path.to_string_lossy().contains(DESKTOP_ENTRY_NAME));
    }

    #[test]
    fn test_desktop_entry_roundtrip() {
        let tmp_dir = std::env::temp_dir().join("zecbox-test-autostart");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let test_path = tmp_dir.join(DESKTOP_ENTRY_NAME);

        let entry = "[Desktop Entry]\nType=Application\nName=ZecBox\nExec=/usr/bin/zecbox\n";
        std::fs::write(&test_path, entry).unwrap();
        assert!(test_path.exists());

        let content = std::fs::read_to_string(&test_path).unwrap();
        assert!(content.contains("ZecBox"));

        let _ = std::fs::remove_file(&test_path);
        let _ = std::fs::remove_dir(&tmp_dir);
    }
}
