# secular-core (DEPRECATED)

**Status:** Archived as of June 8, 2026

This Rust protocol implementation has been deprecated in favor of TrustTunnel's official C++ libraries.

## Decision Rationale

We made this decision to:

1. **Ship faster** — Windows in 3-5 days (vs 15-20 with secular-core)
2. **Leverage battle-tested code** — AdGuard VPN's 50M+ users
3. **Focus on differentiation** — Invest in split tunneling features
4. **Avoid protocol maintenance** — Security updates, endpoint compatibility
5. **Work within constraints** — No Apple Developer account for iOS

## What Was Implemented

This library contained a working implementation of:

- ✅ TLS handshake with uTLS fingerprinting (Chrome/Firefox/Safari)
- ✅ HTTP/2 tunnel establishment
- ✅ QUIC support
- ✅ Packet routing and TUN interface management
- ✅ DNS leak prevention
- ✅ MTU clamping
- ✅ UniFFI bindings for iOS/Android FFI

Total: ~1,500 lines of Rust code implementing the TrustTunnel protocol (~80% complete).

## Why We Archived It

### Cons that led to deprecation:
- Protocol maintenance burden (security updates, endpoint compatibility)
- 15-20 days extra work for iOS Network Extension in Rust
- No commercial need to own the stack (we're open-source)
- TrustTunnel is already battle-tested and maintained by AdGuard
- No Apple Developer account ($99/year) blocks iOS anyway

### What we gained by deprecating:
- Faster time-to-market (Windows in 3-5 days, iOS in 10-14 days)
- More time to invest in split tunneling features (our differentiation)
- Leverage AdGuard's active maintenance and security updates
- Better platform integration (native WFP on Windows, VpnService on Android)

## Could This Be Revived?

**Yes!** If Secular VPN becomes a commercial product and needs:

1. Custom protocol extensions (beyond standard TrustTunnel)
2. Full stack ownership for product differentiation
3. Smaller binary sizes (Rust vs C++)
4. Unified codebase maintenance (one language)

...then this code provides a solid foundation (~80% complete protocol implementation).

## See Also

- [Architecture Decision Record](../../docs/architecture/decisions/0001-deprecate-secular-core.md)
- [TrustTunnel Protocol Spec](https://github.com/TrustTunnel/TrustTunnel/blob/master/PROTOCOL.md)
- [Current Architecture](../../docs/architecture/overview.md)
