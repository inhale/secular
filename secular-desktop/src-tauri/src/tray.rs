// src-tauri/src/tray.rs
// macOS Menu Bar tray - dynamic stats in menu items

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

fn resolve_tray_icon<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
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

/// Handle tray menu events
fn handle_tray_menu_event<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
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
    }
}

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TRAY] Starting tray setup");

    // Try to load icon, but don't fail if not found
    let icon = resolve_tray_icon(app.handle(), "tray-inactive")
        .or_else(|| app.default_window_icon().cloned())
        .ok_or("Tray icon load failed")?;

    // Initial menu - minimal, will be updated on first connect
    let connect_item = MenuItem::with_id(app, "tray-connect", "Connect", true, None::<&str>)?;
    let stats_time = MenuItem::with_id(app, "tray-stats-time", " Session: 00:00:00", false, None::<&str>)?;
    let stats_pkts = MenuItem::with_id(app, "tray-stats-pkts", " ↓ 0 pkts  ↑ 0 pkts", false, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let show_item = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    let menu = Menu::with_items(app, &[
        &connect_item, &stats_time, &stats_pkts, &sep1,
        &show_item, &sep2, &quit_item,
    ])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular - Disconnected")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(handle_tray_menu_event)
        .build(app)?;

    app.on_menu_event(handle_tray_menu_event);

    eprintln!("[TRAY] Tray built OK");
    Ok(())
}

/// Track previous state to only rebuild menu on transitions
static PREV_STATE: std::sync::Mutex<Option<(bool, bool)>> = std::sync::Mutex::new(None);

pub fn update_tray_state<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    payload: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> {
    let connected = payload.connected;
    let connecting = payload.connecting;
    let time_str = payload.session_time.as_deref().unwrap_or("00:00:00");
    let dl = payload.download_pkts.unwrap_or(0);
    let ul = payload.upload_pkts.unwrap_or(0);

    eprintln!("[TRAY] update: connected={}, server='{}', time='{}', dl={}, ul={}",
        connected, payload.server, time_str, dl, ul);

    if let Some(tray) = app.tray_by_id("main-tray") {
        // Tooltip only (no title change - user wants stats in menu, not next to icon)
        let tooltip = if connected {
            format!("Secular - Connected to {} | {} | ↓{} ↑{}", payload.server, time_str, dl, ul)
        } else if connecting {
            format!("Secular - Connecting to {}...", payload.server)
        } else {
            String::from("Secular - Disconnected")
        };

        // No icon change - always use same icon per user requirement
        let _ = tray.set_tooltip(Some(&tooltip));

        // Only rebuild menu on state transitions (not every timer tick)
        // to avoid closing the menu while user is looking at it
        let mut prev = PREV_STATE.lock().unwrap();
        let current = (connected, connecting);
        if prev.as_ref() != Some(&current) {
            *prev = Some(current);
            // Rebuild menu with current stats
            let connect_label = if connected {
                format!("Disconnect from {}", payload.server)
            } else if connecting {
                String::from("Connecting...")
            } else {
                format!("Connect {}", payload.server)
            };
            let connect_item = MenuItem::with_id(app, "tray-connect", &connect_label, connected || !connecting, None::<&str>)?;
            let stats_time = MenuItem::with_id(app, "tray-stats-time", &format!(" Session: {}", time_str), false, None::<&str>)?;
            let stats_pkts = MenuItem::with_id(app, "tray-stats-pkts", &format!(" ↓ {} pkts  ↑ {} pkts", dl, ul), false, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let show_item = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

            let menu = Menu::with_items(app, &[
                &connect_item, &stats_time, &stats_pkts, &sep1, &show_item, &sep2, &quit_item,
            ])?;
            let _ = tray.set_menu(Some(menu));
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn setup_tray(_: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }

#[cfg(not(target_os = "macos"))]
pub fn update_tray_state<R: tauri::Runtime>(
    _: &tauri::AppHandle<R>,
    _: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }