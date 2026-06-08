// src-tauri/src/main.rs
// Secular Desktop — Tauri v2 main entry point with system tray

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "256"]

#[macro_use]
extern crate objc;

mod commands;
mod tray;

use tauri::{Emitter, Listener, Manager};

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            // System menu bar — full macOS menu structure
            {
                use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};

                let hide_item = MenuItem::with_id(app, "hide", "Hide Secular", true, Some("Cmd+H"))?;
                let sep = PredefinedMenuItem::separator(app)?;
                let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;
                let app_submenu = Submenu::with_items(app, "Secular", true, &[&hide_item, &sep, &quit_item])?;

                let close_item = MenuItem::with_id(app, "close", "Close", true, Some("Cmd+W"))?;
                let file_submenu = Submenu::with_items(app, "File", true, &[&close_item])?;

                // Use PredefinedMenuItem for Edit menu — macOS automatically wires
                // Copy/Paste/SelectAll/Cut/Undo/Redo to the focused webview
                let copy_item = PredefinedMenuItem::copy(app, Some("Copy"))?;
                let paste_item = PredefinedMenuItem::paste(app, Some("Paste"))?;
                let select_all_item = PredefinedMenuItem::select_all(app, Some("Select All"))?;
                let edit_submenu = Submenu::with_items(app, "Edit", true, &[&copy_item, &paste_item, &select_all_item])?;

                let minimize_item = MenuItem::with_id(app, "minimize", "Minimize", true, Some("Cmd+M"))?;
                let zoom_item = MenuItem::with_id(app, "zoom", "Zoom", true, None::<&str>)?;
                let window_submenu = Submenu::with_items(app, "Window", true, &[&minimize_item, &zoom_item])?;

                let about_item = MenuItem::with_id(app, "about", "About Secular", true, None::<&str>)?;
                let help_submenu = Submenu::with_items(app, "Help", true, &[&about_item])?;

                let full_menu = Menu::with_items(app, &[&app_submenu, &file_submenu, &edit_submenu, &window_submenu, &help_submenu])?;
                app.set_menu(full_menu)?;

                let app_handle = app.handle().clone();
                app_handle.on_menu_event(move |app, event| {
                    let _ = std::fs::write("/tmp/secular-menu.log", format!("[MAIN] menu event: {}\n", event.id().as_ref()));
                    match event.id().as_ref() {
                        "hide" | "close" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                        "minimize" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.minimize();
                            }
                        }
                        "tray-connect" => {
                            eprintln!("[MAIN] tray-connect menu event");
                            if let Some(window) = app.get_webview_window("main") {
                                eprintln!("[MAIN] found main window, evaluating JS");
                                let _ = window.eval("window.__secular_tray_connect && window.__secular_tray_connect()").map_err(|e| eprintln!("[MAIN] JS eval error: {}", e));
                            } else {
                                eprintln!("[MAIN] no main window found!");
                                let _ = app.emit("tray-connect", ());
                            }
                        }
                        "tray-show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "tray-nav" => {}
                        _ => {}
                    }
                });
            }

            if let Err(e) = tray::setup_tray(app) {
                eprintln!("Warning: tray setup failed: {e}");
            }

            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .manage(commands::AppState {
            connected: std::sync::Mutex::new(false),
            config: std::sync::Mutex::new(commands::ServerConfig::default()),
            tunnel_pid: std::sync::Mutex::new(0),
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect,
            commands::disconnect,
            commands::get_state,
            commands::get_config,
            commands::set_config,
            commands::read_file,
            commands::read_tunnel_log,
            commands::update_tray,
            commands::tray_action,
            commands::show_window,
            commands::hide_window,
            commands::quit_app,
            commands::debug_log,
        ])
        .build(tauri::generate_context!())
        .expect("error building Secular app");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = &event
        {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.hide();
                api.prevent_close();
            }
        }

        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = &event {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }

        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Opened { urls } = &event {
            eprintln!("[MAIN] Opened event with {} URL(s)", urls.len());
            for url in urls {
                let url_str = url.to_string();
                eprintln!("[MAIN] Received URL: {}", url_str);
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app_handle.emit("deeplink", url_str);
            }
        }
    });
}
