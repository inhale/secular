// secular-core/src/config.rs
// Configuration types for Secular VPN

use serde::{Deserialize, Serialize};

/// Server endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerConfig {
    /// Server IP address or hostname
    pub host: String,
    /// Server port
    pub port: u16,
    /// SNI hostname (for TLS handshake)
    pub sni: String,
    /// Authentication token
    pub auth_token: String,
}

/// Full Secular configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecularConfig {
    /// Server to connect to
    pub server: ServerConfig,
    /// DNS resolver inside the tunnel (default: 9.9.9.9)
    pub dns_resolver: String,
    /// Enable IPv6 routing (default: false for leak prevention)
    pub allow_ipv6: bool,
    /// MTU size (0 = auto-detect)
    pub mtu: u16,
    /// Enable uTLS fingerprint randomization
    pub enable_utls: bool,
    /// Kill switch: block all traffic if tunnel drops
    pub kill_switch: bool,
    /// Protocol: "h2" for HTTP/2, "quic" for QUIC
    pub protocol: String,
}

impl Default for SecularConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: String::new(),
                port: 443,
                sni: String::new(),
                auth_token: String::new(),
            },
            dns_resolver: "9.9.9.9".into(),
            allow_ipv6: false,
            mtu: 0,
            enable_utls: true,
            kill_switch: true,
            protocol: "h2".into(),
        }
    }
}

impl SecularConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.server.host.is_empty() {
            return Err("Server host is required".into());
        }
        if self.server.sni.is_empty() {
            return Err("SNI hostname is required".into());
        }
        if self.server.auth_token.is_empty() {
            return Err("Auth token is required".into());
        }
        if self.server.port == 0 {
            return Err("Invalid port".into());
        }
        Ok(())
    }
}
