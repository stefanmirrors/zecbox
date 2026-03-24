//! Commands for wallet server (Zaino) lifecycle.

use tauri::{AppHandle, State};

use crate::config::app_config::AppConfig;
use crate::process::zaino;
use crate::state::{AppState, NodeStatus, WalletStatus};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletStatusInfo {
    pub enabled: bool,
    pub status: String,
    pub endpoint: Option<String>,
    pub message: Option<String>,
}

impl From<&WalletStatus> for WalletStatusInfo {
    fn from(status: &WalletStatus) -> Self {
        match status {
            WalletStatus::Stopped => WalletStatusInfo {
                enabled: false,
                status: "stopped".into(),
                endpoint: None,
                message: None,
            },
            WalletStatus::Starting => WalletStatusInfo {
                enabled: false,
                status: "starting".into(),
                endpoint: None,
                message: None,
            },
            WalletStatus::Running { endpoint } => WalletStatusInfo {
                enabled: true,
                status: "running".into(),
                endpoint: Some(endpoint.clone()),
                message: None,
            },
            WalletStatus::Stopping => WalletStatusInfo {
                enabled: false,
                status: "stopping".into(),
                endpoint: None,
                message: None,
            },
            WalletStatus::Error { message } => WalletStatusInfo {
                enabled: false,
                status: "error".into(),
                endpoint: None,
                message: Some(message.clone()),
            },
        }
    }
}

#[tauri::command]
pub async fn get_wallet_status(
    state: State<'_, AppState>,
) -> Result<WalletStatusInfo, String> {
    let status = state.wallet.status.lock().await;
    Ok(WalletStatusInfo::from(&*status))
}

#[tauri::command]
pub async fn enable_wallet_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Node must be running
    {
        let node_status = state.node.status.lock().await;
        if !matches!(*node_status, NodeStatus::Running { .. }) {
            return Err("Node must be running to enable wallet server".into());
        }
    }

    let data_dir = state.node.data_dir.lock().await.clone();

    // Start Zaino
    zaino::start_zaino(app_handle.clone(), &state.wallet, &data_dir).await?;

    // Persist wallet_server setting
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.wallet_server = true;
    config.save(&state.default_data_dir)?;

    log::info!("Wallet server enabled");
    Ok(())
}

#[tauri::command]
pub async fn disable_wallet_server(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.node.data_dir.lock().await.clone();

    // Stop Zaino
    zaino::stop_zaino(&app_handle, &state.wallet, &data_dir).await?;

    // Persist wallet_server setting
    let mut config = AppConfig::load(&state.default_data_dir)
        .unwrap_or_else(|_| AppConfig::default_for(&state.default_data_dir));
    config.wallet_server = false;
    config.save(&state.default_data_dir)?;

    log::info!("Wallet server disabled");
    Ok(())
}

#[tauri::command]
pub async fn get_wallet_qr(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let status = state.wallet.status.lock().await;
    let endpoint = match &*status {
        WalletStatus::Running { endpoint } => endpoint.clone(),
        _ => return Err("Wallet server is not running".into()),
    };

    // Generate QR code as SVG string
    let qr = qrcode::QrCode::new(endpoint.as_bytes())
        .map_err(|e| format!("Failed to generate QR code: {}", e))?;

    let svg = qr
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(200, 200)
        .dark_color(qrcode::render::svg::Color("#ffffff"))
        .light_color(qrcode::render::svg::Color("#1a1a2e"))
        .build();

    // Return as data URL
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        svg.as_bytes(),
    );
    Ok(format!("data:image/svg+xml;base64,{}", encoded))
}
