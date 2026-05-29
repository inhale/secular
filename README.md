# Secular

> Digital freedom. Unblockable network access.

**Secular** is a cross-platform VPN client built for censorship resistance. It wraps all traffic in obfuscated HTTP/2 and QUIC streams that are indistinguishable from normal web traffic — powered by a Rust core with native clients on macOS, Windows, Linux, iOS, and Android.

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

| Platform | Format | CI Status |
|---|---|---|
| macOS (universal2) | `.dmg` | Built on tag |
| Windows | `.msi` / `.exe` | Built on tag |
| Linux | `.AppImage` / `.deb` | Built on tag |
| iOS | `.ipa` | Built on tag |
| Android | `.apk` | Built on tag |

All builds run on **GitHub Actions** — free for private repos (2,000 min/month).

## Monorepo Structure

```
├── secular-core/        # Rust FFI library (protocol, crypto, DNS, MTU, uTLS)
│   ├── include/         # C headers for FFI
│   ├── src/
│   │   ├── protocol.rs  # Handshake, HTTP/2 + QUIC obfuscation
│   │   ├── dns.rs       # DNS leak prevention, port-53 hijacking
│   │   ├── mtu.rs       # Dynamic MTU clamping
│   │   ├── utls.rs      # uTLS randomized ClientHello fingerprinting
│   │   ├── network.rs   # Packet processing, TUN interface
│   │   ├── config.rs    # Configuration loader
│   │   ├── ffi.rs       # UniFFI export macros
│   │   └── lib.rs       # Library entry point
│   └── Cargo.toml
├── secular-desktop/     # Tauri v2 desktop app
│   ├── src-tauri/       # Rust backend + tray
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── commands.rs
│   │   │   └── tray.rs
│   │   ├── tauri.conf.json
│   │   └── entitlements.plist
│   └── src/             # React/TypeScript frontend
│       ├── App.tsx
│       └── style.css
├── secular-android/     # Android (Kotlin VpnService)
│   ├── app/
│   │   ├── src/main/
│   │   │   ├── kotlin/com/secular/vpn/
│   │   │   │   ├── SecularVpnService.kt
│   │   │   │   └── MainActivity.kt
│   │   │   ├── res/
│   │   │   └── AndroidManifest.xml
│   │   └── build.gradle.kts
│   └── build.gradle.kts
├── secular-ios/         # iOS (Swift NetworkExtension)
│   ├── Secular/
│   │   ├── SecularApp.swift
│   │   ├── ContentView.swift
│   │   ├── Info.plist
│   │   └── Entitlements.plist
│   └── Secular/Extensions/
│       ├── PacketTunnelProvider.swift
│       ├── Info.plist
│       └── Entitlements.plist
├── assets/              # Logo & brand assets (SVG, PNG)
│   └── logo/
├── .github/workflows/   # CI/CD
│   ├── ci.yml           # Test + lint on every push/PR
│   └── release.yml      # Build all 5 platforms on tag
└── docs/                # Architecture, API, design specs
```

## Building Locally

### Rust Core (all platforms)
```bash
cd secular-core
cargo build --all-features
cargo test --all-features
```

### Desktop (requires Tauri prerequisites)
```bash
cd secular-desktop
npm install
npm run tauri dev          # Development
npm run tauri build        # Release
```

### macOS Universal2
```bash
cd secular-core
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
lipo -create target/aarch64-apple-darwin/release/libsecular_core.a \
             target/x86_64-apple-darwin/release/libsecular_core.a \
             -output target/universal/libsecular_core.a
```

### Android (requires NDK)
```bash
cd secular-desktop
npx tauri android init
npx tauri android build --debug
```

### iOS (requires Xcode + Apple Developer account)
```bash
cd secular-desktop
npx tauri ios init
npx tauri ios build --debug
```

## CI/CD via GitHub Actions

Both CI and release are fully automated:

- **CI** (`ci.yml`): Runs on every push to `main` and every PR — tests Rust core, lints all code, checks mobile project structure
- **Release** (`release.yml`): Runs on every `v*` tag push — builds all 5 platforms and creates a GitHub Release with all artifacts

To trigger a release:
```bash
git tag v0.1.0
git push origin v0.1.0
```

This produces:
- `secular-macos.dmg` (universal2)
- `secular-windows.msi`
- `secular-linux.AppImage`
- `secular-ios.ipa`
- `secular-android.apk`

## Philosophy

Secular exists because access to information is a fundamental right. Not a privilege.

We wrap payloads in traffic that mimics standard HTTPS/QUIC — not to hide that you're using a VPN, but to make it *impossible to distinguish* from normal browsing. This is what makes it unblockable.

## License

See `LICENSE` file.
