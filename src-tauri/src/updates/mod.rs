//! Binary update management: version check, download, SHA256 verify, swap, rollback.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tokio::task::JoinHandle;

use crate::process;
use crate::state::{AppState, BinaryUpdateInfo, UpdateState, UpdateStatus};
use crate::tor;

const MANIFEST_URL: &str = "https://zecbox.io/updates/manifest.json";
const TARGET_PLATFORM: &str = "aarch64-apple-darwin";

// --- Manifest types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub app_version: String,
    pub binaries: Vec<BinaryManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryManifestEntry {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

// --- Binary version tracking ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryVersions {
    pub zebrad: String,
    pub zaino: String,
    pub arti: String,
}

impl Default for BinaryVersions {
    fn default() -> Self {
        Self {
            zebrad: "0.0.0".into(),
            zaino: "0.0.0".into(),
            arti: "0.0.0".into(),
        }
    }
}

impl BinaryVersions {
    fn versions_path(data_dir: &Path) -> PathBuf {
        data_dir.join("config").join("binary_versions.json")
    }

    pub fn load(data_dir: &Path) -> Self {
        let path = Self::versions_path(data_dir);
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, data_dir: &Path) -> Result<(), String> {
        let path = Self::versions_path(data_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize versions: {}", e))?;
        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &contents)
            .map_err(|e| format!("Failed to write versions: {}", e))?;
        std::fs::rename(&tmp_path, &path)
            .map_err(|e| format!("Failed to rename versions file: {}", e))?;
        Ok(())
    }

    pub fn get(&self, name: &str) -> &str {
        match name {
            "zebrad" => &self.zebrad,
            "zaino" => &self.zaino,
            "arti" => &self.arti,
            _ => "0.0.0",
        }
    }

    pub fn set(&mut self, name: &str, version: String) {
        match name {
            "zebrad" => self.zebrad = version,
            "zaino" => self.zaino = version,
            "arti" => self.arti = version,
            _ => {}
        }
    }
}

// --- Binary directory resolution ---

pub fn resolve_binary_dir(app_handle: &AppHandle) -> PathBuf {
    if cfg!(debug_assertions) {
        let dev_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
        if dev_dir.exists() {
            return dev_dir;
        }
    }

    // Production: Tauri bundles externalBin in Contents/MacOS/
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        if dir.exists() {
            return dir.clone();
        }
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        return resource_dir;
    }

    exe_dir.unwrap_or_default()
}

fn binary_filename(name: &str) -> String {
    format!("{}-{}", name, TARGET_PLATFORM)
}

// --- Manifest check ---

pub async fn check_manifest(
    data_dir: &Path,
    app_handle: &AppHandle,
) -> Result<Vec<BinaryUpdateInfo>, String> {
    let manifest = fetch_manifest(data_dir).await?;
    let versions = BinaryVersions::load(data_dir);
    let binary_dir = resolve_binary_dir(app_handle);

    let mut updates = Vec::new();

    for entry in &manifest.binaries {
        if entry.platform != TARGET_PLATFORM {
            continue;
        }

        let current = versions.get(&entry.name);
        if version_is_newer(&entry.version, current) {
            // Verify binary actually exists before offering update
            let binary_path = binary_dir.join(binary_filename(&entry.name));
            let current_version = if binary_path.exists() {
                current.to_string()
            } else {
                "not installed".to_string()
            };

            updates.push(BinaryUpdateInfo {
                name: entry.name.clone(),
                current_version,
                new_version: entry.version.clone(),
                download_url: entry.download_url.clone(),
                sha256: entry.sha256.clone(),
                size_bytes: entry.size_bytes,
            });
        }
    }

    Ok(updates)
}

async fn fetch_manifest(data_dir: &Path) -> Result<UpdateManifest, String> {
    // In dev mode, check for local mock manifest first
    if cfg!(debug_assertions) {
        let mock_path = data_dir.join("config").join("mock_update_manifest.json");
        if mock_path.exists() {
            let contents = std::fs::read_to_string(&mock_path)
                .map_err(|e| format!("Failed to read mock manifest: {}", e))?;
            let manifest: UpdateManifest = serde_json::from_str(&contents)
                .map_err(|e| format!("Failed to parse mock manifest: {}", e))?;
            log::info!("Using mock update manifest from {:?}", mock_path);
            return Ok(manifest);
        }
    }

    // Fetch from remote
    let client = reqwest::Client::new();
    let response = client
        .get(MANIFEST_URL)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Manifest fetch returned {}", response.status()));
    }

    response
        .json::<UpdateManifest>()
        .await
        .map_err(|e| format!("Failed to parse manifest: {}", e))
}

fn version_is_newer(new: &str, current: &str) -> bool {
    // Simple semver comparison: split by '.', compare numerically
    let parse = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let new_parts = parse(new);
    let current_parts = parse(current);

    for i in 0..new_parts.len().max(current_parts.len()) {
        let n = new_parts.get(i).copied().unwrap_or(0);
        let c = current_parts.get(i).copied().unwrap_or(0);
        if n > c {
            return true;
        }
        if n < c {
            return false;
        }
    }

    false
}

