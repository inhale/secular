// secular-core/src/lib.rs
// Secular VPN Core Library
// Protocol engine, DNS leak prevention, MTU clamping, uTLS fingerprinting

#![allow(dead_code)]
#![allow(missing_docs)]

pub mod config;
pub mod dns;
pub mod mtu;
pub mod network;
pub mod protocol;
pub mod utls;

#[cfg(feature = "uniffi")]
pub mod ffi;

pub use config::SecularConfig;
pub use protocol::SecularEngine;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Result type alias for Secular operations
pub type SecularResult<T> = anyhow::Result<T>;
