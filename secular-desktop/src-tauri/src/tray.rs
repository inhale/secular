// src-tauri/src/tray.rs
// macOS NSPopover tray with native NSView controls.
//
// Solves: NSMenu snapshots content on open — can't update stats live.
// NSPopover + NSView: NSTextField.setStringValue updates immediately.
//
// Uses raw objc runtime via msg_send! with sel_registerName (no sel! macro)
// because objc 0.2.7's sel! macro is broken on Rust 1.96.

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
    use cocoa::base::{id, nil, YES, NO};
    use cocoa::foundation::{NSAutoreleasePool, NSString, NSURL};
    use objc::declare::ClassDecl;
    use objc::runtime::{Object, Sel, BOOL};
    use objc::{class, msg_send};

    // ── Shared UI object references ──────────────────────────

    struct TrayObj {
        popover: id,
        status_label: id,
        timer_label: id,
        dl_label: id,
        ul_label: id,
        action_button: id,
    }
    unsafe impl Send for TrayObj {}
    unsafe impl Sync for TrayObj {}

    static TRAY: Mutex<Option<TrayObj>> = Mutex::new(None);
    static mut POPOVER_ID: usize = 0;

    // Rust callback bridge for ObjC button actions
    type ActionFn = Box<dyn Fn(&str) + Send + Sync>;
    static ON_ACTION: Mutex<Option<ActionFn>> = Mutex::new(None);

    pub fn set_action_callback(f: impl Fn(&str) + Send + Sync + 'static) {
        *ON_ACTION.lock().unwrap() = Some(Box::new(f));
    }

    fn fire(action: &str) {
        if let Some(ref cb) = *ON_ACTION.lock().unwrap() { cb(action); }
    }

    // Helper: get selector by name (avoids broken sel! macro)
    unsafe fn s(name: &str) -> Sel {
        objc::runtime::sel_registerName(
            std::ffi::CString::new(name).unwrap().as_ptr()
        )
    }

    // ── ObjC delegate class ──────────────────────────────────

    pub fn setup_native_tray() -> Result<(), String> {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Register SecularTrayTarget class
            let mut decl = ClassDecl::new("SecularTrayTarget",
                class!(NSObject)).ok_or("ClassDecl failed")?;

            extern "C" fn icon_click(_self: &Object, _cmd: Sel, sender: id) {
                unsafe {
                    let pop = POPOVER_ID as id;
                    if pop.is_null() { return; }
                    let shown: BOOL = msg_send![pop, isShown];
                    if shown == YES {
                        let _: () = msg_send![pop, performClose:nil];
                    } else {
                        let rect = cocoa::foundation::NSRect::new(
                            cocoa::foundation::NSPoint::new(0.0, 0.0),
                            cocoa::foundation::NSSize::new(1.0, 1.0));
                        let _: () = msg_send![pop, showRelativeToRect:rect ofView:sender preferredEdge:1u64];
                        unsafe { let app: id = msg_send![class!(NSApplication), sharedApplication];
                            let _: () = msg_send![app, activateIgnoringOtherApps:YES]; }
                        eprintln!("[TRAY] popover shown");
                    }
                }
            }
            decl.add_method(s("trayIconClick:"),
                icon_click as extern "C" fn(&Object, Sel, id));

            extern "C" fn action_click(_self: &Object, _cmd: Sel, _sender: id) {
                eprintln!("[TRAY] action"); fire("connect_toggle");
            }
            decl.add_method(s("trayAction:"),
                action_click as extern "C" fn(&Object, Sel, id));

            extern "C" fn show_click(_self: &Object, _cmd: Sel, _sender: id) {
                eprintln!("[TRAY] show"); fire("show");
            }
            decl.add_method(s("trayShow:"),
                show_click as extern "C" fn(&Object, Sel, id));

            extern "C" fn quit_click(_self: &Object, _cmd: Sel, _sender: id) {
                eprintln!("[TRAY] quit"); fire("quit");
            }
            decl.add_method(s("trayQuit:"),
                quit_click as extern "C" fn(&Object, Sel, id));

            let cls = decl.register();
            let target: id = msg_send![cls, new];

            // Build content
            let (cv, st, tl, dl, ul, ab) = build_content(target);

            // Create NSPopover
            let pop: id = msg_send![class!(NSPopover), new];
            let _: () = msg_send![pop, setBehavior:1i64];
            let _: () = msg_send![pop, setAnimates:YES];
            let _: () = msg_send![pop, setContentSize:
                cocoa::foundation::NSSize::new(300.0, 175.0)];
            let vc: id = msg_send![class!(NSViewController), new];
            let _: () = msg_send![vc, setView:cv];
            let _: () = msg_send![pop, setContentViewController:vc];
            POPOVER_ID = pop as usize;

            *TRAY.lock().unwrap() = Some(TrayObj {
                popover: pop, status_label: st, timer_label: tl,
                dl_label: dl, ul_label: ul, action_button: ab,
            });

            // Status bar item
            let bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: id = msg_send![bar, statusItemWithLength:-1i64];

            // Icon
            let mut loaded = false;
            for name in &[
                "tray-inactive.png","tray_inactive.png","icon.png",
                "icons/tray-inactive.png","icons/icon.png",
                "../icons/tray-inactive.png","../icons/icon.png",
                "src-tauri/icons/tray-inactive.png","src-tauri/icons/icon.png"] {
                if std::path::Path::new(name).exists() {
                    let abs = std::fs::canonicalize(name).unwrap().display().to_string();
                    let url = NSURL::fileURLWithPath_(nil,
                        NSString::alloc(nil).init_str(&abs));
                    let img: id = msg_send![class!(NSImage), alloc];
                    let img: id = msg_send![img, initWithContentsOfURL:url];
                    if img != nil {
                        let _: () = msg_send![img, setTemplate:YES];
                        let btn: id = msg_send![item, button];
                        let _: () = msg_send![btn, setImage:img];
                        loaded = true;
                        eprintln!("[TRAY] icon: {}", abs);
                        break;
                    }
                }
            }
            if !loaded {
                let btn: id = msg_send![item, button];
                let _: () = msg_send![btn, setTitle:
                    NSString::alloc(nil).init_str("S")];
            }

            let btn: id = msg_send![item, button];
            let _: () = msg_send![btn, setTarget:target];
            let _: () = msg_send![btn, setAction:s("trayIconClick:")];

            eprintln!("[TRAY] ready");
            Ok(())
        }
    }

    // ── NSView builder ────────────────────────────────────────

    const PW: f64 = 300.0;
    const PH: f64 = 175.0;
    const PD: f64 = 14.0;

    unsafe fn label(text: &str, x: f64, y: f64, w: f64, h: f64,
                   size: f64, r: f64, g: f64, b: f64, a: f64) -> id {
        let tf: id = msg_send![class!(NSTextField), new];
        let _: () = msg_send![tf, setStringValue:NSString::alloc(nil).init_str(text)];
        let _: () = msg_send![tf, setBezeled:NO];
        let _: () = msg_send![tf, setDrawsBackground:NO];
        let _: () = msg_send![tf, setEditable:NO];
        let _: () = msg_send![tf, setSelectable:NO];
        let c: id = msg_send![class!(NSColor), colorWithRed:r green:g blue:b alpha:a];
        let _: () = msg_send![tf, setTextColor:c];
        let f: id = msg_send![class!(NSFont), systemFontOfSize:size];
        let _: () = msg_send![tf, setFont:f];
        let _: () = msg_send![tf, setFrame:cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(x,y),
            cocoa::foundation::NSSize::new(w,h))];
        tf
    }

    unsafe fn button(title: &str, x: f64, y: f64, w: f64, h: f64,
                    target: id, action: Sel) -> id {
        let b: id = msg_send![class!(NSButton), new];
        let _: () = msg_send![b, setTitle:NSString::alloc(nil).init_str(title)];
        let _: () = msg_send![b, setBezelStyle:1i64];
        let _: () = msg_send![b, setTarget:target];
        let _: () = msg_send![b, setAction:action];
        let f: id = msg_send![class!(NSFont), systemFontOfSize:12.0];
        let _: () = msg_send![b, setFont:f];
        let _: () = msg_send![b, setFrame:cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(x,y),
            cocoa::foundation::NSSize::new(w,h))];
        b
    }

    unsafe fn separator(x: f64, y: f64, w: f64) -> id {
        let s: id = msg_send![class!(NSBox), new];
        let _: () = msg_send![s, setBoxType:3i64];
        let _: () = msg_send![s, setFrame:cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(x,y),
            cocoa::foundation::NSSize::new(w,1.0))];
        s
    }

    unsafe fn build_content(target: id) -> (id, id, id, id, id, id) {
        let v: id = msg_send![class!(NSView), new];
        let bg: id = msg_send![class!(NSColor), colorWithRed:0.11 green:0.11 blue:0.12 alpha:1.0];
        let _: () = msg_send![v, setWantsLayer:YES];
        let l: id = msg_send![v, layer];
        let _: () = msg_send![l, setBackgroundColor:msg_send![bg, CGColor]];
        let _: () = msg_send![v, setFrame:cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(0.0,0.0),
            cocoa::foundation::NSSize::new(PW,PH))];

        let cw = PW - PD * 2.0;
        let mut y = PH - PD;

        y -= 18.0;
        let st = label("Disconnected", PD, y, cw, 18.0, 13.0, 0.46, 0.46, 0.46, 1.0);
        let _: () = msg_send![v, addSubview:st];

        y -= 26.0;
        let tl = label("00:00:00", PD, y, cw, 26.0, 19.0, 1.0, 1.0, 1.0, 1.0);
        let _: () = msg_send![v, addSubview:tl];

        y -= 18.0;
        let dl = label("↓0", PD, y, 70.0, 18.0, 11.0, 0.533, 0.808, 0.98, 1.0);
        let _: () = msg_send![v, addSubview:dl];
        let ul = label("↑0", PD+75.0, y, 70.0, 18.0, 11.0, 0.443, 0.776, 0.443, 1.0);
        let _: () = msg_send![v, addSubview:ul];

        y -= 14.0;
        let sp = separator(PD, y, cw);
        let _: () = msg_send![v, addSubview:sp];

        y -= 32.0;
        let ab = button("Connect", PD, y, cw, 26.0, target, s("trayAction:"));
        let _: () = msg_send![v, addSubview:ab];

        y -= 28.0;
        let (hw, gap) = ((cw - 4.0) / 2.0, 4.0);
        let sb = button("Show Secular", PD, y, hw, 22.0, target, s("trayShow:"));
        let _: () = msg_send![v, addSubview:sb];
        let qb = button("Quit", PD+hw+gap, y, hw, 22.0, target, s("trayQuit:"));
        let _: () = msg_send![v, addSubview:qb];

        (v, st, tl, dl, ul, ab)
    }

    // ── Live-update labels ────────────────────────────────────

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

        unsafe {
            let g = TRAY.lock().unwrap();
            if let Some(ref t) = *g {
                set_str(t.status_label, st);
                set_color(t.status_label, sr, sg, sb, 1.0);
                set_str(t.timer_label, time);
                set_str(t.dl_label, &format!("↓{}", dl));
                set_str(t.ul_label, &format!("↑{}", ul));
                set_str(t.action_button, &action_txt); // button uses setTitle:
                let _: () = msg_send![t.action_button, setEnabled:
                    if !payload.connecting { YES } else { NO }];
            }
        }
    }

    unsafe fn set_str(obj: id, text: &str) {
        let _: () = msg_send![obj, setStringValue:
            NSString::alloc(nil).init_str(text)];
    }
    unsafe fn set_color(obj: id, r: f64, g: f64, b: f64, a: f64) {
        let c: id = msg_send![class!(NSColor), colorWithRed:r green:g blue:b alpha:a];
        let _: () = msg_send![obj, setTextColor:c];
    }
}

// ═══════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════

#[cfg(target_os = "macos")]
pub fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let h = app.handle().clone();
    mac::set_action_callback(move |a: &str| match a {
        "connect_toggle" => { let _ = h.emit("tray-connect", ()); }
        "show" => { if let Some(w) = h.get_webview_window("main") {
            let _ = w.show(); let _ = w.set_focus(); } }
        "quit" => { h.exit(0); }
        _ => {}
    });
    mac::setup_native_tray()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e).into())
}

#[cfg(target_os = "macos")]
pub fn update_tray_state(
    _: &tauri::AppHandle, payload: TrayStatePayload,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[TRAY] c={} s={} t={} dl={} ul={}",
        payload.connected, payload.server,
        payload.session_time.as_deref().unwrap_or(""),
        payload.download_pkts.unwrap_or(0),
        payload.upload_pkts.unwrap_or(0));
    mac::update_tray_ui(&payload);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn setup_tray(_: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> { Ok(()) }

#[cfg(not(target_os = "macos"))]
pub fn update_tray_state(_: &tauri::AppHandle, _: TrayStatePayload)
    -> Result<(), Box<dyn std::error::Error>> { Ok(()) }
