// src-tauri/src/tray.rs
// macOS NSPopover tray with native NSView controls via objc2.

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

#[cfg(target_os = "macos")]
mod mac {
    use super::TrayStatePayload;
    use objc2::declare::ClassDecl;
    use objc2::runtime::{AnyObject, NSObject, Sel};
    use objc2::{class, msg_send, ClassType};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{
        CGPoint, CGRect, CGSize, MainThreadMarker, NSAutoreleasePool, NSString, NSURL,
    };
    use std::ffi::CString;
    use std::sync::Mutex;

    type Id = *mut AnyObject;

    struct TrayObj {
        popover: Id, status_label: Id, timer_label: Id,
        dl_label: Id, ul_label: Id, action_button: Id,
    }
    unsafe impl Send for TrayObj {}
    unsafe impl Sync for TrayObj {}

    static TRAY: Mutex<Option<TrayObj>> = Mutex::new(None);
    static mut POPOVER_ID: usize = 0;

    type ActionFn = Box<dyn Fn(&str) + Send + Sync>;
    static ON_ACTION: Mutex<Option<ActionFn>> = Mutex::new(None);

    pub fn set_action_callback(f: impl Fn(&str) + Send + Sync + 'static) {
        *ON_ACTION.lock().unwrap() = Some(Box::new(f));
    }
    fn fire(action: &str) {
        if let Some(ref cb) = *ON_ACTION.lock().unwrap() { cb(action); }
    }
    fn s(name: &str) -> Sel {
        unsafe { objc2::runtime::sel_registerName(CString::new(name).unwrap().as_ptr()) }
    }

    pub fn setup_native_tray() -> Result<(), String> {
        let _pool = unsafe { NSAutoreleasePool::new() };
        unsafe {
            let class_name = std::ffi::CString::new("SecularTrayTarget").unwrap();
            let mut decl = ClassDecl::new(&class_name, NSObject::class())
                .ok_or("ClassDecl failed")?;

            extern "C" fn icon_click(_self: &AnyObject, _cmd: Sel, sender: Id) {
                unsafe {
                    let pop = POPOVER_ID as Id;
                    if pop.is_null() { return; }
                    let shown: bool = msg_send![pop, isShown];
                    if shown {
                        let _: () = msg_send![pop, performClose: std::ptr::null::<AnyObject>()];
                    } else {
                        let rect = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
                        let _: () = msg_send![pop, showRelativeToRect: rect ofView: sender preferredEdge: 1u64];
                        let mtm = MainThreadMarker::new().unwrap();
                        let app = NSApplication::sharedApplication(mtm);
                        let _: () = msg_send![app, activateIgnoringOtherApps: true];
                        eprintln!("[TRAY] popover shown");
                    }
                }
            }
            decl.add_method(s("trayIconClick:"), icon_click as extern "C" fn(&AnyObject, Sel, Id));

            extern "C" fn action_click(_self: &AnyObject, _cmd: Sel, _s: Id) {
                fire("connect_toggle");
            }
            decl.add_method(s("trayAction:"), action_click as extern "C" fn(&AnyObject, Sel, Id));

            extern "C" fn show_click(_self: &AnyObject, _cmd: Sel, _s: Id) {
                fire("show");
            }
            decl.add_method(s("trayShow:"), show_click as extern "C" fn(&AnyObject, Sel, Id));

            extern "C" fn quit_click(_self: &AnyObject, _cmd: Sel, _s: Id) {
                fire("quit");
            }
            decl.add_method(s("trayQuit:"), quit_click as extern "C" fn(&AnyObject, Sel, Id));

            let cls = decl.register();
            let target: Id = msg_send![cls, new];
            let (cv, st, tl, dl, ul, ab) = build_content(target);

            let pop: Id = msg_send![class!(NSPopover), new];
            let _: () = msg_send![pop, setBehavior: 1i64];
            let _: () = msg_send![pop, setAnimates: true];
            let _: () = msg_send![pop, setContentSize: CGSize::new(300.0, 175.0)];
            let vc: Id = msg_send![class!(NSViewController), new];
            let _: () = msg_send![vc, setView: cv];
            let _: () = msg_send![pop, setContentViewController: vc];
            POPOVER_ID = pop as usize;

            *TRAY.lock().unwrap() = Some(TrayObj {
                popover: pop, status_label: st, timer_label: tl,
                dl_label: dl, ul_label: ul, action_button: ab,
            });

            let bar: Id = msg_send![class!(NSStatusBar), systemStatusBar];
            let item: Id = msg_send![bar, statusItemWithLength: -1i64];

            let mut loaded = false;
            for name in &["tray-inactive.png","tray_inactive.png","icon.png",
                "icons/tray-inactive.png","icons/icon.png",
                "../icons/tray-inactive.png","../icons/icon.png",
                "src-tauri/icons/tray-inactive.png","src-tauri/icons/icon.png"] {
                if std::path::Path::new(name).exists() {
                    let abs = std::fs::canonicalize(name).unwrap().display().to_string();
                    let ns_str = NSString::from_str(&abs);
                    let url = NSURL::fileURLWithPath(&ns_str);
                    let img: Id = msg_send![class!(NSImage), alloc];
                    let img: Id = msg_send![img, initWithContentsOfURL: &url];
                    if !img.is_null() {
                        let _: () = msg_send![img, setTemplate: true];
                        let btn: Id = msg_send![item, button];
                        let _: () = msg_send![btn, setImage: img];
                        loaded = true;
                        eprintln!("[TRAY] icon: {}", abs);
                        break;
                    }
                }
            }
            if !loaded {
                let btn: Id = msg_send![item, button];
                let title = NSString::from_str("S");
                let _: () = msg_send![btn, setTitle: &title];
            }

            let btn: Id = msg_send![item, button];
            let _: () = msg_send![btn, setTarget: target];
            let _: () = msg_send![btn, setAction: s("trayIconClick:")];
            eprintln!("[TRAY] ready");
            Ok(())
        }
    }

