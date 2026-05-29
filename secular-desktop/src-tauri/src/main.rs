// src-tauri/src/main.rs
// Secular Desktop — Tauri v2 main entry point with system tray

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Fix for dispatch2 bitflags recursion limit on macOS
#![recursion_limit = "256"]

mod commands;
mod tray;

fn main() {
    let mut app = tauri::Builder::default()
        .setup(|app| {
            tray::setup_tray(app)?;
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::get_state,
            commands::get_config,
            commands::set_config,
        ])
        .build(tauri::generate_context!())
        .expect("error building Secular app");

    app.run(|_app_handle, _event| {
        // Event loop placeholder
    });
}
