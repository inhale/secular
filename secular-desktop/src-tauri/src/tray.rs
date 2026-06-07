// src-tauri/src/tray.rs
// System tray / Menu Bar — popup via WebviewWindowBuilder.
// Uses set_ignore_cursor_events + set_focusable to prevent focus steal.

use tauri::{
    tray::{TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
    webview::WebviewWindowBuilder,
};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TrayStatePayload {
    pub connected: bool,
    pub connecting: bool,
    pub server: String,
    pub session_time: Option<String>,
    pub download_pkts: Option<u64>,
    pub upload_pkts: Option<u64>,
}

const POPUP_LABEL: &str = "tray-menu";

fn configure_popup<R: tauri::Runtime>(window: &tauri::webview::WebviewWindow<R>) {
    let _ = window.set_ignore_cursor_events(true);
    // NOTE: do NOT call set_focusable(false) — on macOS a non-focusable
    // window with decorations(false) fails canBecomeKeyWindow and will
    // immediately hide/flash. The window must be focusable to stay visible.
    eprintln!("[TRAY] Popup configured");
}

#[cfg(target_os = "macos")]
fn position_popup<R: tauri::Runtime>(window: &tauri::webview::WebviewWindow<R>) {
    // Try current_monitor first, fall back to primary_monitor.
    // For an invisible window current_monitor() often returns None.
    let monitor = window.current_monitor().ok().flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let Some(monitor) = monitor {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let x = (size.width as f64 / scale) - 240.0 - 8.0;
        let y = 28.0;
        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
    }
}

fn create_popup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if app.get_webview_window(POPUP_LABEL).is_some() {
        return;
    }
    match WebviewWindowBuilder::new(
        app, POPUP_LABEL,
        tauri::WebviewUrl::App("index.html?tray-menu".into()),
    )
    .title("Secular").inner_size(240.0, 380.0).resizable(false)
    .decorations(false).always_on_top(true).skip_taskbar(true)
    .visible(false).build()
    {
        Ok(window) => {
            // Position immediately while window is still hidden
            #[cfg(target_os = "macos")]
            position_popup(&window);
            configure_popup(&window);
            eprintln!("[TRAY] Popup created & positioned");
        }
        Err(e) => eprintln!("[TRAY] Popup create failed: {e}"),
    }
}

fn toggle_popup<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window(POPUP_LABEL) {
        match window.is_visible() {
            Ok(true) => { let _ = window.hide(); eprintln!("[TRAY] hidden"); }
            _ => {
                #[cfg(target_os = "macos")] position_popup(&window);
                let _ = window.show();
                eprintln!("[TRAY] shown");
            }
        }
    }
}

fn resolve_tray_icon<R: tauri::Runtime>(app: &tauri::AppHandle<R>, name: &str) -> Option<tauri::image::Image<'static>> {
    for f in &[format!("icons/{}.png", name), format!("icons/{}@2x.png", name)] {
        if let Ok(p) = app.path().resolve(f, tauri::path::BaseDirectory::Resource) {
            if p.exists() { return tauri::image::Image::from_path(&p).ok(); }
        }
    }
    None
}

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let icon = resolve_tray_icon(app.handle(), "tray-template")
        .or_else(|| app.default_window_icon().cloned())
        .ok_or("Tray icon load failed")?;
    let ah = app.handle().clone();
    create_popup(&ah);
    let _tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Secular").icon(icon).icon_as_template(true)
        .on_tray_icon_event(move |_t, e| {
            if let TrayIconEvent::Click { .. } = e { toggle_popup(&ah); }
        })
        .build(app)?;
    Ok(())
}

pub fn update_tray_state<R: tauri::Runtime>(app: &tauri::AppHandle<R>, p: TrayStatePayload) -> Result<(), Box<dyn std::error::Error>> {
    let _ = app.emit("tray-state-update", &p);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn setup_tray(_: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
#[cfg(not(target_os = "macos"))]
pub fn update_tray_state<R: tauri::Runtime>(_: &tauri::AppHandle<R>, _: TrayStatePayload) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
