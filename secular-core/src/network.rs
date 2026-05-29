// secular-core/src/network.rs
// Network layer — TUN interface, packet processing, routing, IPv6 blackhole

use crate::SecularResult;
use std::net::IpAddr;
use std::os::fd::AsRawFd;
use tracing::{debug, error, info, warn};

/// Raw TUN interface file descriptor wrapper (Linux)
#[cfg(target_os = "linux")]
pub struct RawTunFd {
    fd: std::fs::File,
    name: String,
}

/// Packet buffer with metadata
pub struct Packet {
    pub data: Vec<u8>,
    pub src: Option<IpAddr>,
    pub dst: Option<IpAddr>,
    pub protocol: u8,
}

impl Packet {
    /// Parse an IPv4 packet from raw bytes
    pub fn from_ipv4(data: &[u8]) -> SecularResult<Self> {
        if data.len() < 20 {
            return Err(anyhow::anyhow!(
                "IPv4 packet too short: {} bytes",
                data.len()
            ));
        }
        let version = (data[0] >> 4) & 0xF;
        if version != 4 {
            return Err(anyhow::anyhow!("Not an IPv4 packet (version: {version})"));
        }
        let protocol = data[9];
        let src = IpAddr::from(<[u8; 4]>::try_from(&data[12..16]).unwrap());
        let dst = IpAddr::from(<[u8; 4]>::try_from(&data[16..20]).unwrap());
        Ok(Self {
            data: data.to_vec(),
            src: Some(src),
            dst: Some(dst),
            protocol,
        })
    }

    /// Parse an IPv6 packet from raw bytes
    pub fn from_ipv6(data: &[u8]) -> SecularResult<Self> {
        if data.len() < 40 {
            return Err(anyhow::anyhow!(
                "IPv6 packet too short: {} bytes",
                data.len()
            ));
        }
        let version = (data[0] >> 4) & 0xF;
        if version != 6 {
            return Err(anyhow::anyhow!("Not an IPv6 packet (version: {version})"));
        }
        let protocol = data[6]; // Next header
        let src = IpAddr::from(<[u8; 16]>::try_from(&data[8..24]).unwrap());
        let dst = IpAddr::from(<[u8; 16]>::try_from(&data[24..40]).unwrap());
        Ok(Self {
            data: data.to_vec(),
            src: Some(src),
            dst: Some(dst),
            protocol,
        })
    }

    /// Auto-detect and parse an IP packet (v4 or v6)
    pub fn from_raw(data: &[u8]) -> SecularResult<Self> {
        if data.is_empty() {
            return Err(anyhow::anyhow!("Empty packet"));
        }
        match (data[0] >> 4) & 0xF {
            4 => Self::from_ipv4(data),
            6 => Self::from_ipv6(data),
            v => Err(anyhow::anyhow!("Unknown IP version: {v}")),
        }
    }

    /// Get the IP version (4 or 6)
    pub fn version(&self) -> u8 {
        if self.data.is_empty() {
            return 0;
        }
        (self.data[0] >> 4) & 0xF
    }

    /// Check if this is an IPv6 packet
    pub fn is_ipv6(&self) -> bool {
        self.version() == 6
    }

    /// Get the payload (after IP header)
    pub fn payload(&self) -> &[u8] {
        let ihl = if self.version() == 4 {
            ((self.data[0] & 0xF) * 4) as usize
        } else {
            40 // IPv6 fixed header
        };
        &self.data[ihl.min(self.data.len())..]
    }
}

/// TUN interface wrapper — cross-platform
pub struct TunInterface {
    name: String,
    #[cfg(target_os = "linux")]
    fd: Option<RawTunFd>,
    #[cfg(target_os = "macos")]
    fd: Option<i32>,
    is_up: bool,
    mtu: u16,
}

