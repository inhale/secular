// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
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
    let names: Vec<String> = vec![
        format!("icons/{}.png", name),
        format!("icons/{}@2x.png", name),
    ];
    for filename in &names {
        if let Ok(path) = app.path().resolve(filename, tauri::path::BaseDirectory::Resource) {
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
        .or_else(|| app.default_window_icon().cloned())
        .ok_or("Tray icon load failed")?;

    let connect_item = MenuItem::with_id(app, "tray-connect", "Connect", true, None::<&str>)?;
    let show_item = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;
    let menu = Menu::with_items(app, &[&connect_item, &show_item, &sep, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular - Disconnected")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "tray-connect" => {
                let _ = app.emit("tray-connect", ());
            }
            "tray-show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;

    eprintln!("[TRAY] Tray built OK");
    Ok(())
}

pub fn update_tray_state(
    app: &tauri::AppHandle,
    payload: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> {
    let connected = payload.connected;
    let connecting = payload.connecting;
    let server = &payload.server;
    let time_str = payload.session_time.as_deref().unwrap_or("00:00:00");
    let dl = payload.download_pkts.unwrap_or(0);
    let ul = payload.upload_pkts.unwrap_or(0);

    // Show stats as title text next to tray icon in menu bar
    let title = if connected {
        format!(" {} | {} | {}:{}", server, time_str, dl, ul)
    } else if connecting {
        format!(" {} | connecting...", server)
    } else {
        String::new()
    };

    let tooltip = if connected {
        format!("Secular - Connected to {} | {} | {} down {} up", server, time_str, dl, ul)
    } else if connecting {
        format!("Secular - Connecting to {}...", server)
    } else {
        String::from("Secular - Disconnected")
    };

    eprintln!("[TRAY] connected={}, title={}", connected, title);

    if let Some(tray) = app.tray_by_id("main-tray") {
        tray.set_title(Some(&title))?;
        tray.set_tooltip(Some(&tooltip))?;
        eprintln!("[TRAY] updated OK");
    }

    Ok(())
}

