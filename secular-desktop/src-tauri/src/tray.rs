// src-tauri/src/tray.rs
// macOS NSPopover tray with native NSView controls (objc2 edition).
//
// Solves: NSMenu snapshots content on open — stats can't update live.
// NSPopover + NSView: NSTextField.setStringValue updates immediately.

use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(serde::Deserialize, Clone, Debug)]
pub struct TrayStatePayload {
    pub connected: bool,
    pub connecting: bool,
    pub server: String,
    pub session_time: Option<String>,
    pub download_pkts: Option<u64>,
    pub upload_pkts: Option<u64>,
}

// ═══════════════════════════════════════════════════════════════
// macOS native implementation
// ═══════════════════════════════════════════════════════════════

#[cfg(target_os = "macos")]
mod mac {
    use super::TrayStatePayload;
    use objc2::declare::ClassDecl;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, Sel, NSObject};
    use objc2::{class, msg_send, sel};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{
        NSAutoreleasePool, NSRect, NSSize, NSString, NSURL,
    };
    use std::sync::Mutex;

    // ── Shared native UI references ──────────────────────────

    type Id = *mut AnyObject;

    struct TrayObj {
        popover: Id,
        status_label: Id,
        timer_label: Id,
        dl_label: Id,
        ul_label: Id,
        action_button: Id,
    }

    unsafe impl Send for TrayObj {}
    unsafe impl Sync for TrayObj {}

    static TRAY_OBJ: Mutex<Option<TrayObj>> = Mutex::new(None);
    static mut POPOVER_PTR: usize = 0;

    // Rust callback for button clicks
    type ActionFn = Box<dyn Fn(&str) + Send + Sync>;
    static ON_ACTION: Mutex<Option<ActionFn>> = Mutex::new(None);

    pub fn set_action_callback(f: impl Fn(&str) + Send + Sync + 'static) {
        *ON_ACTION.lock().unwrap() = Some(Box::new(f));
    }

    fn fire(action: &str) {
        let guard = ON_ACTION.lock().unwrap();
        if let Some(cb) = guard.as_ref() {
            cb(action);
        } else {
            eprintln!("[TRAY] action '{}' dropped — no callback", action);
        }
    }

    // ── ObjC delegate class ──────────────────────────────────

    pub fn setup_native_tray() -> Result<(), String> {
        unsafe {
            let _pool = NSAutoreleasePool::new();

            // Register delegate class
            let superclass = NSObject::class();
            let mut decl = ClassDecl::new("SecularTrayTarget", superclass)
                .ok_or("ClassDecl::new failed")?;

            // trayIconClick:
            extern "C" fn icon_click(
                _self: &objc2::runtime::AnyObject,
                _cmd: Sel,
                sender: *mut AnyObject,
            ) {
                unsafe {
                    let popover = POPOVER_PTR as Id;
                    if popover.is_null() { return; }
                    let is_shown: bool = msg_send![popover, isShown];
                    if is_shown {
                        let _: () = msg_send![popover, performClose: std::ptr::null::<AnyObject>()];
                    } else {
                        let rect = NSRect::new(
                            objc2_foundation::NSPoint::new(0.0, 0.0),
                            NSSize::new(1.0, 1.0),
                        );
                        let _: () = msg_send![
                            popover,
                            showRelativeToRect: rect
                            ofView: sender
                            preferredEdge: 1u64 // NSMaxYEdge
                        ];
                        let app = NSApplication::sharedApplication();
                        let _: () = msg_send![app, activateIgnoringOtherApps: true];
                        eprintln!("[TRAY] popover shown");
                    }
                }
            }
            decl.add_method(
                sel!(trayIconClick:),
                icon_click as extern "C" fn(&AnyObject, Sel, *mut AnyObject),
            );

            // trayAction:
            extern "C" fn action_click(
                _self: &AnyObject,
                _cmd: Sel,
                _sender: *mut AnyObject,
            ) {
                eprintln!("[TRAY] trayAction");
                fire("connect_toggle");
            }
            decl.add_method(
                sel!(trayAction:),
                action_click as extern "C" fn(&AnyObject, Sel, *mut AnyObject),
            );

            // trayShow:
            extern "C" fn show_click(
                _self: &AnyObject,
                _cmd: Sel,
                _sender: *mut AnyObject,
            ) {
                eprintln!("[TRAY] trayShow");
                fire("show");
            }
            decl.add_method(
                sel!(trayShow:),
                show_click as extern "C" fn(&AnyObject, Sel, *mut AnyObject),
            );

            // trayQuit:
            extern "C" fn quit_click(
                _self: &AnyObject,
                _cmd: Sel,
                _sender: *mut AnyObject,
            ) {
                eprintln!("[TRAY] trayQuit");
                fire("quit");
            }
            decl.add_method(
                sel!(trayQuit:),
                quit_click as extern "C" fn(&AnyObject, Sel, *mut AnyObject),
            );

            let cls = decl.register();
            let target: *mut AnyObject = msg_send![cls, new];

            // Build popover content
            let (content_view, st, tl, dl, ul, ab) = build_content_view(target);

            // Create NSPopover
            let popover: *mut AnyObject = msg_send![class!(NSPopover), new];
            let _: () = msg_send![popover, setBehavior: 1i64]; // NSPopoverBehaviorTransient
            let _: () = msg_send![popover, setAnimates: true];
            let _: () = msg_send![
                popover,
                setContentSize: NSSize::new(300.0, 175.0)
            ];

            let vc: *mut AnyObject = msg_send![class!(NSViewController), new];
            let _: () = msg_send![vc, setView: content_view];
            let _: () = msg_send![popover, setContentViewController: vc];

            POPOVER_PTR = popover as usize;

            // Store refs
            *TRAY_OBJ.lock().unwrap() = Some(TrayObj {
                popover,
                status_label: st,
                timer_label: tl,
                dl_label: dl,
                ul_label: ul,
                action_button: ab,
            });

            // Create status bar item
            let bar: *mut AnyObject = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: *mut AnyObject = msg_send![
                bar,
                statusItemWithLength: -1i64 // NSStatusItemVariableLength
            ];

            // Load icon or fall back to text
            let mut icon_loaded = false;
            for name in &[
                "tray-inactive.png", "tray_inactive.png", "icon.png",
                "icons/tray-inactive.png", "icons/icon.png",
                "../icons/tray-inactive.png", "../icons/icon.png",
                "src-tauri/icons/tray-inactive.png", "src-tauri/icons/icon.png",
            ] {
                let p = std::path::Path::new(name);
                if p.exists() {
                    let abs = std::fs::canonicalize(p).unwrap().display().to_string();
                    let url = NSURL::fileURLWithPath(&NSString::from_str(&abs));
                    let img: *mut AnyObject = msg_send![class!(NSImage), alloc];
                    let img: *mut AnyObject = msg_send![img, initWithContentsOfURL: &url];
                    if !img.is_null() {
                        let _: () = msg_send![img, setTemplate: true];
                        let btn: *mut AnyObject = msg_send![item, button];
                        let _: () = msg_send![btn, setImage: img];
                        icon_loaded = true;
                        eprintln!("[TRAY] icon: {}", abs);
                        break;
                    }
                }
            }
            if !icon_loaded {
                let btn: *mut AnyObject = msg_send![item, button];
                let _: () = msg_send![btn, setTitle: &NSString::from_str("S")];
                eprintln!("[TRAY] no icon, using 'S'");
            }

            // Wire button click → toggle popover
            let btn: *mut AnyObject = msg_send![item, button];
            let _: () = msg_send![btn, setTarget: target];
            let _: () = msg_send![btn, setAction: sel!(trayIconClick:)];

            eprintln!("[TRAY] native tray ready");
            Ok(())
        }
    }

    // ── NSView builder helpers ────────────────────────────────

    const POP_W: f64 = 300.0;
    const POP_H: f64 = 175.0;
    const PAD: f64 = 14.0;

    unsafe fn nsrect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
        NSRect::new(objc2_foundation::NSPoint::new(x, y), NSSize::new(w, h))
    }

    unsafe fn make_label(
        text: &str, x: f64, y: f64, w: f64, h: f64,
        size: f64, r: f64, g: f64, b: f64, a: f64,
    ) -> Id {
        let tf: Id = msg_send![class!(NSTextField), new];
        let _: () = msg_send![tf, setStringValue: &NSString::from_str(text)];
        let _: () = msg_send![tf, setBezeled: false];
        let _: () = msg_send![tf, setDrawsBackground: false];
        let _: () = msg_send![tf, setEditable: false];
        let _: () = msg_send![tf, setSelectable: false];
        let c: Id = msg_send![class!(NSColor), colorWithRed: r green: g blue: b alpha: a];
        let _: () = msg_send![tf, setTextColor: c];
        let f: Id = msg_send![class!(NSFont), systemFontOfSize: size];
        let _: () = msg_send![tf, setFont: f];
        let _: () = msg_send![tf, setFrame: nsrect(x, y, w, h)];
        tf
    }

    unsafe fn make_button(
        title: &str, x: f64, y: f64, w: f64, h: f64, target: Id, action: Sel,
    ) -> Id {
        let b: Id = msg_send![class!(NSButton), new];
        let _: () = msg_send![b, setTitle: &NSString::from_str(title)];
        let _: () = msg_send![b, setBezelStyle: 1i64];
        let _: () = msg_send![b, setTarget: target];
        let _: () = msg_send![b, setAction: action];
        let f: Id = msg_send![class!(NSFont), systemFontOfSize: 12.0];
        let _: () = msg_send![b, setFont: f];
        let _: () = msg_send![b, setFrame: nsrect(x, y, w, h)];
        b
    }

    unsafe fn make_separator(x: f64, y: f64, w: f64) -> Id {
        let s: Id = msg_send![class!(NSBox), new];
        let _: () = msg_send![s, setBoxType: 3i64];
        let _: () = msg_send![s, setFrame: nsrect(x, y, w, 1.0)];
        s
    }

    unsafe fn build_content_view(target: Id) -> (Id, Id, Id, Id, Id, Id) {
        let view: Id = msg_send![class!(NSView), new];
        let bg: Id = msg_send![class!(NSColor), colorWithRed: 0.11 green: 0.11 blue: 0.12 alpha: 1.0];
        let _: () = msg_send![view, setWantsLayer: true];
        let layer: Id = msg_send![view, layer];
        let cg: Id = msg_send![bg, CGColor];
        let _: () = msg_send![layer, setBackgroundColor: cg];
        let _: () = msg_send![view, setFrame: nsrect(0.0, 0.0, POP_W, POP_H)];

        let cw = POP_W - PAD * 2.0;
        let mut y = POP_H - PAD;

        // Status label
        y -= 18.0;
        let st = make_label("Disconnected", PAD, y, cw, 18.0, 13.0, 0.46, 0.46, 0.46, 1.0);
        let _: () = msg_send![view, addSubview: st];

        // Timer
        y -= 26.0;
        let tl = make_label("00:00:00", PAD, y, cw, 26.0, 19.0, 1.0, 1.0, 1.0, 1.0);
        let _: () = msg_send![view, addSubview: tl];

        // Stats row
        y -= 18.0;
        let dl = make_label("↓0", PAD, y, 70.0, 18.0, 11.0, 0.533, 0.808, 0.98, 1.0);
        let _: () = msg_send![view, addSubview: dl];
        let ul = make_label("↑0", PAD + 75.0, y, 70.0, 18.0, 11.0, 0.443, 0.776, 0.443, 1.0);
        let _: () = msg_send![view, addSubview: ul];

        // Separator
        y -= 14.0;
        let sp = make_separator(PAD, y, cw);
        let _: () = msg_send![view, addSubview: sp];

        // Action button
        y -= 32.0;
        let ab = make_button("Connect", PAD, y, cw, 26.0, target, sel!(trayAction:));
        let _: () = msg_send![view, addSubview: ab];

        // Show + Quit row
        y -= 28.0;
        let (hw, gap) = ((cw - 4.0) / 2.0, 4.0);
        let sb = make_button("Show Secular", PAD, y, hw, 22.0, target, sel!(trayShow:));
        let _: () = msg_send![view, addSubview: sb];
        let qb = make_button("Quit", PAD + hw + gap, y, hw, 22.0, target, sel!(trayQuit:));
        let _: () = msg_send![view, addSubview: qb];

        (view, st, tl, dl, ul, ab)
    }

    // ── Update UI labels (works while popover is open) ────────

    pub fn update_tray_ui(payload: &TrayStatePayload) {
        let time = payload.session_time.as_deref().unwrap_or("00:00:00");
        let dl = payload.download_pkts.unwrap_or(0);
        let ul = payload.upload_pkts.unwrap_or(0);
        let server = &payload.server;

        let (st, sr, sg, sb) = if payload.connected {
            ("Connected", 0.0_f64, 0.902_f64, 0.463_f64)
        } else if payload.connecting {
            ("Connecting...", 1.0_f64, 0.843_f64, 0.251_f64)
        } else {
            ("Disconnected", 0.459_f64, 0.459_f64, 0.459_f64)
        };

        let action_txt = if payload.connected {
            format!("Disconnect from {}", server)
        } else if payload.connecting {
            "Connecting...".to_string()
        } else {
            format!("Connect {}", server)
        };
        let action_en = !payload.connecting;

        unsafe {
            let guard = TRAY_OBJ.lock().unwrap();
            if let Some(t) = guard.as_ref() {
                set_label_str(t.status_label, st);
                set_label_color(t.status_label, sr, sg, sb, 1.0);
                set_label_str(t.timer_label, time);
                set_label_str(t.dl_label, &format!("↓{}", dl));
                set_label_str(t.ul_label, &format!("↑{}", ul));

                let title = NSString::from_str(&action_txt);
                let _: () = msg_send![t.action_button, setTitle: &title];
                let _: () = msg_send![t.action_button, setEnabled: action_en];
            }
        }
    }

    unsafe fn set_label_str(label: Id, text: &str) {
        let s = NSString::from_str(text);
        let _: () = msg_send![label, setStringValue: &s];
    }

    unsafe fn set_label_color(label: Id, r: f64, g: f64, b: f64, a: f64) {
        let c: Id = msg_send![class!(NSColor), colorWithRed: r green: g blue: b alpha: a];
        let _: () = msg_send![label, setTextColor: c];
    }
}

// ═══════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════

#[cfg(target_os = "macos")]
pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    mac::set_action_callback(move |action: &str| {
        match action {
            "connect_toggle" => { let _ = handle.emit("tray-connect", ()); }
            "show" => {
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => { handle.exit(0); }
            _ => {}
        }
    });
    mac::setup_native_tray()
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>)
}

#[cfg(target_os = "macos")]
pub fn update_tray_state(
    _app: &tauri::AppHandle,
    payload: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TRAY] update: c={} s='{}' t='{}' dl={} ul={}",
        payload.connected, payload.server,
        payload.session_time.as_deref().unwrap_or(""),
        payload.download_pkts.unwrap_or(0),
        payload.upload_pkts.unwrap_or(0));
    mac::update_tray_ui(&payload);
    Ok(())
}

// ── Non-macOS stubs ──

#[cfg(not(target_os = "macos"))]
pub fn setup_tray(_app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TRAY] non-macOS: stub");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn update_tray_state(_app: &tauri::AppHandle, _payload: TrayStatePayload)
    -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
