// src-tauri/src/commands.rs
// Tauri command handlers — bridge frontend UI to TrustTunnel VPN client

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use std::sync::Mutex;

/// Check if a process is still running
fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pid;
        true
    }
}

/// Application state shared across commands
pub struct AppState {
    /// Whether the VPN tunnel is up
    pub connected: Mutex<bool>,
    pub config: Mutex<ServerConfig>,
    /// PID of the running trusttunnel_client process (0 = none)
    pub tunnel_pid: Mutex<u32>,
}

/// Server config matching Android ServerProfile / TrustTunnel TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// IP:port address (e.g. "185.103.24.4:443")
    pub address: String,
    /// SNI hostname for TLS handshake
    pub hostname: String,
    /// Username for TrustTunnel auth
    pub username: String,
    /// Password for TrustTunnel auth
    pub password: String,
    /// Protocol: "http2" | "http3"
    pub upstream_protocol: String,
    /// DNS upstreams
    pub dns_upstreams: Vec<String>,
    /// Allow IPv6 traffic
    pub has_ipv6: bool,
    /// Certificate PEM or path
    pub certificate: String,
    /// Skip TLS verification
    pub skip_verification: bool,
    /// Anti-DPI
    pub anti_dpi: bool,
    /// Change system DNS to route through tunnel
    pub change_system_dns: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: String::new(),
            hostname: String::new(),
            username: String::new(),
            password: String::new(),
            upstream_protocol: "http2".into(),
            dns_upstreams: vec!["9.9.9.9".into(), "149.112.112.112".into()],
            has_ipv6: false,
            certificate: String::new(),
            skip_verification: false,
            anti_dpi: false,
            change_system_dns: true,
        }
    }
}

impl ServerConfig {
    /// Generate TrustTunnel TOML config string (matches Android toTrustTunnelToml())
    pub fn to_toml(&self) -> String {
        let sni = if self.hostname.is_empty() {
            self.address.split(':').next().unwrap_or("")
        } else {
            &self.hostname
        };

        let addr = if self.address.is_empty() {
            "\"0.0.0.0:443\"".to_string()
        } else {
            format!("\"{}\"", self.address)
        };

        let proto = match self.upstream_protocol.as_str() {
            "http3" => "http3",
            _ => "auto",
        };

        let dns_list = if self.dns_upstreams.is_empty() {
            "\"9.9.9.9\", \"149.112.112.112\"".to_string()
        } else {
            self.dns_upstreams.iter()
                .map(|d| format!("\"{}\"", d))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let mut toml = String::new();
        toml.push_str("vpn_mode = \"general\"\n");
        toml.push_str("loglevel = \"trace\"\n");
        toml.push_str("killswitch_enabled = false\n");
        toml.push_str("post_quantum_group_enabled = false\n\n");
        toml.push_str("[listener.tun]\n");
        if self.has_ipv6 {
            toml.push_str("included_routes = [\"0.0.0.0/0\", \"::/0\"]\n");
        } else {
            toml.push_str("included_routes = [\"0.0.0.0/0\"]\n");
        }
        // Exclude VPN server IP and Tailscale subnet from tunnel to avoid routing loops
        let server_ip = self.address.split(':').next().unwrap_or("");
        let mut excludes = vec!["100.64.0.0/10".to_string()]; // Tailscale CGNAT range
        if !server_ip.is_empty() {
            excludes.push(format!("{}/32", server_ip));
        }
        let excl_str = excludes.iter().map(|e| format!("\"{}\"", e)).collect::<Vec<_>>().join(", ");
        toml.push_str(&format!("excluded_routes = [{}]\n", excl_str));
        toml.push_str("mtu_size = 1500\n");
        toml.push_str(&format!("change_system_dns = {}\n\n", self.change_system_dns));
        toml.push_str("[endpoint]\n");
        toml.push_str(&format!("hostname = \"{}\"\n", sni));
        toml.push_str(&format!("addresses = [{}]\n", addr));
        toml.push_str(&format!("username = \"{}\"\n", self.username));
        toml.push_str(&format!("password = \"{}\"\n", self.password));
        toml.push_str(&format!("upstream_protocol = \"{}\"\n", proto));
        toml.push_str(&format!("dns_upstreams = [{}]\n", dns_list));
        toml.push_str(&format!("has_ipv6 = {}\n", self.has_ipv6));
        if !self.certificate.is_empty() {
            // Use triple-quoted string for PEM cert
            toml.push_str(&format!("certificate = \"\"\"{}\n\"\"\"\n", self.certificate));
        }
        toml.push_str(&format!("skip_verification = {}\n", self.skip_verification));
        toml.push_str(&format!("anti_dpi = {}\n", self.anti_dpi));
        toml
    }
}

/// Connection state for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionState {
    pub connected: bool,
    pub server: String,
    pub protocol: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// Connect to the VPN server via trusttunnel_client CLI

/// Find trusttunnel_client in system paths (fallback when not bundled)
fn find_system_trusttunnel() -> String {
    let candidates = [
        "/usr/local/bin/trusttunnel_client",
        "/opt/homebrew/bin/trusttunnel_client",
    ];
    candidates.iter().find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "trusttunnel_client".to_string())
}

