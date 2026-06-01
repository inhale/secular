// src-tauri/src/main.rs
// Secular Desktop — Tauri v2 main entry point with system tray

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Fix for dispatch2 bitflags recursion limit on macOS
#![recursion_limit = "256"]

mod commands;
mod tray;

use tauri::Listener;

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            // Tray setup — non-fatal: if tray icon fails (e.g. missing icon file),
            // the app still works, just without the tray menu
            if let Err(e) = tray::setup_tray(app) {
                eprintln!("Warning: tray setup failed: {e}");
            }

            // Listen for connection state changes from frontend
            // to update the tray menu (Connect ↔ Disconnect)
            let handle = app.handle().clone();
            app.listen("tray-state-changed", move |event| {
                let payload = event.payload().to_string();
                let connected = payload.contains("connected");
                let _ = tray::update_tray_state(&handle, connected);
            });

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::get_state,
            commands::get_config,
            commands::set_config,
            commands::read_file,
        ])
        .build(tauri::generate_context!())
        .expect("error building Secular app");

    app.run(|_app_handle, _event| {
        // Event loop placeholder
    });
}