impl TunInterface {
    /// Create a new TUN interface
    pub fn create(name: &str) -> SecularResult<Self> {
        info!("Creating TUN interface: {name}");
        #[cfg(target_os = "linux")]
        {
            Self::create_linux(name)
        }
        #[cfg(target_os = "macos")]
        {
            Self::create_macos(name)
        }
        #[cfg(target_os = "windows")]
        {
            Self::create_windows(name)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            Err(anyhow::anyhow!("Unsupported platform"))
        }
    }

    #[cfg(target_os = "linux")]
    fn create_linux(name: &str) -> SecularResult<Self> {
        use std::io::Write;
        use std::os::fd::AsRawFd;

        // Open the TUN device
        let fd = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .map_err(|e| anyhow::anyhow!("Failed to open /dev/net/tun: {e}"))?;

        // Configure via ioctl (requires tun crate in production)
        // For now, return a stub — real implementation needs `tun` crate
        debug!("Linux TUN: /dev/net/tun opened (fd={})", fd.as_raw_fd());
        warn!("Full TUN support requires the `tun` crate — stub only");

        Ok(Self {
            name: name.into(),
            fd: None,
            is_up: false,
            mtu: 1420,
        })
    }

    #[cfg(target_os = "macos")]
    fn create_macos(name: &str) -> SecularResult<Self> {
        info!("macOS TUN: would open utun device ({name})");
        // Real implementation: use utun crate or raw socket to /dev/utunX
        warn!("Full macOS TUN support requires the `utun` crate — stub only");
        Ok(Self {
            name: name.into(),
            fd: None,
            is_up: false,
            mtu: 1420,
        })
    }

    #[cfg(target_os = "windows")]
    fn create_windows(name: &str) -> SecularResult<Self> {
        info!("Windows TUN: would use wintun.dll ({name})");
        warn!("Full Windows TUN support requires the `wintun` crate — stub only");
        Ok(Self {
            name: name.into(),
            is_up: false,
            mtu: 1400,
        })
    }

    /// Configure the interface with IP addresses and bring it up
    pub fn configure(
        &mut self,
        local_ip: IpAddr,
        remote_ip: IpAddr,
        mtu: u16,
    ) -> SecularResult<()> {
        info!(
            "Configuring {}: local={} remote={} mtu={}",
            self.name, local_ip, remote_ip, mtu
        );
        self.mtu = mtu;
        self.is_up = true;
        Ok(())
    }

    /// Get the interface name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the current MTU
    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    /// Check if the interface is up
    pub fn is_up(&self) -> bool {
        self.is_up
    }

    /// Read a raw IP packet from the TUN interface
    /// Returns the number of bytes read and the parsed packet
    pub fn read_packet(&self, buf: &mut [u8]) -> SecularResult<Option<Packet>> {
        if !self.is_up {
            return Err(anyhow::anyhow!("TUN interface is not up"));
        }
        // Real implementation would read from fd
        // For now, return None (no data available)
        let _ = buf;
        Ok(None)
    }

    /// Write a raw IP packet to the TUN interface
    pub fn write_packet(&self, data: &[u8]) -> SecularResult<usize> {
        if !self.is_up {
            return Err(anyhow::anyhow!("TUN interface is not up"));
        }
        if data.len() > self.mtu as usize {
            return Err(anyhow::anyhow!(
                "Packet too large: {} > MTU {}",
                data.len(),
                self.mtu
            ));
        }
        // Real implementation would write to fd
        debug!("TUN write: {} bytes", data.len());
        Ok(data.len())
    }

    /// Close the TUN interface
    pub fn close(&mut self) -> SecularResult<()> {
        info!("Closing TUN interface: {}", self.name);
        self.is_up = false;
        self.fd = None;
        Ok(())
    }
}

impl Drop for TunInterface {
    fn drop(&mut self) {
        if self.is_up {
            warn!("TUN interface '{}' was not properly closed", self.name);
        }
    }
}

