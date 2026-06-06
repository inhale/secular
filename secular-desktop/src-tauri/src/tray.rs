// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2

use tauri::{
    menu::{Menu, MenuItem, MenuEvent, PredefinedMenuItem},
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
    let names = [
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

/// Build the tray menu — static items only.
/// macOS caches tray menus at system level, so dynamic stats go in set_title()/set_tooltip().
fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    payload: &TrayStatePayload,
) -> Result<Menu<R>, Box<dyn std::error::Error>> {
    // Connect / Disconnect toggle
    let connect_label = if payload.connected {
        "Disconnect"
    } else if payload.connecting {
        "Cancel"
    } else {
        "Connect"
    };
    let connect_item = MenuItem::with_id(app, "tray-connect", connect_label, true, None::<&str>)?;

    let show_item = MenuItem::with_id(app, "tray-show", "Show Secular", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    Menu::with_items(app, &[&connect_item, &show_item, &sep, &quit_item])
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Handle tray menu events — used both as builder callback and global handler
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

    let icon = resolve_tray_icon(app.handle(), "tray-inactive")
        .or_else(|| app.default_window_icon().cloned())
        .ok_or("Tray icon load failed")?;

    let empty_payload = TrayStatePayload {
        connected: false,
        connecting: false,
        server: String::new(),
        session_time: None,
        download_pkts: None,
        upload_pkts: None,
    };
    let menu = build_tray_menu(app.handle(), &empty_payload)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular - Disconnected")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .on_menu_event(handle_tray_menu_event)
        .build(app)?;

    // Also register a global menu event handler so that when set_menu()
    // replaces the menu, clicks on the new menu items are still handled.
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

    // File logging for debugging
    {
        use std::io::Write;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let line = format!("[{}] update_tray_state: connected={}, server='{}', time='{}', dl={}, ul={}\n",
            ts, connected, payload.server, time_str, dl, ul);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/secular-tray.log") {
            let _ = f.write_all(line.as_bytes());
        }
    }

    eprintln!("[TRAY] update: connected={}, server='{}', time='{}', dl={}, ul={}",
        connected, payload.server, time_str, dl, ul);

    if let Some(tray) = app.tray_by_id("main-tray") {
        // Show live stats as title text next to icon in menu bar
        let title = if connected {
            format!(" {} | {} | ↓{} ↑{}", time_str, payload.server, dl, ul)
        } else if connecting {
            format!(" {} — connecting...", payload.server)
        } else {
            String::new()
        };

        let tooltip = if connected {
            format!("Secular - Connected to {} | {} | ↓{} ↑{}", payload.server, time_str, dl, ul)
        } else if connecting {
            format!("Secular - Connecting to {}...", payload.server)
        } else {
            String::from("Secular - Disconnected")
        };

        // Update icon (active/inactive)
        let icon_name = if connected { "tray-active" } else { "tray-inactive" };
        if let Some(icon) = resolve_tray_icon(app, icon_name).or_else(|| app.default_window_icon().cloned()) {
            let _ = tray.set_icon(Some(icon));
        }

        let _ = tray.set_title(Some(&title));
        let _ = tray.set_tooltip(Some(&tooltip));

        eprintln!("[TRAY] updated title='{}'", title);
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