    const PW: f64 = 300.0;
    const PH: f64 = 175.0;
    const PD: f64 = 14.0;

    unsafe fn label(text: &str, x: f64, y: f64, w: f64, h: f64,
                   size: f64, r: f64, g: f64, b: f64, a: f64) -> Id {
        let tf: Id = msg_send![class!(NSTextField), new];
        let s = NSString::from_str(text);
        let _: () = msg_send![tf, setStringValue: &s];
        let _: () = msg_send![tf, setBezeled: false];
        let _: () = msg_send![tf, setDrawsBackground: false];
        let _: () = msg_send![tf, setEditable: false];
        let _: () = msg_send![tf, setSelectable: false];
        let c: Id = msg_send![class!(NSColor), colorWithRed: r green: g blue: b alpha: a];
        let _: () = msg_send![tf, setTextColor: c];
        let f: Id = msg_send![class!(NSFont), systemFontOfSize: size];
        let _: () = msg_send![tf, setFont: f];
        let _: () = msg_send![tf, setFrame: CGRect::new(CGPoint::new(x,y), CGSize::new(w,h))];
        tf
    }

    unsafe fn button(title: &str, x: f64, y: f64, w: f64, h: f64, target: Id, action: Sel) -> Id {
        let b: Id = msg_send![class!(NSButton), new];
        let s = NSString::from_str(title);
        let _: () = msg_send![b, setTitle: &s];
        let _: () = msg_send![b, setBezelStyle: 1i64];
        let _: () = msg_send![b, setTarget: target];
        let _: () = msg_send![b, setAction: action];
        let f: Id = msg_send![class!(NSFont), systemFontOfSize: 12.0];
        let _: () = msg_send![b, setFont: f];
        let _: () = msg_send![b, setFrame: CGRect::new(CGPoint::new(x,y), CGSize::new(w,h))];
        b
    }