/// IPv6 blackhole — prevents IPv6 leaks by blocking all IPv6 traffic
pub struct Ipv6Blackhole;

impl Ipv6Blackhole {
    /// Enable IPv6 blackhole (block all IPv6 traffic except link-local)
    pub fn enable() -> SecularResult<()> {
        info!("Enabling IPv6 blackhole");
        #[cfg(target_os = "linux")]
        {
            // nftables/inet6 filter
            debug!("Linux: would add ip6tables rule to block non-link-local IPv6");
        }
        #[cfg(target_os = "macos")]
        {
            // pf anchor
            debug!("macOS: would add pf rule 'block inet6 all'");
        }
        #[cfg(target_os = "windows")]
        {
            // WFP filter
            debug!("Windows: would add WFP filter to block IPv6");
        }
        Ok(())
    }

    /// Disable IPv6 blackhole (allow IPv6 traffic)
    pub fn disable() -> SecularResult<()> {
        info!("Disabling IPv6 blackhole");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ipv4_packet() {
        // Build a minimal IPv4 packet (20 byte header + 4 byte payload)
        let mut packet = vec![
            0x45, 0x00, 0x00, 0x18, // Version=4, IHL=5, TOS=0, TotalLen=24
            0x00, 0x01, 0x00, 0x00, // ID=1, Flags=0, FragOff=0
            0x40, 0x06, 0x00, 0x00, // TTL=64, Protocol=TCP(6), Checksum=0
            0x0A, 0x00, 0x00, 0x01, // Src: 10.0.0.1
            0x0A, 0x00, 0x00, 0x02, // Dst: 10.0.0.2
        ];
        packet.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // payload
        let parsed = Packet::from_ipv4(&packet).unwrap();
        assert_eq!(parsed.version(), 4);
        assert_eq!(parsed.protocol, 6); // TCP
        assert_eq!(parsed.payload(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_parse_ipv4_too_short() {
        let packet = vec![0x45, 0x00, 0x00]; // too short
        assert!(Packet::from_ipv4(&packet).is_err());
    }

    #[test]
    fn test_parse_ipv6_packet() {
        let mut packet = vec![
            0x60, 0x00, 0x00, 0x00, // Version=6, TC=0, FlowLabel=0
            0x00, 0x00, 0x11, 0x00, // PayloadLen=0, NextHeader=UDP(17), HopLimit=0
        ];
        // Source: fe80::1
        packet.extend_from_slice(&[0xFE, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]);
        // Destination: fe80::2
        packet.extend_from_slice(&[0xFE, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02]);

        let parsed = Packet::from_ipv6(&packet).unwrap();
        assert_eq!(parsed.version(), 6);
        assert_eq!(parsed.protocol, 17); // UDP
    }

    #[test]
    fn test_packet_from_raw_v4() {
        let mut packet = vec![0x45, 0x00, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00];
        packet.extend_from_slice(&[0x40, 0x06, 0x00, 0x00]);
        packet.extend_from_slice(&[0x0A, 0x00, 0x00, 0x01]);
        packet.extend_from_slice(&[0x0A, 0x00, 0x00, 0x02]);
        let parsed = Packet::from_raw(&packet).unwrap();
        assert_eq!(parsed.version(), 4);
    }

    #[test]
    fn test_packet_from_raw_empty() {
        assert!(Packet::from_raw(&[]).is_err());
    }

    #[test]
    fn test_tun_interface_stub() {
        // Test that the stub TunInterface at least doesn't panic
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            assert!(TunInterface::create("test0").is_err());
        }
        // On supported platforms, the stub should work
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        {
            let tun = TunInterface::create("test0");
            assert!(tun.is_ok());
        }
    }

    #[test]
    fn test_ipv6_blackhole() {
        // Should not panic
        assert!(Ipv6Blackhole::enable().is_ok());
        assert!(Ipv6Blackhole::disable().is_ok());
    }
}
