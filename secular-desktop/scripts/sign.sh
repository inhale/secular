#!/bin/bash
# Ad-hoc codesign the app bundle after Tauri builds it
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ARCH="${TAURI_ENV_ARCH:-aarch64}"

if [ "$ARCH" = "x86_64" ]; then
    APP_PATH="$PROJECT_ROOT/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/Secular.app"
else
    APP_PATH="$PROJECT_ROOT/src-tauri/target/release/bundle/macos/Secular.app"
fi

if [ ! -d "$APP_PATH" ]; then
    echo "[SIGN] ERROR: App bundle not found at $APP_PATH"
    exit 1
fi

echo "[SIGN] Ad-hoc signing: $APP_PATH (arch: $ARCH)"

# Sign inner binary first
BINARY_PATH="$APP_PATH/Contents/Resources/binaries/trusttunnel_client"
if [ -f "$BINARY_PATH" ]; then
    echo "[SIGN] Signing trusttunnel_client"
    codesign --force --sign - "$BINARY_PATH"
fi

# Ad-hoc sign the entire app bundle
codesign --force --deep --sign - --timestamp=none "$APP_PATH"

echo "[SIGN] Done. Verifying..."
codesign --verify --deep --strict "$APP_PATH" 2>&1 || true
