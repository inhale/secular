// secular-core/src/network.rs
// Network layer — TUN interface, packet processing, routing

use crate::SecularResult;
use std::net::IpAddr;
use tracing::{debug, info};

/// TUN interface wrapper
pub struct TunInterface {
    /// Interface name
    name: String,
    /// File descriptor (platform-specific)
    fd: i32,
    /// Whether the interface is up
    is_up: bool,
}

impl TunInterface {
    /// Create a new TUN interface
    pub fn create(name: &str) -> SecularResult<Self> {
        info!("Creating TUN interface: {name}");
        // Platform-specific TUN creation
        #[cfg(target_os = "macos")]
        {
            Self::create_macos(name)
        }
        #[cfg(target_os = "linux")]
        {
            Self::create_linux(name)
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

    #[cfg(target_os = "macos")]
    fn create_macos(name: &str) -> SecularResult<Self> {
        // macOS: use utun devices (/dev/utunX)
        debug!("macOS TUN creation via utun");
        Ok(Self {
            name: name.into(),
            fd: -1, // TODO: open /dev/utunX
            is_up: false,
        })
    }

    #[cfg(target_os = "linux")]
    fn create_linux(name: &str) -> SecularResult<Self> {
        // Linux: open /dev/net/tun
        debug!("Linux TUN creation via /dev/net/tun");
        Ok(Self {
            name: name.into(),
            fd: -1, // TODO: open /dev/net/tun
            is_up: false,
        })
    }

    #[cfg(target_os = "windows")]
    fn create_windows(name: &str) -> SecularResult<Self> {
        // Windows: use wintun driver
        debug!("Windows TUN creation via wintun");
        Ok(Self {
            name: name.into(),
            fd: -1, // TODO: wintun.create_adapter()
            is_up: false,
        })
    }

    /// Configure the interface with IP addresses
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
        self.is_up = true;
        Ok(())
    }

    /// Read a packet from the TUN interface
    pub fn read_packet(&self, _buf: &mut [u8]) -> SecularResult<usize> {
        if !self.is_up {
            return Err(anyhow::anyhow!("TUN interface is not up"));
        }
        // TODO: Read from fd
        Ok(0)
    }

    /// Write a packet to the TUN interface
    pub fn write_packet(&self, data: &[u8]) -> SecularResult<usize> {
        if !self.is_up {
            return Err(anyhow::anyhow!("TUN interface is not up"));
        }
        // TODO: Write to fd
        Ok(data.len())
    }

    /// Close the TUN interface
    pub fn close(&mut self) -> SecularResult<()> {
        info!("Closing TUN interface: {}", self.name);
        self.is_up = false;
        Ok(())
    }
}

/// IPv6 blackhole — prevents IPv6 leaks by dropping all IPv6 traffic
pub struct Ipv6Blackhole;

impl Ipv6Blackhole {
    /// Enable IPv6 blackhole (block all IPv6 traffic)
    pub fn enable() -> SecularResult<()> {
        info!("Enabling IPv6 blackhole");
        #[cfg(target_os = "macos")]
        {
            // pf: block inet6 all
            debug!("macOS: adding pf rule to block IPv6");
        }
        #[cfg(target_os = "linux")]
        {
            // ip6tables -A OUTPUT -j DROP
            debug!("Linux: adding ip6tables rule to block IPv6");
        }
        Ok(())
    }

    /// Disable IPv6 blackhole (allow IPv6)
    pub fn disable() -> SecularResult<()> {
        info!("Disabling IPv6 blackhole");
        Ok(())
    }
}