    unsafe fn separator(x: f64, y: f64, w: f64) -> Id {
        let s: Id = msg_send![class!(NSBox), new];
        let _: () = msg_send![s, setBoxType: 3i64];
        let _: () = msg_send![s, setFrame: CGRect::new(CGPoint::new(x,y), CGSize::new(w,1.0))];
        s
    }

    unsafe fn build_content(target: Id) -> (Id, Id, Id, Id, Id, Id) {
        let v: Id = msg_send![class!(NSView), new];
        let bg: Id = msg_send![class!(NSColor), colorWithRed: 0.11 green: 0.11 blue: 0.12 alpha: 1.0];
        let _: () = msg_send![v, setWantsLayer: true];
        let l: Id = msg_send![v, layer];
        let cg: Id = msg_send![bg, CGColor];
        let _: () = msg_send![l, setBackgroundColor: cg];
        let _: () = msg_send![v, setFrame: CGRect::new(CGPoint::new(0.0,0.0), CGSize::new(PW,PH))];

        let cw = PW - PD * 2.0;
        let mut y = PH - PD;

        y -= 18.0;
        let st = label("Disconnected", PD, y, cw, 18.0, 13.0, 0.46, 0.46, 0.46, 1.0);
        let _: () = msg_send![v, addSubview: st];

        y -= 26.0;
        let tl = label("00:00:00", PD, y, cw, 26.0, 19.0, 1.0, 1.0, 1.0, 1.0);
        let _: () = msg_send![v, addSubview: tl];

        y -= 18.0;
        let dl = label("\u{2193}0", PD, y, 70.0, 18.0, 11.0, 0.533, 0.808, 0.98, 1.0);
        let _: () = msg_send![v, addSubview: dl];
        let ul = label("\u{2191}0", PD+75.0, y, 70.0, 18.0, 11.0, 0.443, 0.776, 0.443, 1.0);
        let _: () = msg_send![v, addSubview: ul];

        y -= 14.0;
        let sp = separator(PD, y, cw);
        let _: () = msg_send![v, addSubview: sp];

        y -= 32.0;
        let ab = button("Connect", PD, y, cw, 26.0, target, s("trayAction:"));
        let _: () = msg_send![v, addSubview: ab];

        y -= 28.0;
        let (hw, gap) = ((cw - 4.0) / 2.0, 4.0);
        let sb = button("Show Secular", PD, y, hw, 22.0, target, s("trayShow:"));
        let _: () = msg_send![v, addSubview: sb];
        let qb = button("Quit", PD+hw+gap, y, hw, 22.0, target, s("trayQuit:"));
        let _: () = msg_send![v, addSubview: qb];

        (v, st, tl, dl, ul, ab)
    }

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
                let s = NSString::from_str(st);
                let _: () = msg_send![t.status_label, setStringValue: &s];
                let c: Id = msg_send![class!(NSColor), colorWithRed: sr green: sg blue: sb alpha: 1.0];
                let _: () = msg_send![t.status_label, setTextColor: c];

                let s = NSString::from_str(time);
                let _: () = msg_send![t.timer_label, setStringValue: &s];

                let s = NSString::from_str(&format!("\u{2193}{}", dl));
                let _: () = msg_send![t.dl_label, setStringValue: &s];

                let s = NSString::from_str(&format!("\u{2191}{}", ul));
                let _: () = msg_send![t.ul_label, setStringValue: &s];

                let title = NSString::from_str(&action_txt);
                let _: () = msg_send![t.action_button, setTitle: &title];
                let _: () = msg_send![t.action_button, setEnabled: !payload.connecting];
            }
        }
    }
}

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
pub fn update_tray_state(_: &tauri::AppHandle, payload: TrayStatePayload)
    -> Result<(), Box<dyn std::error::Error>> {
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
