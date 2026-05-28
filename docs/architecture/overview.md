# Secular — Architecture

## Overview

Secular uses a **Rust core library** (`secular-core`) as the single source of truth for all protocol logic. Every platform (desktop, mobile) links against this core via FFI.

```
┌─────────────────────────────────────────────────────────┐
│                    Platform Clients                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │  Tauri   │ │  iOS     │ │ Android  │ │  CLI      │  │
│  │  Desktop │ │  Swift   │ │  Kotlin  │ │  (future) │  │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └─────┬─────┘  │
│       │             │            │              │        │
│       └─────────────┼────────────┼──────────────┘        │
│                     │  FFI / UniFFI                      │
│              ┌──────┴──────┐                             │
│              │ secular-core│                             │
│              │   (Rust)    │                             │
│              └──────┬──────┘                             │
│                     │                                    │
│       ┌─────────────┼─────────────┐                     │
│       │             │             │                      │
│  ┌────┴────┐  ┌─────┴────┐ ┌─────┴────┐                │
│  │ Protocol│  │   DNS    │ │  Network │                │
│  │ Engine  │  │  Leak    │ │  (TUN)   │                │
│  │         │  │  Guard   │ │          │                │
│  └─────────┘  └──────────┘ └──────────┘                │
└─────────────────────────────────────────────────────────┘
```

## Protocol

Secular wraps all traffic in HTTP/2 or QUIC streams that mimic standard web traffic:

1. **Handshake:** Client connects to server with TLS, authenticates via token
2. **Obfuscation:** All packets are wrapped in HTTP/2 DATA frames or QUIC STREAM frames
3. **uTLS:** ClientHello fingerprints are randomized to avoid TLS fingerprinting
4. **DNS:** All DNS queries are routed through the tunnel; port-53 is hijacked to prevent leaks
5. **MTU:** Dynamic MTU clamping prevents fragmentation-based detection

## Platform Integration

| Platform | Language | FFI Method | Key Component |
|----------|----------|------------|---------------|
| macOS/Windows/Linux | Rust (Tauri) | Direct crate link | `src-tauri/` |
| iOS | Swift | UniFFI | `NEPacketTunnelProvider` |
| Android | Kotlin | UniFFI | `VpnService` |

## DNS Leak Prevention

1. Intercept all port-53 UDP/TCP traffic
2. Redirect through tunnel's DNS resolver
3. Block all non-tunnel DNS (DoH/DoT endpoint IPs via firewall rules)
4. IPv6 is blackholed by default to prevent IPv6 leaks

## Kill Switch

- **Windows:** WFP (Windows Filtering Platform) — block all non-tunnel traffic
- **Linux:** NFTables — drop all traffic not via tunnel interface
- **macOS:** PF firewall — anchor rules for tunnel-only traffic
