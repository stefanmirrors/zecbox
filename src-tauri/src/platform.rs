//! Platform-specific utilities: target triple detection and sidecar binary resolution.

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Return the Rust target triple for the current compile target.
pub fn target_triple() -> &'static str {
    env!("TARGET")
}

/// Resolve the path to a sidecar binary by name (e.g. "zebrad", "zaino", "arti").
///
/// Search order:
/// 1. Dev mode: `src-tauri/binaries/{name}-{target_triple}`
/// 2. Production: alongside the main executable (with and without triple suffix)
/// 3. Fallback: Tauri resource directory
pub fn resolve_sidecar_path(app_handle: &AppHandle, name: &str) -> PathBuf {
    let triple = target_triple();
    let name_with_triple = format!("{}-{}", name, triple);

    // In dev mode, look in src-tauri/binaries/
    if cfg!(debug_assertions) {
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&name_with_triple);
        if dev_path.exists() {
            return dev_path;
        }
    }

    // Production: Tauri bundles externalBin alongside the main executable
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        // Tauri may strip the target triple when bundling
        let prod_path = dir.join(name);
        if prod_path.exists() {
            return prod_path;
        }
        let prod_path = dir.join(&name_with_triple);
        if prod_path.exists() {
            return prod_path;
        }
    }

    // Fallback: resource dir
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let prod_path = resource_dir.join(name);
        if prod_path.exists() {
            return prod_path;
        }
    }

    exe_dir.unwrap_or_default().join(name)
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
