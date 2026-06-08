#!/bin/bash
# Build and ad-hoc sign Secular for macOS
# Usage: ./scripts/build.sh [aarch64|x86_64|all]

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_ROOT"

TARGET="${1:-all}"

sign_app() {
    local arch="$1"
    local app_path="$2"

    if [ ! -d "$app_path" ]; then
        echo "[SIGN] WARNING: $app_path not found, skipping"
        return
    fi

    echo "[SIGN] Signing $arch: $app_path"

    # Sign inner binary first
    local binary_path="$app_path/Contents/Resources/binaries/trusttunnel_client"
    if [ -f "$binary_path" ]; then
        codesign --force --sign - "$binary_path"
    fi

    # Sign the app bundle
    codesign --force --deep --sign - --timestamp=none "$app_path"

    # Also sign the DMG
    local dmg_path
    if [ "$arch" = "x86_64" ]; then
        dmg_path="$PROJECT_ROOT/src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/Secular_0.1.0_x64.dmg"
    else
        dmg_path="$PROJECT_ROOT/src-tauri/target/release/bundle/dmg/Secular_0.1.0_aarch64.dmg"
    fi

    if [ -f "$dmg_path" ]; then
        codesign --force --sign - --timestamp=none "$dmg_path"
        echo "[SIGN] Signed DMG: $dmg_path"
    fi

    echo "[SIGN] Done for $arch"
    codesign --verify --deep --strict "$app_path" 2>&1 || true
}

build_arch() {
    local arch="$1"
    echo ""
    echo "===== Building $arch ====="

    if [ "$arch" = "x86_64" ]; then
        PATH="/Users/inhale/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH" npm run tauri build -- --target x86_64-apple-darwin
    else
        PATH="/Users/inhale/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH" npm run tauri build
    fi
}

if [ "$TARGET" = "all" ] || [ "$TARGET" = "aarch64" ]; then
    build_arch "aarch64"
    sign_app "aarch64" "$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Secular.app"
fi

if [ "$TARGET" = "all" ] || [ "$TARGET" = "x86_64" ]; then
    build_arch "x86_64"
    sign_app "x86_64" "$PROJECT_ROOT/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Secular.app"
fi

echo ""
echo "===== Build complete ====="
ls -la "$PROJECT_ROOT/src-tauri/target/release/bundle/dmg/" 2>/dev/null
ls -la "$PROJECT_ROOT/src-tauri/target/x86_64-apple-darwin/release/bundle/dmg/" 2>/dev/null
