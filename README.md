# Secular

> Digital freedom. Unblockable network access.

**Secular** is a cross-platform VPN client built for censorship resistance. It wraps all traffic in obfuscated HTTP/2 and QUIC streams that are indistinguishable from normal web traffic — powered by TrustTunnel protocol with native clients on macOS, Android, and Windows (in development).

> ⚠️ **Architecture Change (June 2026):** We've deprecated the `secular-core` Rust implementation in favor of TrustTunnel's official C++ libraries. This allows us to ship faster and focus on best-in-class split tunneling features. See [ADR-0001](docs/architecture/decisions/0001-deprecate-secular-core.md) for details.

## Design System

Secular uses a light, minimalist design inspired by the paragraph sign (§).

| Token | Value | Usage |
|---|---|---|
| Background | `#F5F7FA` | App background |
| Surface | `#FFFFFF` | Cards, tiles |
| Text Primary | `#242424` | Headings, body text |
| Text Secondary | `#7A869A` | Labels, hints |
| Accent (Connect) | `#d02b57` | Connect button, active states |
| Accent (Info) | `#147cc4` | Info badges, links |
| Accent (Warn) | `#deb052` | Warning states |
| Accent (Success) | `#00F5D4` | Connected indicator |
| Alert | `#FF3B30` | Disconnect button, errors |

**Logo:** Two interlocking S-shaped waves forming an 'S' through negative space, with accent dots top and bottom.

**Window:** 360×520px compact fixed window (desktop), full-screen mobile.

## Supported Platforms

| Platform | Status | Format | Notes |
|---|---|---|---|
| macOS (Apple Silicon) | ✅ Stable | `.dmg` | TrustTunnel CLI subprocess |
| Android | ✅ Stable | `.apk` | TrustTunnel AAR library |
| Windows | 🚧 In Development | `.msi` / `.exe` | See [Issue #5](https://github.com/inhale/secular/issues/5) |
| iOS | ❌ Blocked | `.ipa` | Requires Apple Developer account ($99/year) |

**Focus:** Best-in-class app-level split tunneling (per-process routing)

## Monorepo Structure

```
├── legacy/secular-core/    # DEPRECATED: Rust protocol implementation (archived)
├── secular-desktop/        # Tauri v2 desktop app (macOS, Windows)
│   ├── src-tauri/          # Rust backend (TrustTunnel CLI integration)
│   └── src/                # React/TypeScript frontend
├── secular-android/        # Android (Kotlin + TrustTunnel AAR)
│   └── app/src/main/kotlin/
├── docs/                   # Architecture, decisions, guides
│   ├── architecture/
│   │   ├── decisions/      # ADRs (Architecture Decision Records)
│   │   └── overview.md
│   └── features/
└── .github/workflows/      # CI/CD
```

## Building Locally

### Desktop (macOS, Windows)
```bash
cd secular-desktop
npm install
npm run tauri dev          # Development
npm run tauri build        # Release
```

### Android
```bash
cd secular-android
./gradlew assembleDebug    # Development
./gradlew assembleRelease  # Release (requires keystore)
```

See [docs/](docs/) for platform-specific build guides.

## Philosophy

Secular exists because access to information is a fundamental right. Not a privilege.

We wrap payloads in traffic that mimics standard HTTPS/QUIC — not to hide that you're using a VPN, but to make it *impossible to distinguish* from normal browsing. This is what makes it unblockable.

## License

See `LICENSE` file.
