// src-tauri/src/tray.rs
// System tray / Menu Bar implementation for Tauri v2
// Switches between tray-active.png (solid white) and tray-inactive.png (35% opacity)

use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};
use tauri::{AppHandle, Manager};

pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let tray = TrayIconBuilder::with_id("main-tray", app.handle())
        .tooltip("Secular — Disconnected")
        .icon_as_template(true) // macOS template icon (auto-inverts)
        .on_tray_icon_event(|tray, event| {
            match event {
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
            }
        })
        .build(app)?;

    Ok(())
}
