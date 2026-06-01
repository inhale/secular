// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2
// Shows Secular icon in macOS menu bar / Windows system tray
// Menu: Connect/Disconnect, Show Window, Quit

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, MouseButton, MouseButtonState},
    Manager, Runtime,
};

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Build the tray menu
    let connect_item = MenuItem::with_id(app, "connect", "Connect", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "show", "Show Secular", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    let menu = Menu::with_items(app, &[&connect_item, &show_item, &sep, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular — Disconnected")
        .icon_as_template(true) // macOS template icon (auto-inverts in light/dark mode)
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "connect" => {
                    // Emit event to frontend to toggle connection
                    let _ = app.emit("tray-connect", ());
                }
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                tauri::tray::TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

/// Update the tray menu item text based on connection state
pub fn update_tray_state<R: Runtime>(
    app: &tauri::AppHandle<R>,
    connected: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let label = if connected { "Disconnect" } else { "Connect" };
        if let Some(menu) = tray.menu() {
            if let Some(item) = menu.get("connect") {
                let _ = item.set_text(label);
            }
        }
        let tooltip = if connected {
            "Secular — Connected ✦"
        } else {
            "Secular — Disconnected"
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
    Ok(())
}
