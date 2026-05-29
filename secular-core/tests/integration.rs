// secular-core/tests/integration.rs
// Integration tests for secular-core protocol engine

use secular_core::config::{SecularConfig, ServerConfig};
use secular_core::protocol::ConnectionState;
use secular_core::protocol::SecularEngine;

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
fn test_config_validation() {
    // Valid config
    let config = test_config();
    assert!(config.validate().is_ok());

    // Empty host
    let mut bad = test_config();
    bad.server.host = "".into();
    assert!(bad.validate().is_err());

    // Empty SNI
    let mut bad = test_config();
    bad.server.sni = "".into();
    assert!(bad.validate().is_err());

    // Empty token
    let mut bad = test_config();
    bad.server.auth_token = "".into();
    assert!(bad.validate().is_err());

    // Invalid port
    let mut bad = test_config();
    bad.server.port = 0;
    assert!(bad.validate().is_err());
}

#[test]
fn test_engine_creation() {
    let config = test_config();
    let engine = SecularEngine::new(config);
    assert!(engine.is_ok());
}

#[test]
fn test_engine_bad_config() {
    let config = SecularConfig {
        server: ServerConfig {
            host: "".into(),
            port: 0,
            sni: "".into(),
            auth_token: "".into(),
        },
        ..Default::default()
    };
    let engine = SecularEngine::new(config);
    assert!(engine.is_err());
}

#[tokio::test]
async fn test_engine_initial_state() {
    let config = test_config();
    let engine = SecularEngine::new(config).unwrap();
    assert_eq!(engine.state().await, ConnectionState::Disconnected);
}

#[tokio::test]
async fn test_engine_disconnect_when_disconnected() {
    let config = test_config();
    let engine = SecularEngine::new(config).unwrap();
    // Should not panic
    let _ = engine.disconnect().await;
    assert_eq!(engine.state().await, ConnectionState::Disconnected);
}

#[test]
fn test_version() {
    assert_eq!(secular_core::VERSION, "0.1.0");
}

#[test]
fn test_mtu_defaults() {
    use secular_core::mtu::{MtuClamper, DEFAULT_MTU, SAFE_MTU};
    
    let m = MtuClamper::new();
    assert_eq!(m.current(), DEFAULT_MTU);
    assert_eq!(m.safe_mtu(), SAFE_MTU);
}

#[test]
fn test_dns_guard_creation() {
    use secular_core::dns::DnsGuard;
    
    let guard = DnsGuard::new(5353, "9.9.9.9", true);
    assert!(guard.is_ok());
}

#[test]
fn test_ipv6_blackhole() {
    use secular_core::network::Ipv6Blackhole;
    
    // Should not panic (stub implementation)
    assert!(Ipv6Blackhole::enable().is_ok());
    assert!(Ipv6Blackhole::disable().is_ok());
}