// --- Download ---

pub async fn download_binary(url: &str, dest_path: &Path) -> Result<(), String> {
    let tmp_path = dest_path.with_extension("new");

    if url.starts_with("file://") {
        // Mock/local file copy for testing
        let source = url.strip_prefix("file://").unwrap();
        std::fs::copy(source, &tmp_path)
            .map_err(|e| format!("Failed to copy mock binary: {}", e))?;
    } else {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(300))
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download returned {}", response.status()));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read download: {}", e))?;

        std::fs::write(&tmp_path, &bytes)
            .map_err(|e| format!("Failed to write downloaded binary: {}", e))?;
    }

    // Rename .new to final destination
    std::fs::rename(&tmp_path, dest_path)
        .map_err(|e| format!("Failed to rename downloaded binary: {}", e))?;

    Ok(())
}

// --- SHA256 verification ---

pub fn verify_sha256(file_path: &Path, expected_hex: &str) -> Result<(), String> {
    let bytes =
        std::fs::read(file_path).map_err(|e| format!("Failed to read file for hash: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();
    let actual_hex = hex_encode(&result);

    if actual_hex != expected_hex.to_lowercase() {
        return Err(format!(
            "SHA256 mismatch: expected {}, got {}",
            expected_hex, actual_hex
        ));
    }

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// --- Binary swap and rollback ---

pub fn swap_binary(name: &str, binary_dir: &Path) -> Result<(), String> {
    let filename = binary_filename(name);
    let active_path = binary_dir.join(&filename);
    let backup_path = binary_dir.join(format!("{}.backup", filename));
    let new_path = binary_dir.join(format!("{}.update", filename));

    if !new_path.exists() {
        return Err(format!("New binary not found at {:?}", new_path));
    }

    // Backup current binary if it exists
    if active_path.exists() {
        std::fs::rename(&active_path, &backup_path)
            .map_err(|e| format!("Failed to backup current binary: {}", e))?;
    }

    // Move new binary to active position
    std::fs::rename(&new_path, &active_path)
        .map_err(|e| format!("Failed to install new binary: {}", e))?;

    // Set executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&active_path, perms)
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;
    }

    Ok(())
}

pub fn rollback_binary(name: &str, binary_dir: &Path) -> Result<(), String> {
    let filename = binary_filename(name);
    let active_path = binary_dir.join(&filename);
    let backup_path = binary_dir.join(format!("{}.backup", filename));

    if !backup_path.exists() {
        return Err(format!("No backup found for {}", name));
    }

    // Remove failed new binary if it exists
    if active_path.exists() {
        let _ = std::fs::remove_file(&active_path);
    }

    // Restore backup
    std::fs::rename(&backup_path, &active_path)
        .map_err(|e| format!("Failed to restore backup: {}", e))?;

    Ok(())
}

pub fn cleanup_backup(name: &str, binary_dir: &Path) {
    let filename = binary_filename(name);
    let backup_path = binary_dir.join(format!("{}.backup", filename));
    let _ = std::fs::remove_file(&backup_path);
}

// --- Full update orchestration ---

pub async fn apply_binary_update(
    app_handle: AppHandle,
    update_info: &BinaryUpdateInfo,
    state: &AppState,
) -> Result<(), String> {
    let binary_dir = resolve_binary_dir(&app_handle);
    let filename = binary_filename(&update_info.name);
    let download_dest = binary_dir.join(format!("{}.update", filename));
    let data_dir = state.node.data_dir.lock().await.clone();

    // Step 1: Download
    emit_update_status(
        &app_handle,
        &state.update,
        UpdateStatus::Downloading {
            binary: update_info.name.clone(),
            progress: 0,
        },
    )
    .await;

    download_binary(&update_info.download_url, &download_dest).await?;

    emit_update_status(
        &app_handle,
        &state.update,
        UpdateStatus::Downloading {
            binary: update_info.name.clone(),
            progress: 100,
        },
    )
    .await;

    // Step 2: Verify SHA256
    verify_sha256(&download_dest, &update_info.sha256)?;

    // Step 3: Stop the relevant process
    emit_update_status(
        &app_handle,
        &state.update,
        UpdateStatus::Installing {
            binary: update_info.name.clone(),
        },
    )
    .await;

    let was_running = stop_binary_process(&app_handle, &update_info.name, state).await?;

    // Step 4: Swap binary
    if let Err(e) = swap_binary(&update_info.name, &binary_dir) {
        // Swap failed — try to restart old binary if it was running
        if was_running {
            let _ = start_binary_process(&app_handle, &update_info.name, state).await;
        }
        return Err(e);
    }

    // Step 5: Restart process if it was running, verify health
    if was_running {
        if let Err(e) = start_and_verify(&app_handle, &update_info.name, state).await {
            // Step 6: Rollback on failure
            log::warn!(
                "New {} binary failed health check, rolling back: {}",
                update_info.name,
                e
            );
            emit_update_status(
                &app_handle,
                &state.update,
                UpdateStatus::RollingBack {
                    binary: update_info.name.clone(),
                },
            )
            .await;

            let _ = stop_binary_process(&app_handle, &update_info.name, state).await;
            rollback_binary(&update_info.name, &binary_dir)?;
            let _ = start_binary_process(&app_handle, &update_info.name, state).await;

            return Err(format!(
                "Update failed for {}: new binary did not start correctly. Rolled back to previous version.",
                update_info.name
            ));
        }
    }

    // Step 7: Update version tracking
    let mut versions = BinaryVersions::load(&data_dir);
    versions.set(&update_info.name, update_info.new_version.clone());
    versions.save(&data_dir)?;

    // Cleanup backup
    cleanup_backup(&update_info.name, &binary_dir);

    log::info!(
        "Successfully updated {} to version {}",
        update_info.name,
        update_info.new_version
    );

    Ok(())
}

async fn stop_binary_process(
    app_handle: &AppHandle,
    name: &str,
    state: &AppState,
) -> Result<bool, String> {
    match name {
        "zebrad" => {
            let status = state.node.status.lock().await;
            if status.is_stopped_or_error() {
                return Ok(false);
            }
            drop(status);
            process::zebrad::stop_zebrad(app_handle, &state.node).await?;
            Ok(true)
        }
        "zaino" => {
            let status = state.wallet.status.lock().await;
            if status.is_stopped_or_error() {
                return Ok(false);
            }
            drop(status);
            let data_dir = state.node.data_dir.lock().await.clone();
            process::zaino::stop_zaino(app_handle, &state.wallet, &data_dir).await?;
            Ok(true)
        }
        "arti" => {
            let status = state.shield.status.lock().await;
            let is_active = matches!(*status, crate::state::ShieldStatus::Active);
            drop(status);
            if !is_active {
                return Ok(false);
            }
            tor::stop_arti(app_handle, &state.shield).await?;
            Ok(true)
        }
        _ => Err(format!("Unknown binary: {}", name)),
    }
}

async fn start_binary_process(
    app_handle: &AppHandle,
    name: &str,
    state: &AppState,
) -> Result<(), String> {
    match name {
        "zebrad" => {
            process::zebrad::start_zebrad(app_handle.clone(), &state.node).await
        }
        "zaino" => {
            let data_dir = state.node.data_dir.lock().await.clone();
            process::zaino::start_zaino(app_handle.clone(), &state.wallet, &data_dir).await
        }
        "arti" => {
            tor::start_arti(app_handle.clone(), &state.shield).await
        }
        _ => Err(format!("Unknown binary: {}", name)),
    }
}

async fn start_and_verify(
    app_handle: &AppHandle,
    name: &str,
    state: &AppState,
) -> Result<(), String> {
    start_binary_process(app_handle, name, state).await?;

    // Wait up to 10 seconds to verify process doesn't immediately crash
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    let still_running = match name {
        "zebrad" => {
            let status = state.node.status.lock().await;
            !status.is_stopped_or_error()
        }
        "zaino" => {
            let status = state.wallet.status.lock().await;
            !status.is_stopped_or_error()
        }
        "arti" => {
            let status = state.shield.status.lock().await;
            matches!(
                *status,
                crate::state::ShieldStatus::Active | crate::state::ShieldStatus::Bootstrapping { .. }
            )
        }
        _ => false,
    };

    if still_running {
        Ok(())
    } else {
        Err(format!("{} exited shortly after starting with new binary", name))
    }
}

// --- Periodic update checker ---

pub fn spawn_update_checker(app_handle: AppHandle) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Wait 30s for app to settle
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        loop {
            let state = app_handle.state::<AppState>();
            let data_dir = state.node.data_dir.lock().await.clone();

            match check_manifest(&data_dir, &app_handle).await {
                Ok(updates) if !updates.is_empty() => {
                    log::info!("Found {} binary update(s) available", updates.len());
                    let mut available = state.update.available_updates.lock().await;
                    *available = updates.clone();
                    drop(available);

                    emit_update_status(
                        &app_handle,
                        &state.update,
                        UpdateStatus::UpdateAvailable,
                    )
                    .await;

                    let _ = app_handle.emit("update_available", &updates);
                }
                Ok(_) => {
                    log::debug!("No binary updates available");
                }
                Err(e) => {
                    log::debug!("Update check failed (expected without server): {}", e);
                }
            }

            // Check every 24 hours
            tokio::time::sleep(std::time::Duration::from_secs(86400)).await;
        }
    })
}

// --- Event emission ---

pub async fn emit_update_status(
    app_handle: &AppHandle,
    update_state: &Arc<UpdateState>,
    status: UpdateStatus,
) {
    let mut current = update_state.status.lock().await;
    *current = status.clone();
    drop(current);
    let _ = app_handle.emit("update_status_changed", &status);
}
