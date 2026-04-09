//! Platform-specific utilities: target triple detection and sidecar binary resolution.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Return the Rust target triple for the current compile target.
pub fn target_triple() -> &'static str {
    env!("TARGET")
}

/// Append `.exe` to a binary name on Windows, return as-is on other platforms.
fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

/// Resolve the path to a sidecar binary by name (e.g. "zebrad", "zaino", "arti").
///
/// Search order:
/// 1. Dev mode: `src-tauri/binaries/{name}-{target_triple}[.exe]`
/// 2. Production: alongside the main executable (with and without triple suffix, with and without .exe)
/// 3. Fallback: Tauri resource directory
pub fn resolve_sidecar_path(app_handle: &AppHandle, name: &str) -> PathBuf {
    let triple = target_triple();
    let name_with_triple = format!("{}-{}", name, triple);

    // Build candidate names (with .exe on Windows)
    let candidates: Vec<String> = if cfg!(windows) {
        vec![
            exe_name(&name_with_triple),
            name_with_triple.clone(),
            exe_name(name),
            name.to_string(),
        ]
    } else {
        vec![name_with_triple.clone(), name.to_string()]
    };

    // Check updates directory first (user-writable, takes priority over bundled)
    if let Ok(data_dir) = app_handle.path().app_data_dir() {
        let updates_dir = data_dir.join("updates");
        for candidate in &candidates {
            let path = updates_dir.join(candidate);
            if path.exists() {
                return path;
            }
        }
    }

    // In dev mode, look in src-tauri/binaries/
    if cfg!(debug_assertions) {
        let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");
        for candidate in &candidates {
            let path = bin_dir.join(candidate);
            if path.exists() {
                return path;
            }
        }
    }

    // Production: Tauri bundles externalBin alongside the main executable
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if path.exists() {
                return path;
            }
        }
    }

    // Fallback: resource dir
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        for candidate in &candidates {
            let path = resource_dir.join(candidate);
            if path.exists() {
                return path;
            }
        }
    }

    // Default: return the platform-appropriate name in the exe dir
    exe_dir.unwrap_or_default().join(exe_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_triple_is_not_empty() {
        let triple = target_triple();
        assert!(!triple.is_empty());
        // Should contain OS identifier
        assert!(
            triple.contains("darwin") || triple.contains("linux") || triple.contains("windows"),
            "unexpected triple: {}",
            triple
        );
    }
}
