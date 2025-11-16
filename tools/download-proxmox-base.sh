#!/bin/bash
# download-proxmox-base.sh - Download Proxmox base image from GitHub Releases

set -euo pipefail

VERSION="${1:-v1.0-proxmox}"
RELEASE_URL="https://github.com/repr0bated/operation-dbus/releases/download/$VERSION/proxmox-opdbus.tar.gz"
OUTPUT_DIR="deploy"
ARCHIVE_NAME="proxmox-opdbus.tar.gz"

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

# Extract
echo "Extracting archive..."
cd "$OUTPUT_DIR"
tar xzf "$ARCHIVE_NAME"

echo ""
echo "✓ Extraction complete!"
echo ""
echo "Files:"
ls -lh vm-100-disk-1.raw 2>/dev/null || ls -lh
echo ""
echo "Next step: Run ./tools/deploy-proxmox-base.sh /dev/sda"
