// secular-core/src/mtu.rs
// Dynamic MTU clamping — prevents fragmentation-based detection

use tracing::debug;

/// Default MTU values by platform
#[cfg(target_os = "macos")]
pub const DEFAULT_MTU: u16 = 1420;
#[cfg(target_os = "linux")]
pub const DEFAULT_MTU: u16 = 1420;
#[cfg(target_os = "windows")]
pub const DEFAULT_MTU: u16 = 1400;
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub const DEFAULT_MTU: u16 = 1400;

/// Safe MTU for HTTP/2 wrapped traffic over standard Ethernet
pub const SAFE_MTU: u16 = 1380;

/// MTU clamping engine
pub struct MtuClamper {
    /// Current MTU value
    current: u16,
    /// Minimum allowed MTU
    min_mtu: u16,
    /// Maximum allowed MTU
    max_mtu: u16,
}

impl MtuClamper {
    /// Create a new MTU clamper with auto-detection
    pub fn new() -> Self {
        Self {
            current: DEFAULT_MTU,
            min_mtu: 576,
            max_mtu: 1500,
        }
    }

    /// Create with explicit MTU
    pub fn with_mtu(mtu: u16) -> Self {
        let mut s = Self::new();
        s.current = mtu.clamp(s.min_mtu, s.max_mtu);
        s
    }

    /// Get the current safe MTU for packet construction
    pub fn safe_mtu(&self) -> u16 {
        self.current.min(SAFE_MTU)
    }

    /// Get the raw current MTU
    pub fn current(&self) -> u16 {
        self.current
    }

    /// Attempt path MTU discovery
    /// In production, this performs ICMP-based PMTUD or uses TCP MSS clamping
    pub fn discover_mtu(&mut self, host: &str) -> u16 {
        debug!("Performing MTU discovery for {host}...");
        // TODO: Implement actual PMTUD
        // For now, use safe default
        self.current = SAFE_MTU;
        self.current
    }

    /// Adjust MTU based on protocol overhead
    pub fn adjust_for_protocol(&mut self, protocol: &str) -> u16 {
        self.current = match protocol {
            // HTTP/2 framing overhead: ~40 bytes (frame header + HPACK)
            "h2" => self.current.saturating_sub(40).max(self.min_mtu),
            // QUIC overhead: ~48 bytes (header + encryption)
            "quic" => self.current.saturating_sub(48).max(self.min_mtu),
            _ => self.current,
        };
        debug!("MTU adjusted for {}: {}", protocol, self.current);
        self.current
    }
}

impl Default for MtuClamper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_mtu() {
        let m = MtuClamper::new();
        assert!(m.current() >= 1400);
    }

    #[test]
    fn test_safe_mtu() {
        let m = MtuClamper::with_mtu(1500);
        assert_eq!(m.safe_mtu(), SAFE_MTU);
    }

    #[test]
    fn test_protocol_adjustment() {
        let mut m = MtuClamper::with_mtu(1420);
        let h2_mtu = m.adjust_for_protocol("h2");
        assert!(h2_mtu < 1420);
        assert!(h2_mtu >= 576);
    }

    #[test]
    fn test_clamping() {
        let m = MtuClamper::with_mtu(2000);
        assert_eq!(m.current(), 1500); // clamped to max
    }
}
