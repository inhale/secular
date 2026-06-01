// src-tauri/src/commands.rs
// Tauri command handlers — bridge frontend UI to secular-core engine
// Data model matches Android ServerProfile / TrustTunnel TOML config

use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;

/// Application state shared across commands
pub struct AppState {
    /// Whether we think we're connected (stub until native TrustTunnel is wired)
    pub connected: Mutex<bool>,
    pub config: Mutex<ServerConfig>,
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
        toml.push_str("loglevel = \"debug\"\n");
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
            toml.push_str(&format!("certificate = \"{}\"\n", self.certificate));
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

/// Connect to the VPN server
/// Currently a stub — sets connected=true.
/// TODO: Wire to TrustTunnel native library (XCFramework on macOS)
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

    // Generate the TrustTunnel TOML config for future native integration
    let toml_config = config.to_toml();
    tracing::info!("TrustTunnel TOML config:\n{}", toml_config);

    // TODO: Actually connect via TrustTunnel native library
    // On macOS: use TrustTunnelClient.xcframework (Swift/ObjC adapter)
    // The TOML config above is ready to pass to VpnClient(tomlConfig, listener)

    let mut connected = state.connected.lock().unwrap();
    *connected = true;
    let mut cfg = state.config.lock().unwrap();
    *cfg = config.clone();

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

    let mut connected = state.connected.lock().unwrap();
    *connected = false;

    Ok(())
}

/// Get current connection state
#[tauri::command]
pub async fn get_state(state: State<'_, AppState>) -> Result<ConnectionState, String> {
    let connected = state.connected.lock().unwrap();
    let config = state.config.lock().unwrap();

    Ok(ConnectionState {
        connected: *connected,
        server: if *connected {
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
