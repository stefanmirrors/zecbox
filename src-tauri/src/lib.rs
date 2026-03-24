mod commands;
mod config;
mod health;
mod process;
pub mod state;
mod storage;
mod tor;
mod updates;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use config::app_config::AppConfig;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Resolve default data directory (always fixed at app data dir)
            let default_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");
            std::fs::create_dir_all(&default_data_dir)
                .expect("failed to create app data directory");

            log::info!("Default data directory: {:?}", default_data_dir);

            // Load app config to determine effective data directory
            let app_config = AppConfig::load(&default_data_dir)
                .unwrap_or_else(|e| {
                    log::warn!("Failed to load config, using defaults: {}", e);
                    AppConfig::default_for(&default_data_dir)
                });

            // Use configured data_dir if it exists, otherwise fall back to default
            let data_dir = if app_config.data_dir != default_data_dir
                && app_config.data_dir.exists()
            {
                log::info!("Using custom data directory: {:?}", app_config.data_dir);
                app_config.data_dir.clone()
            } else {
                default_data_dir.clone()
            };

            // Check if data directory mount is available
            let drive_connected = storage::is_mount_available(&data_dir);
            if !drive_connected {
                log::warn!("Data directory mount not available: {:?}", data_dir);
            }

            // Initialize application state
            let app_state = AppState::new(data_dir, default_data_dir);

            // Set initial drive_connected state
            tauri::async_runtime::block_on(async {
                let mut connected = app_state.storage.drive_connected.lock().await;
                *connected = drive_connected;
            });

            // Check for orphaned zebrad process from a prior crash
            let node = app_state.node.clone();
            tauri::async_runtime::block_on(async {
                if let Err(e) = process::zebrad::check_orphan(&node).await {
                    log::warn!("Orphan check failed: {}", e);
                }
            });

            // Spawn storage monitor task
            let storage_arc = app_state.storage.clone();
            let node_arc = app_state.node.clone();
            let monitor_handle = storage::spawn_storage_monitor(
                app.handle().clone(),
                node_arc,
                storage_arc.clone(),
            );
            tauri::async_runtime::block_on(async {
                let mut task = storage_arc.monitor_task.lock().await;
                *task = Some(monitor_handle);
            });

            app.manage(app_state);

            // System tray
            let status_item_for_tray = MenuItem::with_id(app, "status", "Status: Stopped", false, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit ZecBox", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show ZecBox", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status_item_for_tray, &show, &quit])?;

            // Store tray status item reference for dynamic updates
            {
                let managed_state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    let mut tray = managed_state.tray_status.lock().await;
                    *tray = Some(status_item_for_tray);
                });
            }

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        // Graceful shutdown: stop storage monitor and zebrad before exiting
                        let state = app.state::<AppState>();
                        let node = state.node.clone();
                        let storage = state.storage.clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::block_on(async {
                            // Abort storage monitor
                            if let Some(task) = storage.monitor_task.lock().await.take() {
                                task.abort();
                            }
                            // Stop zebrad
                            let _ =
                                process::zebrad::stop_zebrad(&app_handle, &node)
                                    .await;
                        });
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::node::start_node,
            commands::node::stop_node,
            commands::node::get_node_status,
            commands::storage::get_volumes,
            commands::storage::get_storage_info,
            commands::storage::set_data_dir,
            commands::onboarding::get_app_config,
            commands::onboarding::complete_onboarding,
            commands::logs::get_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ZecBox");
}
