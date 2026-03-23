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

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Resolve data directory
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");
            std::fs::create_dir_all(&data_dir)
                .expect("failed to create app data directory");

            log::info!("Data directory: {:?}", data_dir);

            // Initialize application state
            let app_state = AppState::new(data_dir);

            // Check for orphaned zebrad process from a prior crash
            let node = app_state.node.clone();
            tauri::async_runtime::block_on(async {
                if let Err(e) = process::zebrad::check_orphan(&node).await {
                    log::warn!("Orphan check failed: {}", e);
                }
            });

            app.manage(app_state);

            // System tray
            let quit = MenuItem::with_id(app, "quit", "Quit ZecBox", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show ZecBox", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        // Graceful shutdown: stop zebrad before exiting
                        let state = app.state::<AppState>();
                        let node = state.node.clone();
                        let app_handle = app.clone();
                        tauri::async_runtime::block_on(async {
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
        .invoke_handler(tauri::generate_handler![
            commands::node::start_node,
            commands::node::stop_node,
            commands::node::get_node_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ZecBox");
}
