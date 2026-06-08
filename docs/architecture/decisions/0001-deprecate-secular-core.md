# ADR 0001: Deprecate secular-core in Favor of TrustTunnel Libraries

**Status:** Accepted  
**Date:** 2026-06-08  
**Deciders:** @inhale

## Context

We started building `secular-core`, a Rust implementation of the TrustTunnel protocol, with the goal of:

1. Unified codebase across all platforms (macOS, Windows, iOS, Android)
2. Full control over protocol features and performance
3. Smaller binary sizes compared to C++
4. Modern async/await patterns via Tokio

We successfully implemented ~1,500 lines of Rust code covering:
- TLS handshake with uTLS fingerprinting
- HTTP/2 and QUIC tunnel establishment
- Packet routing and TUN interface management
- DNS leak prevention and MTU clamping
- UniFFI bindings for cross-platform FFI

However, we faced a critical constraint: **no Apple Developer account** ($99/year), blocking iOS distribution. Additionally, as an open-source project, we lack the resources to maintain protocol compatibility and security updates long-term.

## Decision

**We will deprecate `secular-core` and use TrustTunnel's official libraries:**

- **macOS/Windows:** TrustTunnel CLI subprocess (current macOS approach)
- **Android:** TrustTunnel AAR (already implemented)
- **iOS:** Blocked (no Apple Developer account) — community contributions welcome

The `secular-core` code will be archived in `legacy/` for future reference.

## Rationale

### Why TrustTunnel Libraries?

**Pros:**
1. ✅ **Battle-tested** — Used by AdGuard VPN's 50M+ users
2. ✅ **Actively maintained** — Security updates, endpoint compatibility, performance
3. ✅ **Faster time-to-market** — Windows in 3-5 days (vs 15-20 with secular-core)
4. ✅ **Better split tunneling support** — Leverage native WFP (Windows), VpnService (Android)
5. ✅ **No protocol maintenance burden** — We don't need to track TrustTunnel endpoint updates
6. ✅ **Proven iOS solution** — TrustTunnel Flutter app is App Store approved

**Cons:**
1. ❌ No unified Rust codebase (C++ on mobile, CLI subprocess on desktop)
2. ❌ Dependent on AdGuard's roadmap (can't easily add custom features)
3. ❌ Larger binary sizes (C++ stdlib + TrustTunnel libs)
4. ❌ Loss of our Rust implementation work

### Why Not Keep secular-core?

**If this were a commercial product:**
- We'd invest 20-30 days to finish iOS Network Extension in Rust
- We'd maintain protocol compatibility ourselves
- We'd own the full stack for product differentiation

**But as an open-source project:**
- We lack resources for long-term protocol maintenance
- We can't distribute iOS without a paid Apple Developer account anyway
- We'd rather invest time in **split tunneling features** (our differentiation)

## Consequences

### Positive:
- ✅ Ship Windows in 3-5 days (90% code reuse from macOS)
- ✅ Focus on split tunneling UI (Android: 2-3 days, Windows: 5-7 days)
- ✅ Leverage AdGuard's security updates automatically
- ✅ Better platform integration (native APIs vs Rust FFI)

### Negative:
- ❌ No unified Rust codebase (desktop uses CLI subprocess, mobile uses C++ libs)
- ❌ Dependent on TrustTunnel's maintenance (but they have strong incentives)
- ❌ Can't easily add custom protocol features (but we don't need them)

### Neutral:
- 📦 Code is preserved in `legacy/secular-core/` for potential future revival
- 📝 Could be revived if project becomes commercial and needs stack ownership

## Alternatives Considered

### Alternative 1: Finish secular-core (Rust everywhere)
- **Timeline:** 20-30 days for iOS Network Extension
- **Outcome:** Unified Rust codebase, full control
- **Why rejected:** No Apple Developer account blocks iOS anyway, too slow for open-source project

### Alternative 2: Hybrid (TrustTunnel for mobile, secular-core for desktop)
- **Timeline:** 10-15 days to migrate macOS/Windows from CLI → secular-core FFI
- **Outcome:** Rust on desktop, C++ on mobile (split codebase)
- **Why rejected:** No clear benefit vs just using TrustTunnel CLI everywhere

### Alternative 3: Keep secular-core for iOS only (when we get Apple account)
- **Timeline:** 20-30 days when account is available
- **Outcome:** Rust on iOS, C++/CLI on other platforms
- **Why rejected:** Most complex option, least code reuse

## Reversal Criteria

We would revive `secular-core` if:

1. **Project becomes commercial** → Need stack ownership for differentiation
2. **Apple Developer account acquired** → iOS becomes viable, Rust FFI attractive
3. **TrustTunnel abandoned** → Need to maintain protocol ourselves
4. **Custom protocol features needed** → Extensions beyond standard TrustTunnel

The code is preserved in `legacy/` to make revival feasible.

## References

- [TrustTunnel GitHub](https://github.com/TrustTunnel/TrustTunnel)
- [TrustTunnel Protocol Spec](https://github.com/TrustTunnel/TrustTunnel/blob/master/PROTOCOL.md)
- [TrustTunnel Client Libraries](https://github.com/TrustTunnel/TrustTunnelClient)
- [secular-core Implementation](../../legacy/secular-core/)
