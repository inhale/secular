// src-tauri/src/tray.rs
// macOS NSPopover tray with native NSView controls.
//
// Solves: NSMenu snapshots content on open — stats can't update live.
// NSPopover + NSView: NSTextField.setStringValue updates immediately.
//
// NOTE: The cocoa 0.26 crate is deprecated and only exports a limited set
// of appkit types. We use raw objc runtime (class! + msg_send!) for all
// AppKit objects (NSPopover, NSButton, NSTextField, etc.) since they're
// all just id pointers anyway.

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
    use cocoa::appkit::NSApplication;
    use cocoa::base::{id, nil, YES, NO};
    use cocoa::foundation::{NSAutoreleasePool, NSString, NSURL};
    use objc::declare::ClassDecl;
    use objc::runtime::{Object, Sel, BOOL};
    use objc::{class, msg_send, sel};

    // ── Shared native UI references ──────────────────────────

    struct TrayObj {
        popover: id,
        status_label: id,
        timer_label: id,
        dl_label: id,
        ul_label: id,
        action_button: id,
    }

    static TRAY_OBJ: Mutex<Option<TrayObj>> = Mutex::new(None);
    static mut POPOVER_PTR: usize = 0;

    // Rust callback for button clicks in the popover
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

    // Registers a SecularTrayTarget class with these action methods:
    //   trayIconClick:  — status bar icon clicked → toggle popover
    //   trayAction:     — connect/disconnect button
    //   trayShow:       — show secular button
    //   trayQuit:       — quit button

    pub fn setup_native_tray() -> Result<(), String> {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);

            // Register delegate class
            let superclass = class!(NSObject);
            let mut decl = ClassDecl::new("SecularTrayTarget", superclass)
                .map_err(|e| format!("ClassDecl::new failed: {:?}", e))?;

            // trayIconClick:
            extern "C" fn icon_click(_self: &Object, _cmd: Sel, sender: id) {
                unsafe {
                    let popover = POPOVER_PTR as id;
                    if popover == nil { return; }
                    let visible: BOOL = msg_send![popover, isShown];
                    if visible == YES {
                        let _: () = msg_send![popover, performClose:nil];
                    } else {
                        // sender is the NSStatusItem's button
                        let rect = cocoa::foundation::NSRect::new(
                            cocoa::foundation::NSPoint::new(0.0, 0.0),
                            cocoa::foundation::NSSize::new(1.0, 1.0),
                        );
                        let _: () = msg_send![popover, showRelativeToRect:rect ofView:sender preferredEdge:1];
                        let app = NSApplication::sharedApplication(nil);
                        let _: () = msg_send![app, activateIgnoringOtherApps:YES];
                        eprintln!("[TRAY] popover shown");
                    }
                }
            }
            decl.add_method(sel!(trayIconClick:), icon_click as extern "C" fn(&Object, Sel, id));

            // trayAction:
            extern "C" fn action_click(_self: &Object, _cmd: Sel, _sender: id) {
                eprintln!("[TRAY] trayAction");
                fire("connect_toggle");
            }
            decl.add_method(sel!(trayAction:), action_click as extern "C" fn(&Object, Sel, id));

            // trayShow:
            extern "C" fn show_click(_self: &Object, _cmd: Sel, _sender: id) {
                eprintln!("[TRAY] trayShow");
                fire("show");
            }
            decl.add_method(sel!(trayShow:), show_click as extern "C" fn(&Object, Sel, id));

            // trayQuit:
            extern "C" fn quit_click(_self: &Object, _cmd: Sel, _sender: id) {
                eprintln!("[TRAY] trayQuit");
                fire("quit");
            }
            decl.add_method(sel!(trayQuit:), quit_click as extern "C" fn(&Object, Sel, id));

            let cls = decl.register();
            let target: id = msg_send![cls, new];

            // Build popover content
            let (content_view, st, tl, dl, ul, ab) = build_content_view(target);

            // Create NSPopover
            let popover: id = msg_send![class!(NSPopover), new];
            let _: () = msg_send![popover, setBehavior:1 /* NSPopoverBehaviorTransient */];
            let _: () = msg_send![popover, setAnimates:YES];
            let pop_w: f64 = 300.0;
            let pop_h: f64 = 175.0;
            let _: () = msg_send![popover, setContentSize:
                cocoa::foundation::NSSize::new(pop_w, pop_h)];

            let vc: id = msg_send![class!(NSViewController), new];
            let _: () = msg_send![vc, setView:content_view];
            let _: () = msg_send![popover, setContentViewController:vc];

            POPOVER_PTR = popover as usize;

            // Store refs for live label updates
            *TRAY_OBJ.lock().unwrap() = Some(TrayObj {
                popover, status_label: st, timer_label: tl,
                dl_label: dl, ul_label: ul, action_button: ab,
            });

            // Create status bar item
            let bar: id = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: id = msg_send![bar, statusItemWithLength:-1 /* NSStatusItemVariableLength */];

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
                    let url = NSURL::fileURLWithPath_(nil, NSString::alloc(nil).init_str(&abs));
                    let img: id = msg_send![class!(NSImage), alloc];
                    let img: id = msg_send![img, initWithContentsOfURL:url];
                    if img != nil {
                        let _: () = msg_send![img, setTemplate:YES];
                        let btn: id = msg_send![item, button];
                        let _: () = msg_send![btn, setImage:img];
                        icon_loaded = true;
                        eprintln!("[TRAY] icon: {}", abs);
                        break;
                    }
                }
            }
            if !icon_loaded {
                let btn: id = msg_send![item, button];
                let _: () = msg_send![btn, setTitle:NSString::alloc(nil).init_str("S")];
                eprintln!("[TRAY] no icon, using 'S'");
            }

            // Wire button click → toggle popover
            let btn: id = msg_send![item, button];
            let _: () = msg_send![btn, setTarget:target];
            let _: () = msg_send![btn, setAction:sel!(trayIconClick:)];

            eprintln!("[TRAY] native tray ready");
            Ok(())
        }
    }

    // ── NSView builder helpers ────────────────────────────────

    const POP_W: f64 = 300.0;
    const POP_H: f64 = 175.0;
    const PAD: f64 = 14.0;

    unsafe fn nsrect(x: f64, y: f64, w: f64, h: f64) -> cocoa::foundation::NSRect {
        cocoa::foundation::NSRect::new(
            cocoa::foundation::NSPoint::new(x, y),
            cocoa::foundation::NSSize::new(w, h),
        )
    }

    unsafe fn make_label(text: &str, x: f64, y: f64, w: f64, h: f64,
                        size: f64, r: f64, g: f64, b: f64, a: f64) -> id {
        let tf: id = msg_send![class!(NSTextField), new];
        let _: () = msg_send![tf, setStringValue: NSString::alloc(nil).init_str(text)];
        let _: () = msg_send![tf, setBezeled:NO];
        let _: () = msg_send![tf, setDrawsBackground:NO];
        let _: () = msg_send![tf, setEditable:NO];
        let _: () = msg_send![tf, setSelectable:NO];
        let c: id = msg_send![class!(NSColor), colorWithRed:r green:g blue:b alpha:a];
        let _: () = msg_send![tf, setTextColor:c];
        let f: id = msg_send![class!(NSFont), systemFontOfSize:size];
        let _: () = msg_send![tf, setFont:f];
        let _: () = msg_send![tf, setFrame:nsrect(x, y, w, h)];
        tf
    }

    unsafe fn make_button(title: &str, x: f64, y: f64, w: f64, h: f64, target: id, action: Sel) -> id {
        let b: id = msg_send![class!(NSButton), new];
        let _: () = msg_send![b, setTitle: NSString::alloc(nil).init_str(title)];
        let _: () = msg_send![b, setBezelStyle:1];
        let _: () = msg_send![b, setTarget:target];
        let _: () = msg_send![b, setAction:action];
        let f: id = msg_send![class!(NSFont), systemFontOfSize:12.0];
        let _: () = msg_send![b, setFont:f];
        let _: () = msg_send![b, setFrame:nsrect(x, y, w, h)];
        b
    }

    unsafe fn make_separator(x: f64, y: f64, w: f64) -> id {
        let s: id = msg_send![class!(NSBox), new];
        let _: () = msg_send![s, setBoxType:3];
        let _: () = msg_send![s, setFrame:nsrect(x, y, w, 1.0)];
        s
    }

    unsafe fn build_content_view(target: id) -> (id, id, id, id, id, id) {
        let view: id = msg_send![class!(NSView), new];
        let bg: id = msg_send![class!(NSColor), colorWithRed:0.11 green:0.11 blue:0.12 alpha:1.0];
        let _: () = msg_send![view, setWantsLayer:YES];
        let layer: id = msg_send![view, layer];
        let cg: id = msg_send![bg, CGColor];
        let _: () = msg_send![layer, setBackgroundColor:cg];
        let _: () = msg_send![view, setFrame:nsrect(0.0, 0.0, POP_W, POP_H)];

        let cw = POP_W - PAD * 2.0;
        let mut y = POP_H - PAD;

        // Status label
        y -= 18.0;
        let st = make_label("Disconnected", PAD, y, cw, 18.0, 13.0, 0.46, 0.46, 0.46, 1.0);
        let _: () = msg_send![view, addSubview:st];

        // Timer
        y -= 26.0;
        let tl = make_label("00:00:00", PAD, y, cw, 26.0, 19.0, 1.0, 1.0, 1.0, 1.0);
        let _: () = msg_send![view, addSubview:tl];

        // Stats row
        y -= 18.0;
        let dl = make_label("↓0", PAD, y, 70.0, 18.0, 11.0, 0.533, 0.808, 0.98, 1.0);
        let _: () = msg_send![view, addSubview:dl];
        let ul = make_label("↑0", PAD + 75.0, y, 70.0, 18.0, 11.0, 0.443, 0.776, 0.443, 1.0);
        let _: () = msg_send![view, addSubview:ul];

        // Separator
        y -= 14.0;
        let sp = make_separator(PAD, y, cw);
        let _: () = msg_send![view, addSubview:sp];

        // Action button
        y -= 32.0;
        let ab = make_button("Connect", PAD, y, cw, 26.0, target, sel!(trayAction:));
        let _: () = msg_send![view, addSubview:ab];

        // Show + Quit row
        y -= 28.0;
        let (hw, gap) = ((cw - 4.0) / 2.0, 4.0);
        let sb = make_button("Show Secular", PAD, y, hw, 22.0, target, sel!(trayShow:));
        let _: () = msg_send![view, addSubview:sb];
        let qb = make_button("Quit", PAD + hw + gap, y, hw, 22.0, target, sel!(trayQuit:));
        let _: () = msg_send![view, addSubview:qb];

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

                let title = NSString::alloc(nil).init_str(&action_txt);
                let _: () = msg_send![t.action_button, setTitle:title];
                let _: () = msg_send![t.action_button, setEnabled: if action_en { YES } else { NO }];
            }
        }
    }

    unsafe fn set_label_str(label: id, text: &str) {
        let s = NSString::alloc(nil).init_str(text);
        let _: () = msg_send![label, setStringValue:s];
    }

    unsafe fn set_label_color(label: id, r: f64, g: f64, b: f64, a: f64) {
        let c: id = msg_send![class!(NSColor), colorWithRed:r green:g blue:b alpha:a];
        let _: () = msg_send![label, setTextColor:c];
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
