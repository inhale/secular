// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, MouseButton, MouseButtonState},
    Emitter, Manager,
};

#[derive(serde::Deserialize, Clone, Debug)]
pub struct TrayStatePayload {
    pub connected: bool,
    pub connecting: bool,
    pub server: String,
    pub session_time: Option<String>,
    pub download_pkts: Option<u64>,
    pub upload_pkts: Option<u64>,
}

fn resolve_tray_icon(
    app: &tauri::AppHandle,
    name: &str,
) -> Option<tauri::image::Image<'static>> {
    let names: Vec<String> = if cfg!(target_os = "macos") {
        vec![
            format!("icons/{name}@2x.png"),
            format!("icons/{name}.png"),
        ]
    } else {
        vec![
            format!("icons/{name}.png"),
            format!("icons/{name}@2x.png"),
        ]
    };

    for filename in &names {
        if let Ok(path) = app
            .path()
            .resolve(filename, tauri::path::BaseDirectory::Resource)
        {
            if path.exists() {
                if let Ok(img) = tauri::image::Image::from_path(&path) {
                    return Some(img);
                }
            }
        }
    }
    None
}

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TRAY] Starting tray setup");

    let icon = resolve_tray_icon(app.handle(), "tray-inactive")
        .or_else(|| {
            eprintln!("[TRAY] tray-inactive not found, trying default_window_icon");
            app.default_window_icon().cloned()
        })
        .ok_or("Tray icon load failed")?;

    eprintln!("[TRAY] Icon loaded OK, size: {}x{}", icon.width(), icon.height());

    // Create initial menu
    let connect_item = MenuItem::with_id(app, "tray-connect", "Connect", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;
    let menu = Menu::with_items(app, &[&connect_item, &show_item, &sep, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular — Disconnected")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-connect" => {
                eprintln!("[TRAY] Connect clicked");
                let _ = app.emit("tray-connect", ());
            }
            "tray-show" => {
                eprintln!("[TRAY] Show clicked");
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;

    eprintln!("[TRAY] Tray built successfully with menu");
    Ok(())
}

/// Update the tray icon, tooltip, and Connect menu item based on connection state.
pub fn update_tray_state(
    app: &tauri::AppHandle,
    payload: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> {
    let connected = payload.connected;
    let connecting = payload.connecting;
    let server = &payload.server;
    let session_time = &payload.session_time;
    let download_pkts = payload.download_pkts;
    let upload_pkts = payload.upload_pkts;

    // Build tooltip
    let tooltip = if connected {
        let time_str = session_time.as_deref().unwrap_or("00:00:00");
        let dl = download_pkts.unwrap_or(0);
        let ul = upload_pkts.unwrap_or(0);
        format!(
            "Secular — Connected to {}\nSession: {}\n↓ {} pkts  ↑ {} pkts",
            server, time_str, dl, ul
        )
    } else if connecting {
        format!("Secular — Connecting to {}...", server)
    } else {
        format!("Secular — Disconnected ({})", server)
    };

    // Build menu label
    let menu_label = if connecting {
        format!("Connecting [{}]...", server)
    } else if connected {
        format!("Disconnect [{}]", server)
    } else {
        format!("Connect [{}]", server)
    };

    // Always use template mode — macOS renders black/white automatically
    let icon_name = "tray-inactive";
    let use_template = true;

    // Rebuild the entire tray menu with updated label
    let connect_item = MenuItem::with_id(app, "tray-connect", &menu_label, !connecting, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    // Build menu items array — include stats when connected
    let server_item = MenuItem::with_id(app, "tray-server", format!("Server: {}", server), false, None::<&str>)?;
    let time_item = if connected {
        let time_str = session_time.as_deref().unwrap_or("00:00:00");
        MenuItem::with_id(app, "tray-time", format!("Session: {}", time_str), false, None::<&str>)?
    } else {
        MenuItem::with_id(app, "tray-time", "Session: --", false, None::<&str>)?
    };
    let stats_item = if connected {
        let dl = download_pkts.unwrap_or(0);
        let ul = upload_pkts.unwrap_or(0);
        MenuItem::with_id(app, "tray-stats", format!("↓ {} pkts  ↑ {} pkts", dl, ul), false, None::<&str>)?
    } else {
        MenuItem::with_id(app, "tray-stats", "No connection", false, None::<&str>)?
    };

    let menu = if connected {
        Menu::with_items(app, &[&server_item, &time_item, &stats_item, &sep1, &connect_item, &show_item, &sep2, &quit_item])?
    } else {
        Menu::with_items(app, &[&server_item, &sep1, &connect_item, &show_item, &sep2, &quit_item])?
    };

    eprintln!("[TRAY] update_tray_state: connected={}, server={}", connected, server);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(&tooltip));
        tray.set_menu(Some(menu))?;

        if let Some(img) = resolve_tray_icon(app, icon_name) {
            let _ = tray.set_icon(Some(img));
            let _ = tray.set_icon_as_template(use_template);
        }
    }

    Ok(())
}
