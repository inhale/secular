// src-tauri/src/commands.rs
// Tauri command handlers — bridge frontend UI to secular-core engine

use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;

/// Application state shared across commands
pub struct AppState {
    /// Secular engine handle (would be secular_core::SecularEngine in production)
    pub connected: Mutex<bool>,
    pub config: Mutex<ConnectionConfig>,
}

/// Connection configuration from the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub sni: String,
    pub auth_token: String,
    pub protocol: String,
    pub allow_ipv6: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 443,
            sni: String::new(),
            auth_token: String::new(),
            protocol: "h2".into(),
            allow_ipv6: false,
        }
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
#[tauri::command]
pub async fn connect(
    config: ConnectionConfig,
    state: State<'_, AppState>,
) -> Result<ConnectionState, String> {
    tracing::info!("Connect requested: {}:{}", config.host, config.port);

    // TODO: Initialize secular-core engine and connect
    // let engine = SecularEngine::new(config.into())?;
    // engine.connect().await?;

    let mut connected = state.connected.lock().unwrap();
    *connected = true;
    let mut cfg = state.config.lock().unwrap();
    *cfg = config.clone();

    Ok(ConnectionState {
        connected: true,
        server: format!("{}:{}", config.host, config.port),
        protocol: config.protocol,
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
            format!("{}:{}", config.host, config.port)
        } else {
            String::new()
        },
        protocol: config.protocol.clone(),
        bytes_sent: 0,
        bytes_received: 0,
    })
}

/// Get current configuration
#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<ConnectionConfig, String> {
    Ok(state.config.lock().unwrap().clone())
}

/// Update configuration
#[tauri::command]
pub async fn set_config(
    config: ConnectionConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    *cfg = config;
    Ok(())
}

/// Read a text file (for TOML upload)
#[tauri::command]
pub async fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))
}
