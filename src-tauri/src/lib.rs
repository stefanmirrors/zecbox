mod commands;
mod config;
mod health;
mod network;
pub mod platform;
mod power;
mod process;
pub mod state;
mod storage;
mod tor;
mod updates;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
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

            // Restore Shield Mode if it was enabled before shutdown/crash
            // Only restore if onboarding is complete (not during first run)
            // Runs async so it doesn't block the window from opening
            if app_config.shield_mode && app_config.first_run_complete {
                let managed_state = app.state::<AppState>();
                let shield_arc = managed_state.shield.clone();
                let restore_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    log::info!("Restoring Shield Mode from saved config");
                    if let Err(e) = tor::start_arti(restore_handle.clone(), &shield_arc).await {
                        log::error!("Failed to restore Shield Mode Arti: {}", e);
                    } else if let Err(e) = tor::firewall::enable_firewall() {
                        log::error!("Failed to restore Shield Mode firewall: {}", e);
                    } else {
                        log::info!("Shield Mode restored successfully");
                    }
                });
            }

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

            // Restore network serve if previously enabled
            if app_config.serve_network && !app_config.shield_mode {
                let app_handle = app.handle().clone();
                let net_arc = app.state::<AppState>().network.clone();
                let ddd = app.state::<AppState>().default_data_dir.clone();
                tokio::spawn(async move {
                    // Wait for node to come up before enabling network serve
                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(5))
                        .build()
                        .ok();
                    for _ in 0..60 {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        if let Some(ref c) = client {
                            if let Ok(resp) = c.post("http://127.0.0.1:8232")
                                .json(&serde_json::json!({"jsonrpc":"2.0","method":"getinfo","params":[],"id":1}))
                                .send().await
                            {
                                if resp.status().is_success() {
                                    log::info!("Node is up, restoring network serve");
                                    let (public_ip, upnp_active, cgnat) = network::enable_upnp(8233).await.unwrap_or_default();
                                    let public_ip_opt = if public_ip.is_empty() { None } else { Some(public_ip.clone()) };
                                    let reachable = if !public_ip.is_empty() {
                                        network::check_reachability(&public_ip, 8233).await
                                    } else { None };
                                    let local_ip = network::get_local_ip();
                                    let active = state::NetworkServeStatus::Active {
                                        public_ip: public_ip_opt,
                                        reachable,
                                        inbound_peers: None,
                                        outbound_peers: None,
                                        upnp_active,
                                        local_ip,
                                        cgnat_detected: cgnat,
                                    };
                                    {
                                        let mut status = net_arc.status.lock().await;
                                        *status = active.clone();
                                    }
                                    let info = crate::commands::network::NetworkServeStatusInfo::from(&active);
                                    let _ = app_handle.emit("network_serve_status_changed", &info);
                                    let monitor = network::spawn_network_monitor(
                                        app_handle, net_arc.clone(), upnp_active, ddd,
                                    );
                                    *net_arc.monitor_task.lock().await = Some(monitor);
                                    return;
                                }
                            }
                        }
                    }
                    log::warn!("Node never came up, not restoring network serve");
                    // Clear the config since we couldn't restore
                    if let Ok(mut config) = AppConfig::load(&ddd) {
                        config.serve_network = false;
                        let _ = config.save(&ddd);
                    }
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
                        // Graceful shutdown: stop all processes
                        let state = app.state::<AppState>();
                        let node = state.node.clone();
                        let storage = state.storage.clone();
                        let shield = state.shield.clone();
                        let wallet = state.wallet.clone();
                        let update = state.update.clone();
                        let network_state = state.network.clone();
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
                            // Abort network serve monitor and clean up UPnP
                            if let Some(task) = network_state.monitor_task.lock().await.take() {
                                task.abort();
                            }
                            let needs_upnp_cleanup = {
                                let status = network_state.status.lock().await;
                                matches!(*status, state::NetworkServeStatus::Active { upnp_active: true, .. })
                            };
                            if needs_upnp_cleanup {
                                network::disable_upnp(8233).await;
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
            commands::shield::get_onion_address,
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
            commands::shield::is_shield_supported,
            commands::network::get_network_serve_status,
            commands::network::enable_network_serve,
            commands::network::disable_network_serve,
            commands::network::recheck_reachability,
        ])
        .run(tauri::generate_context!())
        .expect("zecbox failed to launch. Please reinstall the application.");
}
