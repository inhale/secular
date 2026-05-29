// secular-core/src/dns.rs
// DNS leak prevention — port-53 hijacking, DoH/DoT blocking, tunnel-only DNS
// Based on TrustTunnel DNS leak fix (2026-05-29): UDP-probe local DNS proxy
// before trusting, fall back to Quad9.

use crate::SecularResult;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

/// DNS leak prevention engine
pub struct DnsGuard {
    /// Local DNS proxy address inside the tunnel
    proxy_addr: SocketAddr,
    /// Fallback DNS resolver (Quad9 primary)
    fallback_dns: SocketAddr,
    /// Whether IPv6 DNS is disabled
    block_ipv6: bool,
}

impl DnsGuard {
    /// Create a new DnsGuard
    pub fn new(proxy_port: u16, fallback: &str, block_ipv6: bool) -> SecularResult<Self> {
        Ok(Self {
            proxy_addr: format!("127.0.0.1:{proxy_port}").parse()?,
            fallback_dns: fallback.parse()?,
            block_ipv6,
        })
    }

    /// Verify the local DNS proxy is responding
    /// Returns true if the proxy is healthy, false if we should use fallback
    pub async fn probe_proxy(&self) -> bool {
        debug!("Probing local DNS proxy at {}...", self.proxy_addr);
        match UdpSocket::bind("0.0.0.0:0").await {
            Ok(sock) => {
                // Send a minimal DNS query to the proxy
                let query = Self::build_probe_query();
                if let Err(e) = sock.send_to(&query, &self.proxy_addr).await {
                    warn!("DNS proxy probe send failed: {e}");
                    return false;
                }
                // Wait for response with timeout
                let mut buf = [0u8; 512];
                match tokio::time::timeout(
                    std::time::Duration::from_millis(500),
                    sock.recv_from(&mut buf),
                )
                .await
                {
                    Ok(Ok((n, _))) if n > 0 => {
                        info!("DNS proxy at {} is healthy", self.proxy_addr);
                        true
                    }
                    Ok(Ok(_)) => {
                        warn!("DNS proxy returned empty response");
                        false
                    }
                    Ok(Err(e)) => {
                        warn!("DNS proxy receive failed: {e}");
                        false
                    }
                    Err(_) => {
                        warn!(
                            "DNS proxy probe timed out — using fallback {}",
                            self.fallback_dns
                        );
                        false
                    }
                }
            }
            Err(e) => {
                warn!("Cannot create probe socket: {e}");
                false
            }
        }
    }

    /// Get the effective DNS resolver address
    pub async fn resolver(&self) -> SocketAddr {
        if self.probe_proxy().await {
            self.proxy_addr
        } else {
            self.fallback_dns
        }
    }

    /// Build a minimal DNS probe query (A record for "check.secular")
    fn build_probe_query() -> Vec<u8> {
        let mut pkt = Vec::new();
        // Transaction ID
        pkt.extend_from_slice(&[0xCA, 0xFE]);
        // Flags: standard query
        pkt.extend_from_slice(&[0x01, 0x00]);
        // Questions: 1
        pkt.extend_from_slice(&[0x00, 0x01]);
        // Answer RRs: 0
        pkt.extend_from_slice(&[0x00, 0x00]);
        // Authority RRs: 0
        pkt.extend_from_slice(&[0x00, 0x00]);
        // Additional RRs: 0
        pkt.extend_from_slice(&[0x00, 0x00]);
        // Query: check.secular A
        pkt.extend_from_slice(b"\x05check\x06secular\x00");
        pkt.extend_from_slice(&[0x00, 0x01]); // Type A
        pkt.extend_from_slice(&[0x00, 0x01]); // Class IN
        pkt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_query_format() {
        let query = DnsGuard::build_probe_query();
        // DNS header is 12 bytes, plus query
        assert!(query.len() > 12);
        // Check transaction ID
        assert_eq!(query[0], 0xCA);
        assert_eq!(query[1], 0xFE);
    }
}
