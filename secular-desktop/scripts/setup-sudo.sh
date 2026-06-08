#!/bin/bash
# Configure passwordless sudo for Secular VPN desktop client
# Safe to run via: curl -fsSL ... | bash

set -euo pipefail

SUDOERS_FILE="/etc/sudoers.d/secular"

echo "=== Secular Sudo Setup ==="
echo ""
echo "Secular needs root to create a virtual network interface (utun)."
echo "This script adds passwordless sudo for the trusttunnel_client binary."
echo ""

# Find the bundled trusttunnel_client binary inside Secular.app
APP_CLI="/Applications/Secular.app/Contents/Resources/binaries/trusttunnel_client"

if [ ! -f "$APP_CLI" ]; then
    echo "ERROR: trusttunnel_client not found at $APP_CLI"
    echo "Make sure Secular.app is installed to /Applications first."
    exit 1
fi

echo "Found: $APP_CLI"
echo ""

# Determine the real user (works when already root or called via sudo)
REAL_USER="${SUDO_USER:-${USER:-$(id -un)}}"
echo "Configuring sudo for user: $REAL_USER"
echo ""

# Write sudoers file via tee (avoids any shell quoting/newline issues)
SUDOERS_LINE="$REAL_USER ALL=(ALL) NOPASSWD: $APP_CLI"

echo "Need sudo to write $SUDOERS_FILE (you will be prompted for your password):"
printf '# Secular VPN — passwordless sudo for the bundled trusttunnel_client\n%s\n' "$SUDOERS_LINE" \
    | sudo tee "$SUDOERS_FILE" > /dev/null
sudo chmod 440 "$SUDOERS_FILE"

# Verify syntax
if sudo visudo -c -f "$SUDOERS_FILE" >/dev/null 2>&1; then
    echo "✓ Done! $REAL_USER can now run trusttunnel_client without a password."
    echo ""
    echo "To verify:"
    echo "  sudo -n $APP_CLI --version"
else
    echo "ERROR: sudoers syntax error. Removing file."
    sudo rm -f "$SUDOERS_FILE"
    exit 1
fi
