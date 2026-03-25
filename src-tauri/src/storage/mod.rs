use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sysinfo::Disks;
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;

use crate::process::{zebrad, zaino};
use crate::state::{AppState, NodeState, NodeStatus, StorageInfo, StorageState, StorageWarningLevel, VolumeInfo};
use crate::tor;

const WARN_THRESHOLD: u64 = 50_000_000_000;
const CRITICAL_THRESHOLD: u64 = 10_000_000_000;
const PAUSE_THRESHOLD: u64 = 2_000_000_000;
const MONITOR_INTERVAL: Duration = Duration::from_secs(60);

/// macOS system volume mount points to filter out.
const MACOS_SYSTEM_MOUNTS: &[&str] = &[
    "/System/Volumes/VM",
    "/System/Volumes/Preboot",
    "/System/Volumes/Update",
    "/System/Volumes/xarts",
    "/System/Volumes/iSCPreboot",
    "/System/Volumes/Hardware",
    "/System/Volumes/Data",
    "/private/var/vm",
];

pub fn enumerate_volumes() -> Vec<VolumeInfo> {
    let disks = Disks::new_with_refreshed_list();
    let mut volumes = Vec::new();

    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy().to_string();

        // Filter out macOS internal system volumes
        if MACOS_SYSTEM_MOUNTS.iter().any(|m| mount == *m) {
            continue;
        }
        // Skip /dev mount points
        if mount.starts_with("/dev") {
            continue;
        }

        let name = disk.name().to_string_lossy().to_string();
        let display_name = if name.is_empty() {
            if mount == "/" {
                "Macintosh HD".to_string()
            } else {
                mount
                    .rsplit('/')
                    .next()
                    .unwrap_or("Unknown")
                    .to_string()
            }
        } else {
            name
        };

        volumes.push(VolumeInfo {
            name: display_name,
            mount_point: mount,
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            is_removable: disk.is_removable(),
        });
    }

    volumes
}

pub fn get_data_dir_storage(data_dir: &Path) -> Result<StorageInfo, String> {
    let disks = Disks::new_with_refreshed_list();

    // Find the disk containing this path by longest mount point prefix match
    let data_dir_str = data_dir.to_string_lossy();
    let mut best_match: Option<(&sysinfo::Disk, usize)> = None;

    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy().to_string();
        if data_dir_str.starts_with(&mount) {
            let len = mount.len();
            if best_match.is_none() || len > best_match.unwrap().1 {
                best_match = Some((disk, len));
            }
        }
    }

    let disk = best_match
        .map(|(d, _)| d)
        .ok_or_else(|| format!("No volume found for path: {}", data_dir.display()))?;

    let available = disk.available_space();
    let mount_str = disk.mount_point().to_string_lossy().to_string();
    let name = disk.name().to_string_lossy().to_string();
    let display_name = if name.is_empty() {
        if mount_str == "/" {
            "Macintosh HD".to_string()
        } else {
            mount_str
                .rsplit('/')
                .next()
                .unwrap_or("Unknown")
                .to_string()
        }
    } else {
        name
    };

    Ok(StorageInfo {
        data_dir: data_dir.to_string_lossy().to_string(),
        volume_name: display_name,
        total_bytes: disk.total_space(),
        available_bytes: available,
        is_external: is_external_volume(disk.mount_point()),
        warning_level: warning_level(available),
    })
}

pub fn warning_level(available_bytes: u64) -> StorageWarningLevel {
    if available_bytes < PAUSE_THRESHOLD {
        StorageWarningLevel::Paused
    } else if available_bytes < CRITICAL_THRESHOLD {
        StorageWarningLevel::Critical
    } else if available_bytes < WARN_THRESHOLD {
        StorageWarningLevel::Warning
    } else {
        StorageWarningLevel::None
    }
}

pub fn is_external_volume(mount_point: &Path) -> bool {
    let mount_str = mount_point.to_string_lossy();
    mount_str.starts_with("/Volumes/")
}

pub fn is_mount_available(data_dir: &Path) -> bool {
    // For the root volume, always available
    if data_dir.starts_with("/Users") || data_dir.starts_with("/Applications") {
        return true;
    }

    // For external volumes (/Volumes/X/...), check the mount point exists
    let path_str = data_dir.to_string_lossy();
    if path_str.starts_with("/Volumes/") {
        // Extract /Volumes/DriveName
        let parts: Vec<&str> = path_str.splitn(4, '/').collect();
        if parts.len() >= 3 {
            let mount_point = format!("/{}/{}", parts[1], parts[2]);
            return Path::new(&mount_point).exists();
        }
    }

    // Default: check if the path's parent exists
    data_dir.exists() || data_dir.parent().map_or(false, |p| p.exists())
}

pub fn spawn_storage_monitor(
    app_handle: AppHandle,
    node: Arc<NodeState>,
    storage: Arc<StorageState>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(MONITOR_INTERVAL);
        loop {
            interval.tick().await;

            let data_dir = node.data_dir.lock().await.clone();

            // Check if mount is available (external drive detection)
            if !is_mount_available(&data_dir) {
                let mut connected = storage.drive_connected.lock().await;
                if *connected {
                    *connected = false;
                    let _ = app_handle.emit("storage_drive_disconnected", ());
                    log::warn!("External drive disconnected: {:?}", data_dir);

                    // Stop all running processes that depend on this drive
                    let status = node.status.lock().await.clone();
                    if matches!(status, NodeStatus::Running { .. } | NodeStatus::Starting { .. }) {
                        log::warn!("Stopping zebrad due to drive disconnect");
                        let _ = zebrad::stop_zebrad(&app_handle, &node).await;
                    }
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if !state.wallet.status.lock().await.is_stopped_or_error() {
                            let _ = zaino::stop_zaino(&app_handle, &state.wallet, &data_dir).await;
                        }
                        if state.shield.is_active().await {
                            let _ = tor::stop_arti(&app_handle, &state.shield).await;
                        }
                    }
                }
                continue;
            } else {
                let mut connected = storage.drive_connected.lock().await;
                if !*connected {
                    *connected = true;
                    let _ = app_handle.emit("storage_drive_reconnected", ());
                    log::info!("External drive reconnected: {:?}", data_dir);
                }
            }

            // Check disk space
            match get_data_dir_storage(&data_dir) {
                Ok(info) => {
                    let _ = app_handle.emit("storage_info_updated", &info);

                    match info.warning_level {
                        StorageWarningLevel::Paused => {
                            let mut paused = storage.paused_low_space.lock().await;
                            if !*paused {
                                *paused = true;
                                log::error!("Disk space critically low (<2GB), pausing node");
                                let _ = app_handle.emit(
                                    "storage_node_paused",
                                    "Disk space critically low. Node paused to prevent data corruption.",
                                );
                                // Only stop if node is actually running
                                let status = node.status.lock().await.clone();
                                if matches!(status, NodeStatus::Running { .. } | NodeStatus::Starting { .. }) {
                                    drop(status);
                                    let _ = zebrad::stop_zebrad(&app_handle, &node).await;
                                }
                            }
                        }
                        StorageWarningLevel::Critical => {
                            let _ = app_handle.emit("storage_warning", "critical");
                        }
                        StorageWarningLevel::Warning => {
                            let _ = app_handle.emit("storage_warning", "warning");
                        }
                        StorageWarningLevel::None => {
                            let mut paused = storage.paused_low_space.lock().await;
                            *paused = false;
                        }
                    }
                }
                Err(e) => log::warn!("Failed to check storage: {}", e),
            }
        }
    })
}
