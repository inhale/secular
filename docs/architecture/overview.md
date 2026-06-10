# Secular — Architecture

## Overview

Secular uses **TrustTunnel's native C++ libraries** as the VPN engine on every platform. The secular codebase provides the UI/UX layer and server configuration management.

```
┌─────────────────────────────────────────────────────────┐
│                    Platform Clients                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐               │
│  │  Tauri   │ │ Android  │ │ Windows  │               │
│  │  Desktop │ │  Kotlin  │ │  (Tauri) │               │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘               │
│       │             │            │                       │
│       │  subprocess │  JNI/AAR   │  subprocess          │
│       │             │            │                       │
│  ┌────┴─────────────┴────────────┴────┐                 │
│  │     TrustTunnel Native Libraries    │                 │
│  │     (C++, trusttunnel_client)      │                 │
│  └────────────────────────────────────┘                 │
└─────────────────────────────────────────────────────────┘
```

## Protocol

Secular wraps all traffic in HTTP/2 or QUIC streams that mimic standard web traffic:

1. **Handshake:** Client connects to server with TLS, authenticates via username/password
2. **Obfuscation:** All packets are wrapped in HTTP/2 DATA frames or QUIC STREAM frames
3. **uTLS:** ClientHello fingerprints are randomized to avoid TLS fingerprinting
4. **DNS:** All DNS queries are routed through the tunnel's configured DNS upstreams
5. **Bypass List:** Per-server domain/IP exclusions via TrustTunnel's `exclusions` config

## Platform Integration

| Platform | Language | VPN Engine | Integration |
|----------|----------|------------|-------------|
| macOS | Rust (Tauri) + React | TrustTunnel CLI subprocess | `src-tauri/` |
| Android | Kotlin | TrustTunnel AAR (JNI) | `VpnService` |
| Windows | Rust (Tauri) + React | TrustTunnel CLI subprocess | `src-tauri/` |

## TrustTunnel Config

Each server profile generates a TOML config file that TrustTunnel consumes:

```toml
vpn_mode = "general"
loglevel = "trace"
exclusions = ["*.example.com", "10.0.0.1"]    # bypass list

[endpoint]
hostname = "vpn.example.com"
addresses = ["1.2.3.4:443"]
username = "user"
password = "pass"
upstream_protocol = "http2"
dns_upstreams = ["9.9.9.9"]

[listener.tun]
included_routes = ["0.0.0.0/0"]
excluded_routes = ["100.64.0.0/10", "1.2.3.4/32"]
mtu_size = 1280
```

See [ADR-0001](decisions/0001-deprecate-secular-core.md) for why we moved from a custom Rust core to TrustTunnel's native libraries.
