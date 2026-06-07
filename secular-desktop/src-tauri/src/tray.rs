// src-tauri/src/tray.rs
// macOS Menu Bar tray - dynamic stats in menu items.
//
// Strategy: create menu items once, keep references, update text in-place
// via set_text(). Never call set_menu() after setup — that would close
// the open menu. Stats update dynamically without closing the menu.

use std::sync::Mutex;
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

/// Held across updates so we can call set_text() in-place.
struct MenuItems {
    connect_item: MenuItem<tauri::Wry>,
    stats_time:   MenuItem<tauri::Wry>,
    stats_pkts:   MenuItem<tauri::Wry>,
    show_item:    MenuItem<tauri::Wry>,
}

static ITEMS: Mutex<Option<MenuItems>> = Mutex::new(None);

fn resolve_tray_icon<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    name: &str,
) -> Option<tauri::image::Image<'static>> {
    for f in &[format!("icons/{}.png", name), format!("icons/{}@2x.png", name)] {
        if let Ok(p) = app.path().resolve(f, tauri::path::BaseDirectory::Resource) {
            if p.exists() {
                return tauri::image::Image::from_path(&p).ok();
            }
        }
    }
    None
}

fn handle_tray_menu_event<R: tauri::Runtime>(app: &tauri::AppHandle<R>, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "tray-connect" => { let _ = app.emit("tray-connect", ()); }
        "tray-show" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        _ => {}
    }
}

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TRAY] Starting tray setup");

    let icon = resolve_tray_icon(app.handle(), "tray-inactive")
        .or_else(|| app.default_window_icon().cloned())
        .ok_or("Tray icon load failed")?;

    let ah = app.handle().clone();

    // Build initial items
    let connect_item = MenuItem::with_id(app, "tray-connect", "Connect", true, None::<&str>)?;
    let stats_time   = MenuItem::with_id(app, "tray-stats-time", " Session: 00:00:00", false, None::<&str>)?;
    let stats_pkts   = MenuItem::with_id(app, "tray-stats-pkts", " ↓ 0 pkts  ↑ 0 pkts", false, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let show_item    = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item    = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    let menu = Menu::with_items(app, &[
        &connect_item, &stats_time, &stats_pkts, &sep1,
        &show_item, &sep2, &quit_item,
    ])?;

    // Keep item references for in-place updates
    *ITEMS.lock().unwrap() = Some(MenuItems { connect_item, stats_time, stats_pkts, show_item });

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

pub fn update_tray_state<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    payload: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> {
    let connected = payload.connected;
    let connecting = payload.connecting;
    let time_str = payload.session_time.as_deref().unwrap_or("00:00:00");
    let dl = payload.download_pkts.unwrap_or(0);
    let ul = payload.upload_pkts.unwrap_or(0);

    eprintln!("[TRAY] update: c={} s='{}' t='{}' dl={} ul={}",
        connected, payload.server, time_str, dl, ul);

    // Update tooltip
    if let Some(tray) = app.tray_by_id("main-tray") {
        let tooltip = if connected {
            format!("Connected to {} | {} | ↓{} ↑{}", payload.server, time_str, dl, ul)
        } else if connecting {
            format!("Connecting to {}...", payload.server)
        } else {
            String::from("Disconnected")
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }

    // Update menu items in-place — no set_menu(), menu stays open
    let guard = ITEMS.lock().unwrap();
    if let Some(items) = guard.as_ref() {
        // Connect/disconnect label
        let connect_label = if connected {
            format!("Disconnect from {}", payload.server)
        } else if connecting {
            String::from("Connecting…")
        } else {
            format!("Connect {}", payload.server)
        };
        let _ = items.connect_item.set_text(&connect_label);
        let _ = items.stats_time.set_text(&format!(" Session: {}", time_str));
        let _ = items.stats_pkts.set_text(&format!(" ↓ {} pkts  ↑ {} pkts", dl, ul));
        let enabled = connected || !connecting;
        let _ = items.connect_item.set_enabled(enabled);
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn setup_tray(_: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }

#[cfg(not(target_os = "macos"))]
pub fn update_tray_state<R: tauri::Runtime>(
    _: &tauri::AppHandle<R>, _: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
