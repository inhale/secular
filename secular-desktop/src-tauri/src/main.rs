// src-tauri/src/main.rs
// Secular Desktop — Tauri v2 main entry point with system tray

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod tray;

fn main() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::get_state,
            commands::get_config,
            commands::set_config,
        ])
        .system_tray(tray::create_tray())
        .on_system_tray_event(tray::handle_tray_event)
        .run(tauri::generate_context!())
        .expect("error while running Secular");
}
