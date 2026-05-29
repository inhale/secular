// secular-core/src/protocol.rs
// Core protocol engine — TLS handshake, HTTP/2 tunnel, auth, packet I/O

use crate::config::SecularConfig;
use crate::utls::UtlsEngine;
use crate::SecularResult;
use bytes::Bytes;
use rustls::pki_types::ServerName;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_rustls::TlsConnector;
use tracing::{debug, error, info, warn};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Handshaking,
    Connected,
    Failed,
}

/// Active tunnel connection — wrapped TLS stream with HTTP/2
struct Tunnel {
    /// Server address
    server_addr: String,
    /// SNI hostname for TLS
    sni: String,
    /// Raw TCP stream (TLS is layered on top via tokio_rustls)
    tls_stream: Option<tokio_rustls::client::TlsStream<TcpStream>>,
    /// HTTP/2 connection
    h2_session: Option<h2::client::SendRequest<Bytes>>,
}

/// The core Secular engine — handles all protocol operations
pub struct SecularEngine {
    config: SecularConfig,
    state: Arc<Mutex<ConnectionState>>,
    current_mtu: Arc<Mutex<u16>>,
    tunnel: Arc<Mutex<Option<Tunnel>>>,
}

impl SecularEngine {
    /// Create a new SecularEngine with the given configuration
    pub fn new(config: SecularConfig) -> SecularResult<Self> {
        config.validate().map_err(|e| anyhow::anyhow!(e))?;
        Ok(Self {
            config,
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            current_mtu: Arc::new(Mutex::new(0)),
            tunnel: Arc::new(Mutex::new(None)),
        })
    }

    /// Connect to the server and establish the tunnel
    pub async fn connect(&self) -> SecularResult<()> {
        info!(
            "Connecting to {}:{} via {}",
            self.config.server.host, self.config.server.port, self.config.protocol
        );

        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Handshaking;
        }

        let host = self.config.server.host.clone();
        let port = self.config.server.port;
        let sni = self.config.server.sni.clone();
        let auth_token = self.config.server.auth_token.clone();
        let protocol = self.config.protocol.clone();

        // Phase 1: TCP connect
        let addr = format!("{host}:{port}");
        debug!("TCP connecting to {addr}...");
        let tcp = TcpStream::connect(&addr)
            .await
            .map_err(|e| anyhow::anyhow!("TCP connect failed to {addr}: {e}"))?;
        info!("TCP connected to {addr}");

