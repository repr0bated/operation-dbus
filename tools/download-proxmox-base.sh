#!/bin/bash
# download-proxmox-base.sh - Download Proxmox base image from GitHub Releases

set -euo pipefail

VERSION="${1:-proxmox-base}"
RELEASE_URL="https://github.com/repr0bated/operation-dbus/releases/download/$VERSION/proxmox-op-dbus.tar.gz"
OUTPUT_DIR="deploy"
ARCHIVE_NAME="proxmox-op-dbus.tar.gz"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox Base Image Download"
echo "  Version: $VERSION"
echo "  Output: $OUTPUT_DIR/"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Create deploy directory
mkdir -p "$OUTPUT_DIR"

# Download from GitHub Releases
echo "Downloading Proxmox base image (1.4GB, this may take a while)..."
wget -O "$OUTPUT_DIR/$ARCHIVE_NAME" "$RELEASE_URL" || \
    curl -L -o "$OUTPUT_DIR/$ARCHIVE_NAME" "$RELEASE_URL"

echo ""
echo "✓ Downloaded to $OUTPUT_DIR/$ARCHIVE_NAME"
echo ""

echo ""
echo "✓ Download complete!"
echo ""
echo "Archive: $OUTPUT_DIR/$ARCHIVE_NAME ($(du -h "$OUTPUT_DIR/$ARCHIVE_NAME" | cut -f1))"
echo ""
echo "Next step: Run ./tools/deploy-proxmox-base.sh /dev/sda"
echo "           (extraction will happen directly to target device)"