#[tauri::command]
pub async fn connect(
    app: tauri::AppHandle,
    config: ServerConfig,
    state: State<'_, AppState>,
) -> Result<ConnectionState, String> {
    // Validate required fields (matching Android's checks)
    if config.address.is_empty() {
        return Err("Server address is required".into());
    }
    if !config.address.contains(':') {
        return Err(format!("Address '{}' is missing a port. Use format: host:port (e.g. {}:443)", config.address, config.address));
    }
    if config.username.is_empty() || config.password.is_empty() {
        return Err("Username and password are required".into());
    }

    tracing::info!("Connect requested: {} (SNI: {})", config.address, config.hostname);

    // Generate TrustTunnel TOML config
    let toml_config = config.to_toml();
    tracing::info!("TrustTunnel TOML config:\n{}", toml_config);

    // Write TOML to temp config file
    let config_dir = std::env::temp_dir().join("secular");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    let config_path = config_dir.join("trusttunnel_client.toml");
    std::fs::write(&config_path, &toml_config)
        .map_err(|e| format!("Failed to write config: {}", e))?;
    tracing::info!("Config written to {:?}", config_path);

    // Find trusttunnel_client binary — check bundled resource first, then system paths
    let tt_binary = if cfg!(target_os = "macos") {
        // Check if bundled with the app (in Contents/Resources)
        if let Ok(resource_path) = app.path().resolve("binaries/trusttunnel_client", tauri::path::BaseDirectory::Resource) {
            if resource_path.exists() {
                eprintln!("[CONNECT] using bundled trusttunnel_client: {:?}", resource_path);
                resource_path.to_string_lossy().to_string()
            } else {
                eprintln!("[CONNECT] bundled binary not found, checking system paths");
                find_system_trusttunnel()
            }
        } else {
            find_system_trusttunnel()
        }
    } else {
        "trusttunnel_client".to_string()
    };

    // Kill any existing tunnel process
    {
        let mut pid = state.tunnel_pid.lock().unwrap();
        if *pid > 0 {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .output();
            *pid = 0;
        }
    }

    // Spawn trusttunnel_client as background process (via sudo on macOS for TUN device)
    let skip_flag = if config.skip_verification { Some("-s") } else { None };
    let mut cmd = if cfg!(target_os = "macos") {
        // macOS: use sudo (passwordless via /etc/sudoers.d/trusttunnel) to create utun device
        let mut c = std::process::Command::new("/usr/bin/sudo");
        c.arg("-n") // non-interactive — fail if no sudoers entry
         .arg(&tt_binary);
        c
    } else {
        std::process::Command::new(&tt_binary)
    };
    cmd.arg("--config").arg(&config_path);
    if config.skip_verification {
        cmd.arg("-s");
    }
    cmd.arg("--loglevel").arg("trace");

    // Redirect stdout/stderr to log file
    let log_path = config_dir.join("trusttunnel.log");
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Failed to create log file: {}", e))?;
    cmd.stdout(log_file.try_clone().map_err(|e| e.to_string())?);
    cmd.stderr(log_file);

    let child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn trusttunnel_client: {}. Is it installed at {}?", e, tt_binary))?;

    let child_pid = child.id();
    tracing::info!("trusttunnel_client spawned with PID {}", child_pid);

    // Store PID and mark connected
    {
        let mut pid = state.tunnel_pid.lock().unwrap();
        *pid = child_pid;
    }
    {
        let mut connected = state.connected.lock().unwrap();
        *connected = true;
    }
    {
        let mut cfg = state.config.lock().unwrap();
        *cfg = config.clone();
    }

    // Wait for tunnel to actually establish by polling the log
    let log_path_check = config_dir.join("trusttunnel.log");
    let mut connected_confirmed = false;
    for _ in 0..60 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        if log_path_check.exists() {
            if let Ok(content) = std::fs::read_to_string(&log_path_check) {
                if content.contains("TLS handshake completed")
                    || content.contains("Connection stable")
                    || content.contains("tunnel is up")
                    || content.contains("connected")
                {
                    connected_confirmed = true;
                    tracing::info!("Tunnel connection confirmed");
                    break;
                }
                if !is_process_alive(child_pid) {
                    tracing::warn!("trusttunnel_client process died during connection");
                    break;
                }
            }
        }
    }

    if !connected_confirmed {
        tracing::warn!("Tunnel connection not confirmed within timeout, but process may still be connecting");
    }

    Ok(ConnectionState {
        connected: true,
        server: config.address.clone(),
        protocol: config.upstream_protocol.clone(),
        bytes_sent: 0,
        bytes_received: 0,
    })
}

