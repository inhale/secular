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

/// Build the tray menu with current stats as disabled items
fn build_tray_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    payload: &TrayStatePayload,
) -> Result<Menu<R>, Box<dyn std::error::Error>> {
    let time_str = payload.session_time.as_deref().unwrap_or("00:00:00");
    let dl = payload.download_pkts.unwrap_or(0);
    let ul = payload.upload_pkts.unwrap_or(0);

    // Stats lines (disabled — not clickable)
    let stats1 = if payload.connected {
        MenuItem::with_id(app, "tray-stats-1", &format!("  Server: {}", payload.server), false, None::<&str>)?
    } else if payload.connecting {
        MenuItem::with_id(app, "tray-stats-1", &format!("  {} — connecting...", payload.server), false, None::<&str>)?
    } else {
        MenuItem::with_id(app, "tray-stats-1", "  Disconnected", false, None::<&str>)?
    };

    let stats2 = if payload.connected {
        MenuItem::with_id(app, "tray-stats-2", &format!("  Session: {}", time_str), false, None::<&str>)?
    } else {
        MenuItem::with_id(app, "tray-stats-2", "  Session: --", false, None::<&str>)?
    };

    let stats3 = if payload.connected {
        MenuItem::with_id(app, "tray-stats-3", &format!("  ↓ {} pkts  ↑ {} pkts", dl, ul), false, None::<&str>)?
    } else {
        MenuItem::with_id(app, "tray-stats-3", "  ↓ --  ↑ --", false, None::<&str>)?
    };

    let sep1 = PredefinedMenuItem::separator(app)?;

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
    let sep2 = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit Secular"))?;

    Menu::with_items(app, &[
        &stats1, &stats2, &stats3, &sep1,
        &connect_item, &show_item, &sep2, &quit_item,
    ])
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
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

    eprintln!(
        "[TRAY] update: connected={}, server='{}', time='{}', dl={}, ul={}",
        connected, payload.server, time_str, dl, ul
    );

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

        // Rebuild menu with updated stats for next time user opens it
        if let Ok(menu) = build_tray_menu(app, &payload) {
            let _ = tray.set_menu(Some(menu));
        }

        eprintln!("[TRAY] updated title='{}'", title);
    }

    Ok(())
}
