// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2
// Shows Secular icon in macOS menu bar / Windows system tray
// Desktop-only: green icon when connected, white/black when disconnected
// Menu: Connect/Disconnect, Show Window, Quit

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, MouseButton, MouseButtonState},
    Emitter, Manager,
};

/// Load a tray icon from the bundled icons directory.
/// macOS requires @2x variants for Retina — we try the @2x version first.
fn load_tray_icon(
    app: &tauri::App,
    name: &str,
) -> Result<tauri::image::Image, Box<dyn std::error::Error>> {
    // Try @2x first (Retina), then fall back to 1x
    let names = if cfg!(target_os = "macos") {
        [format!("{name}@2x.png"), format!("{name}.png")]
    } else {
        [format!("{name}.png"), format!("{name}@2x.png")]
    };

    for filename in &names {
        let path = app.path()
            .resolve(format!("icons/{filename}"), tauri::path::BaseDirectory::Resource)
            .ok();

        if let Some(p) = path {
            if p.exists() {
                if let Ok(img) = tauri::image::Image::from_path(&p) {
                    tracing::info!("Loaded tray icon: {}", p.display());
                    return Ok(img);
                }
            }
        }
    }

    Err(format!("Tray icon '{name}' not found in bundled resources").into())
}

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Build the tray menu items
    let connect_item = MenuItem::with_id(app, "connect", "Connect", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "show", "Show Secular", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    let menu = Menu::with_items(app, &[&connect_item, &show_item, &sep, &quit_item])?;

    // Load the inactive (white/black) tray icon for startup
    let icon = match load_tray_icon(app, "tray-inactive") {
        Ok(img) => img,
        Err(e) => {
            // Fall back to the app's default window icon
            tracing::warn!("Could not load tray-inactive icon: {e} — falling back to default icon");
            match app.default_window_icon().cloned() {
                Some(i) => i,
                None => {
                    tracing::warn!("No default window icon either — skipping tray");
                    return Ok(());
                }
            }
        }
    };

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
        let icon_name = if connected { "tray-active" } else { "tray-inactive" };

        // Resolve the icon path from bundled resources
        let names = if cfg!(target_os = "macos") {
            [format!("{icon_name}@2x.png"), format!("{icon_name}.png")]
        } else {
            [format!("{icon_name}.png"), format!("{icon_name}@2x.png")]
        };

        for filename in &names {
            let path = app.path()
                .resolve(format!("icons/{filename}"), tauri::path::BaseDirectory::Resource)
                .ok();

            if let Some(p) = path {
                if p.exists() {
                    if let Ok(img) = tauri::image::Image::from_path(&p) {
                        let _ = tray.set_icon(Some(img));
                        tracing::debug!("Tray icon swapped to: {}", p.display());
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
