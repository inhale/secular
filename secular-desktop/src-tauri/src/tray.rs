// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2
// Desktop-only: green icon when connected, white/black when disconnected
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

    // Load the inactive (white/black) tray icon for startup
    // Try @2x first for Retina, then fall back to 1x
    let icon = load_tray_icon_from_app(app)
        .or_else(|_| app.default_window_icon().cloned().ok_or("no default icon"))
        .map_err(|e| format!("Tray icon load failed: {e}"))?;

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

/// Try loading a tray icon PNG from bundled resources.
/// Tries @2x (Retina) then 1x, for both tray-inactive and the default icon.
fn load_tray_icon_from_app(app: &tauri::App) -> Result<tauri::image::Image<'static>, String> {
    let names = if cfg!(target_os = "macos") {
        vec!["icons/tray-inactive@2x.png", "icons/tray-inactive.png"]
    } else {
        vec!["icons/tray-inactive.png", "icons/tray-inactive@2x.png"]
    };

    for filename in &names {
        if let Ok(path) = app.path().resolve(filename, tauri::path::BaseDirectory::Resource) {
            if path.exists() {
                if let Ok(img) = tauri::image::Image::from_path(&path) {
                    tracing::info!("Loaded tray icon: {}", path.display());
                    return Ok(img);
                }
            }
        }
    }

    Err("tray-inactive icon not found in resources".into())
}

/// Update the tray icon, tooltip, and Connect menu item based on connection state.
/// Desktop-only: green icon ↔ connected, white/black icon ↔ disconnected.
pub fn update_tray_state(
    app: &tauri::AppHandle,
    connected: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tray) = app.tray_by_id("main-tray") {
        // Update tooltip
        let tooltip = if connected {
            "Secular — Connected ✦"
        } else {
            "Secular — Disconnected"
        };
        let _ = tray.set_tooltip(Some(tooltip));

        // Swap the tray icon
        let icon_name = if connected {
            "tray-active"
        } else {
            "tray-inactive"
        };

        // Resolve icon from bundled resources — try @2x (Retina) then 1x
        let names = if cfg!(target_os = "macos") {
            vec![
                format!("icons/{icon_name}@2x.png"),
                format!("icons/{icon_name}.png"),
            ]
        } else {
            vec![
                format!("icons/{icon_name}.png"),
                format!("icons/{icon_name}@2x.png"),
            ]
        };

        for filename in &names {
            if let Ok(path) = app.path().resolve(filename, tauri::path::BaseDirectory::Resource) {
                if path.exists() {
                    if let Ok(img) = tauri::image::Image::from_path(&path) {
                        let _ = tray.set_icon(Some(img));
                        tracing::debug!("Tray icon swapped to: {}", path.display());
                        break;
                    }
                }
            }
        }
    }

    // Update the Connect/Disconnect menu item text
    if let Some(menu_item) = app.menu().and_then(|m| m.get("connect")) {
        if let tauri::menu::MenuItemKind::MenuItem(item) = menu_item {
            let _ = item.set_text(if connected { "Disconnect" } else { "Connect" });
        }
    }

    Ok(())
}
