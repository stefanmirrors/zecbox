//! Application state shared across Tauri commands.

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AppState {
    // Populated in Phase 1+
}