/// Disconnect from the VPN server
#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    tracing::info!("Disconnect requested");

    // Kill the trusttunnel_client process
    let pid = {
        let mut p = state.tunnel_pid.lock().unwrap();
        let old = *p;
        *p = 0;
        old
    };

    if pid > 0 {
        tracing::info!("Killing trusttunnel_client PID {}", pid);
        let _ = std::process::Command::new("kill")
            .arg(pid.to_string())
            .output();
        // Also kill any child processes
        let _ = std::process::Command::new("pkill")
            .arg("-P")
            .arg(pid.to_string())
            .output();
        // On macOS, trusttunnel_client may be a child of sudo — kill by name too
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("sudo")
                .arg("-n")
                .arg("pkill")
                .arg("-f")
                .arg("trusttunnel_client")
                .output();
        }
    }

    // On macOS, remove only tunnel routes (don't flush all routes!)
    #[cfg(target_os = "macos")]
    {
        // Remove utun* routes specifically
        if let Ok(output) = std::process::Command::new("route")
            .arg("-n")
            .arg("show")
            .output()
        {
            if let Ok(text) = String::from_utf8(output.stdout) {
                for line in text.lines() {
                    let iface = line.split_whitespace().last().unwrap_or("");
                    if iface.starts_with("utun") {
                        let _ = std::process::Command::new("route")
                            .arg("-n")
                            .arg("delete")
                            .arg(line.split_whitespace().next().unwrap_or(""))
                            .arg(iface)
                            .output();
                    }
                }
            }
        }
        // Restore default gateway from DHCP lease
        let _ = std::process::Command::new("ipconfig")
            .arg("set")
            .arg("en0")
            .arg("DHCP")
            .output();
        // Flush DNS cache
        let _ = std::process::Command::new("dscacheutil")
            .arg("-flushcache")
            .output();
    }

    {
        let mut connected = state.connected.lock().unwrap();
        *connected = false;
    }

    Ok(())
}

/// Get current connection state
#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Result<ConnectionState, String> {
    let connected = state.connected.lock().unwrap();
    let config = state.config.lock().unwrap();
    let pid = state.tunnel_pid.lock().unwrap();

    // Check if process is still alive
    let actually_connected = if *connected && *pid > 0 {
        // Check if the process still exists
        std::process::Command::new("kill")
            .arg("-0") // just check if process exists
            .arg(pid.to_string())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };

    Ok(ConnectionState {
        connected: actually_connected,
        server: if actually_connected {
            config.address.clone()
        } else {
            String::new()
        },
        protocol: config.upstream_protocol.clone(),
        bytes_sent: 0,
        bytes_received: 0,
    })
}

/// Get current configuration
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<ServerConfig, String> {
    Ok(state.config.lock().unwrap().clone())
}

/// Update configuration
#[tauri::command]
pub async fn set_config(
    config: ServerConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    *cfg = config;
    Ok(())
}

/// Read a text file (for TOML/cert upload)
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))
}

/// Read the trusttunnel_client log
#[tauri::command]
pub async fn read_tunnel_log() -> Result<String, String> {
    let log_path = std::env::temp_dir().join("secular/trusttunnel.log");
    if log_path.exists() {
        std::fs::read_to_string(&log_path).map_err(|e| format!("Failed to read log: {}", e))
    } else {
        Ok("No tunnel log yet".into())
    }
}

#[tauri::command]
pub fn show_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn debug_log(msg: String) {
    use std::io::Write;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[{}] {}\n", ts, msg);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/secular-debug.log") {
        let _ = f.write_all(line.as_bytes());
    }
    eprintln!("[DEBUG] {}", msg);
}

#[tauri::command]
pub fn update_tray(
    app: tauri::AppHandle,
    connected: bool,
    connecting: bool,
    server: String,
    sessionTime: String,
    downloadPkts: u64,
    uploadPkts: u64,
) -> Result<(), String> {
    eprintln!("[CMD] update_tray: connected={}, connecting={}, server={}, time={}, dl={}, ul={}",
        connected, connecting, server, sessionTime, downloadPkts, uploadPkts);
    debug_log(format!("update_tray: connected={}, connecting={}, server={}, time={}, dl={}, ul={}",
        connected, connecting, server, sessionTime, downloadPkts, uploadPkts));
    let payload = crate::tray::TrayStatePayload {
        connected,
        connecting,
        server,
        session_time: if sessionTime.is_empty() { None } else { Some(sessionTime) },
        download_pkts: Some(downloadPkts),
        upload_pkts: Some(uploadPkts),
    };
    crate::tray::update_tray_state(&app, payload).map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct TrayAction {
    pub action: String,
    pub screen: Option<String>,
}

#[tauri::command]
pub fn tray_action(app: tauri::AppHandle, payload: TrayAction) -> Result<(), String> {
    eprintln!("[TRAY] action: {}", payload.action);
    match payload.action.as_str() {
        "connect" => {
            eprintln!("[TRAY] connect action received, emitting tray-connect");
            let _ = app.emit("tray-connect", ());
            eprintln!("[TRAY] tray-connect emitted");
        }
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "hide" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        "quit" => {
            app.exit(0);
        }
        "nav" => {
            if let Some(screen) = payload.screen {
                let _ = app.emit("tray-nav", screen);
            }
        }
        _ => {}
    }
    // Close the popup window after action
    if let Some(popup) = app.get_webview_window("tray-menu") {
        let _ = popup.hide();
    }
    Ok(())
}
