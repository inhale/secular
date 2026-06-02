// src-tauri/src/commands.rs
// Tauri command handlers — bridge frontend UI to TrustTunnel VPN client

use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;

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
        toml.push_str("loglevel = \"info\"\n");
        toml.push_str("killswitch_enabled = false\n");
        toml.push_str("post_quantum_group_enabled = false\n\n");
        toml.push_str("[listener.tun]\n");
        toml.push_str("included_routes = [\"0.0.0.0/0\", \"::/0\"]\n");
        toml.push_str("excluded_routes = []\n");
        toml.push_str("mtu_size = 1500\n");
        toml.push_str("change_system_dns = false\n\n");
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
#[tauri::command]
pub async fn connect(
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

    // Find trusttunnel_client binary
    let tt_binary = if cfg!(target_os = "macos") {
        // Check common locations
        let candidates = [
            "/usr/local/bin/trusttunnel_client",
            "/opt/homebrew/bin/trusttunnel_client",
        ];
        candidates.iter().find(|p| std::path::Path::new(p).exists())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "trusttunnel_client".to_string())
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

    // Spawn trusttunnel_client as background process
    let skip_flag = if config.skip_verification { Some("-s") } else { None };
    let mut cmd = std::process::Command::new(&tt_binary);
    cmd.arg("--config").arg(&config_path);
    if config.skip_verification {
        cmd.arg("-s");
    }
    cmd.arg("--loglevel").arg("debug");

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