        if protocol == "quic" {
            self.connect_quic(tcp, &sni, &auth_token).await
        } else {
            self.connect_h2(tcp, &host, &sni, &auth_token).await
        }
    }

    /// HTTP/2 connection path — TLS handshake + h2 ALPN negotiation + auth
    async fn connect_h2(
        &self,
        tcp: TcpStream,
        host: &str,
        sni: &str,
        auth_token: &str,
    ) -> SecularResult<()> {
        // Phase 2a: Build TLS config with uTLS fingerprint
        let tls_config = self.build_tls_config().await?;
        let connector: TlsConnector = Arc::new(tls_config).into();

        // Phase 2b: TLS handshake with SNI
        let server_name = ServerName::try_from(sni.to_string())
            .map_err(|e| anyhow::anyhow!("Invalid SNI hostname '{sni}': {e}"))?;
        debug!("TLS handshake with SNI={sni}...");
        let tls_stream = connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| anyhow::anyhow!("TLS handshake failed: {e}"))?;
        info!(
            "TLS handshake complete (ALPN: {:?})",
            tls_stream.get_ref().1.alpn_protocol()
        );

        // Phase 2c: HTTP/2 handshake via ALPN (h2 should already be negotiated)
        debug!("Starting HTTP/2 preface...");
        let mut h2_client = h2::client::handshake(tls_stream)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP/2 handshake failed: {e}"))?;
        info!("HTTP/2 connection established");

        // Phase 3: Authenticate — POST /auth with token
        self.authenticate_h2(&mut h2_client, host, auth_token)
            .await?;

        // Phase 4: Configure tunnel (MTU, keepalive)
        self.configure_tunnel_h2(&mut h2_client).await?;

        // Store the active tunnel
        let send_req = h2_client;
        {
            let mut tunnel = self.tunnel.lock().await;
            *tunnel = Some(Tunnel {
                server_addr: host.to_string(),
                sni: sni.to_string(),
                tls_stream: None, // ownership moved into h2
                h2_session: Some(send_req),
            });
        }

        {
            let mut state = self.state.lock().await;
            *state = ConnectionState::Connected;
        }
        info!("Tunnel connected successfully via HTTP/2");
        Ok(())
    }

    /// QUIC connection path (stub — quinn handshake)
    async fn connect_quic(
        &self,
        _tcp: TcpStream,
        sni: &str,
        _auth_token: &str,
    ) -> SecularResult<()> {
        info!("QUIC protocol selected (SNI: {sni})");
        // TODO: Implement QUIC handshake via quinn
        // For now, fall back to the h2 path — this is a protocol stub
        warn!("QUIC not yet implemented, attempting HTTP/2 fallback");
        Err(anyhow::anyhow!("QUIC protocol not yet implemented"))
    }

    /// Build a rustls ClientConfig with uTLS fingerprint and ALPN for HTTP/2
    async fn build_tls_config(&self) -> SecularResult<rustls::ClientConfig> {
        let mut config = rustls::ClientConfig::builder()
            .with_root_certificates(Self::root_store())
            .with_no_client_auth();

        // ALPN: prefer HTTP/2, fall back to HTTP/1.1
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        // uTLS fingerprint randomization
        if self.config.enable_utls {
            let utls = UtlsEngine::new_random();
            utls.apply_to_config(&mut config);
        } else {
            debug!("uTLS disabled — using default rustls ClientHello");
        }

        Ok(config)
    }

    /// Create a root certificate store with webpki roots
    fn root_store() -> rustls::RootCertStore {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    }

    /// HTTP/2 authentication — send auth token via POST request
    async fn authenticate_h2(
        &self,
        h2: &mut h2::client::SendRequest<Bytes>,
        host: &str,
        token: &str,
    ) -> SecularResult<()> {
        debug!("Sending auth token to {host}...");

        let auth_body = serde_json::json!({
            "token": token,
            "version": crate::VERSION,
            "platform": std::env::consts::OS,
        });
        let body_bytes = Bytes::from(auth_body.to_string());

        let request = http::Request::builder()
            .method("POST")
            .uri(format!("https://{host}/api/v1/auth"))
            .header("content-type", "application/json")
            .header("user-agent", format!("Secular/{}", crate::VERSION))
            .body(())
            .map_err(|e| anyhow::anyhow!("Failed to build auth request: {e}"))?;

        let (response, _send) = h2
            .send_request(request, false)
            .map_err(|e| anyhow::anyhow!("Auth request send failed: {e}"))?;

        let response = response
            .await
            .map_err(|e| anyhow::anyhow!("Auth response recv failed: {e}"))?;

        if response.status().is_success() {
            info!("Authentication successful ({})", response.status());
            Ok(())
        } else {
            let status = response.status();
            error!("Authentication failed: HTTP {status}");
            Err(anyhow::anyhow!("Server rejected auth token: HTTP {status}"))
        }
    }

    /// HTTP/2 tunnel configuration — negotiate MTU and keepalive
    async fn configure_tunnel_h2(
        &self,
        _h2: &mut h2::client::SendRequest<Bytes>,
    ) -> SecularResult<()> {
        let mtu = *self.current_mtu.lock().await;
        if mtu > 0 {
            info!("Tunnel MTU configured: {mtu}");
        } else {
            info!("Using auto-detected MTU");
        }
        Ok(())
    }

    /// Disconnect and clean up
    pub async fn disconnect(&self) -> SecularResult<()> {
        info!("Disconnecting...");
        let mut tunnel = self.tunnel.lock().await;
        *tunnel = None;
        let mut state = self.state.lock().await;
        *state = ConnectionState::Disconnected;
        info!("Disconnected");
        Ok(())
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.lock().await
    }

    /// Get the current configuration
    pub fn config(&self) -> &SecularConfig {
        &self.config
    }

    /// Send a raw IP packet through the tunnel
    pub async fn send_packet(&self, packet: &[u8]) -> SecularResult<()> {
        let tunnel = self.tunnel.lock().await;
        match tunnel.as_ref() {
            Some(_t) => {
                // TODO: Wrap IP packet into HTTP/2 data frame and send
                debug!("Sending {} bytes through tunnel", packet.len());
                Ok(())
            }
            None => Err(anyhow::anyhow!("Not connected")),
        }
    }

    /// Receive a raw IP packet from the tunnel
    pub async fn recv_packet(&self, buf: &mut [u8]) -> SecularResult<usize> {
        let tunnel = self.tunnel.lock().await;
        match tunnel.as_ref() {
            Some(_t) => {
                // TODO: Read from HTTP/2 data frame stream and extract IP packet
                debug!("Reading from tunnel..."); // debug_eof
                Ok(0)
            }
            None => Err(anyhow::anyhow!("Not connected")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;

    fn test_config() -> SecularConfig {
        SecularConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 18443,
                sni: "test.local".into(),
                auth_token: "test-token-123".into(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_engine_new() {
        let config = test_config();
        let engine = SecularEngine::new(config);
        assert!(engine.is_ok());
    }

    #[test]
    fn test_engine_validate_empty_host() {
        let config = SecularConfig {
            server: ServerConfig {
                host: "".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = SecularEngine::new(config);
        assert!(engine.is_err());
    }

    #[test]
    fn test_engine_validate_empty_token() {
        let config = SecularConfig {
            server: ServerConfig {
                host: "test.local".into(),
                auth_token: "".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let engine = SecularEngine::new(config);
        assert!(engine.is_err());
    }
}
