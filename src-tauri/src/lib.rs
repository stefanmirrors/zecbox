mod commands;
mod config;
mod health;
mod power;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Resolve default data directory (always fixed at app data dir)
            let default_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Could not determine application data directory: {}", e))?;
            std::fs::create_dir_all(&default_data_dir)
                .map_err(|e| format!("Could not create application data directory at {:?}: {}", default_data_dir, e))?;

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

            // Check for orphaned processes from a prior crash
            let node = app_state.node.clone();
            tauri::async_runtime::block_on(async {
                if let Err(e) = process::zebrad::check_orphan(&node).await {
                    log::warn!("zebrad orphan check failed: {}", e);
                }
                let node_data_dir = node.data_dir.lock().await.clone();
                if let Err(e) = process::zaino::check_zaino_orphan(&node_data_dir).await {
                    log::warn!("Zaino orphan check failed: {}", e);
                }
                if let Err(e) = tor::check_arti_orphan(&node_data_dir).await {
                    log::warn!("Arti orphan check failed: {}", e);
                }
            });

            // Clean up orphaned update files from interrupted binary swaps
            updates::cleanup_orphaned_update_files(&app.handle());

            // Spawn storage monitor task
            let storage_arc = app_state.storage.clone();
            let node_arc = app_state.node.clone();
            tauri::async_runtime::block_on(async {
                let monitor_handle = storage::spawn_storage_monitor(
                    app.handle().clone(),
                    node_arc,
                    storage_arc.clone(),
                );
                let mut task = storage_arc.monitor_task.lock().await;
                *task = Some(monitor_handle);
            });

            app.manage(app_state);

            // Spawn power monitor (sleep/wake handling)
            {
                let managed_state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    let (thread_handle, wake_task) = power::spawn_power_monitor(app.handle().clone());
                    *managed_state.power_thread.lock().await = Some(thread_handle);
                    *managed_state.power_wake_task.lock().await = Some(wake_task);
                });
            }

            // Spawn periodic update checker
            {
                let managed_state = app.state::<AppState>();
                let update_arc = managed_state.update.clone();
                tauri::async_runtime::block_on(async {
                    let checker_handle = updates::spawn_update_checker(app.handle().clone());
                    let mut task = update_arc.check_task.lock().await;
                    *task = Some(checker_handle);
                });
            }

            // System tray
            let status_item_for_tray = MenuItem::with_id(app, "status", "Status: Stopped", false, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit zecbox", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show zecbox", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&status_item_for_tray, &show, &quit])?;

            // Store tray status item reference for dynamic updates
            {
                let managed_state = app.state::<AppState>();
                tauri::async_runtime::block_on(async {
                    let mut tray = managed_state.tray_status.lock().await;
                    *tray = Some(status_item_for_tray);
                });
            }

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&menu);
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }
            let _tray = tray_builder
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        // Graceful shutdown: stop Zaino, zebrad, Arti, storage monitor
                        let state = app.state::<AppState>();
                        let node = state.node.clone();
                        let storage = state.storage.clone();
                        let shield = state.shield.clone();
                        let wallet = state.wallet.clone();
                        let update = state.update.clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::block_on(async {
                            // Abort update checker
                            if let Some(task) = update.check_task.lock().await.take() {
                                task.abort();
                            }
                            // Abort storage monitor
                            if let Some(task) = storage.monitor_task.lock().await.take() {
                                task.abort();
                            }
                            // Stop power monitor
                            power::stop_power_monitor();
                            {
                                let wake_task = app_handle.state::<AppState>()
                                    .power_wake_task.lock().await.take();
                                if let Some(task) = wake_task {
                                    task.abort();
                                }
                            }
                            // Stop Zaino first (depends on zebrad)
                            let data_dir = node.data_dir.lock().await.clone();
                            let _ =
                                process::zaino::stop_zaino(&app_handle, &wallet, &data_dir)
                                    .await;
                            // Stop zebrad
                            let _ =
                                process::zebrad::stop_zebrad(&app_handle, &node)
                                    .await;
                            // Stop Arti if running
                            let _ = tor::stop_arti(&app_handle, &shield).await;
                            // Disable PF firewall rules if active
                            let _ = tor::firewall::disable_firewall();
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
            commands::node::rebuild_database,
            commands::node::get_node_stats,
            commands::storage::get_volumes,
            commands::storage::get_storage_info,
            commands::storage::set_data_dir,
            commands::onboarding::get_app_config,
            commands::onboarding::complete_onboarding,
            commands::onboarding::reset_onboarding,
            commands::logs::get_logs,
            commands::shield::get_shield_status,
            commands::shield::enable_shield_mode,
            commands::shield::disable_shield_mode,
            commands::wallet::get_wallet_status,
            commands::wallet::enable_wallet_server,
            commands::wallet::disable_wallet_server,
            commands::wallet::get_wallet_qr,
            commands::updates::get_versions,
            commands::updates::get_update_status,
            commands::updates::check_for_updates,
            commands::updates::apply_update,
            commands::updates::apply_all_updates,
            commands::updates::dismiss_updates,
            commands::updates::check_app_update,
            commands::settings::get_auto_start_enabled,
            commands::settings::set_auto_start,
            commands::shield::install_firewall_helper,
            commands::shield::is_firewall_helper_installed,
        ])
        .run(tauri::generate_context!())
        .expect("zecbox failed to launch. Please reinstall the application.");
}
