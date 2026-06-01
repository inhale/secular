// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2
// Shows Secular icon in macOS menu bar / Windows system tray
// Menu: Connect/Disconnect, Show Window, Quit

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, MouseButton, MouseButtonState},
    Emitter, Manager,
};

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Build the tray menu items
    let connect_item = MenuItem::with_id(app, "connect", "Connect", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "show", "Show Secular", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    let menu = Menu::with_items(app, &[&connect_item, &show_item, &sep, &quit_item])?;

    // Load the tray icon — use the app's default window icon (already bundled)
    let icon = app.default_window_icon().cloned().unwrap_or_else(|| {
        // Fallback: create a simple 32x32 green circle icon
        tauri::image::Image::new_rgba(32, 32, vec![0u8; 32 * 32 * 4].into())
    });

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular — Disconnected")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "connect" => {
                let _ = app.emit("tray-connect", ());
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
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
        })
        .build(app)?;

    Ok(())
}

/// Update the tray tooltip and Connect menu item based on connection state
pub fn update_tray_state(
    app: &tauri::AppHandle,
    connected: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Update tooltip
    if let Some(tray) = app.tray_by_id("main-tray") {
        let tooltip = if connected {
            "Secular — Connected ✦"
        } else {
            "Secular — Disconnected"
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }

    // Update the Connect/Disconnect menu item text directly
    if let Some(menu_item) = app.menu().and_then(|m| m.get("connect")) {
        if let tauri::menu::MenuItemKind::MenuItem(item) = menu_item {
            let _ = item.set_text(if connected { "Disconnect" } else { "Connect" });
        }
    }

    Ok(())
}
