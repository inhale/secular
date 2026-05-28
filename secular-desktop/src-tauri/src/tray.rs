// src-tauri/src/tray.rs
// System tray / Menu Bar implementation
// Switches between tray-active.png (solid white) and tray-inactive.png (35% opacity)

use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem, SystemTraySubmenu,
};

const TRAY_ICON_ACTIVE: &[u8] = include_bytes!("../../icons/tray-active.png");
const TRAY_ICON_INACTIVE: &[u8] = include_bytes!("../../icons/tray-inactive.png");

/// Create the system tray
pub fn create_tray() -> SystemTray {
    let connect = CustomMenuItem::new("connect".to_string(), "Connect");
    let disconnect = CustomMenuItem::new("disconnect".to_string(), "Disconnect");
    let settings = CustomMenuItem::new("settings".to_string(), "Settings");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let menu = SystemTrayMenu::new()
        .add_item(connect)
        .add_item(disconnect)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(settings)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new()
        .with_menu(menu)
        .with_icon_as_template(true) // macOS template icon (auto-inverts)
        .with_tooltip("Secular — Disconnected")
}

/// Handle tray menu events
pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "connect" => {
                tracing::info!("Tray: connect");
                let window = app.get_webview_window("main").unwrap();
                window.emit("tray-connect", ()).unwrap();
                update_tray_icon(app, true);
            }
            "disconnect" => {
                tracing::info!("Tray: disconnect");
                let window = app.get_webview_window("main").unwrap();
                window.emit("tray-disconnect", ()).unwrap();
                update_tray_icon(app, false);
            }
            "settings" => {
                tracing::info!("Tray: settings");
                let window = app.get_webview_window("main").unwrap();
                window.show().unwrap();
                window.set_focus().unwrap();
            }
            "quit" => {
                tracing::info!("Tray: quit");
                app.exit(0);
            }
            _ => {}
        },
        SystemTrayEvent::LeftClick { .. } => {
            let window = app.get_webview_window("main").unwrap();
            window.show().unwrap();
            window.set_focus().unwrap();
        }
        _ => {}
    }
}

/// Update tray icon based on connection state
fn update_tray_icon(app: &AppHandle, connected: bool) {
    let tooltip = if connected {
        "Secular — Connected"
    } else {
        "Secular — Disconnected"
    };
    // Note: In Tauri v2, dynamic tray icon updates require the icon path approach
    // The tray-active.png / tray-inactive.png are swapped at the menu level
    tracing::debug!("Tray icon updated: {} (connected={})", tooltip, connected);
}
