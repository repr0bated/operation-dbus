#!/bin/bash
# download-pve9.sh - Download Proxmox VE 9 ISO (Trixie-based)
set -euo pipefail

DOWNLOAD_DIR="${1:-./iso}"
PVE_VERSION="9.0-1"
ISO_URL="https://enterprise.proxmox.com/iso/proxmox-ve_${PVE_VERSION}.iso"
ISO_NAME="proxmox-ve_${PVE_VERSION}.iso"
SHA256_URL="${ISO_URL}.sha256sum"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE 9 ISO Downloader"
echo "  Version: ${PVE_VERSION}"
echo "  Based on: Debian Trixie (13)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Create download directory
mkdir -p "$DOWNLOAD_DIR"
cd "$DOWNLOAD_DIR"

# Check if ISO already exists
if [ -f "$ISO_NAME" ]; then
    echo "✓ ISO already exists: $ISO_NAME"
    echo ""
    echo "To re-download, delete the file first:"
    echo "  rm $DOWNLOAD_DIR/$ISO_NAME"
    echo ""
    echo "Skipping download."
    exit 0
fi

echo "📥 Downloading Proxmox VE 9 ISO..."
echo "URL: $ISO_URL"
echo ""

# Download ISO
wget --progress=bar:force \
    --show-progress \
    -O "$ISO_NAME" \
    "$ISO_URL"

echo ""
echo "📝 Downloading SHA256 checksum..."
wget -q -O "${ISO_NAME}.sha256sum" "$SHA256_URL"

echo ""
echo "🔐 Verifying checksum..."

# Extract just the hash from the checksum file (format: "hash  filename")
EXPECTED_SHA256=$(awk '{print $1}' "${ISO_NAME}.sha256sum")
ACTUAL_SHA256=$(sha256sum "$ISO_NAME" | awk '{print $1}')

if [ "$EXPECTED_SHA256" = "$ACTUAL_SHA256" ]; then
    echo "✓ Checksum verified!"
else
    echo "✗ Checksum MISMATCH!"
    echo "  Expected: $EXPECTED_SHA256"
    echo "  Actual:   $ACTUAL_SHA256"
    echo ""
    echo "The downloaded file may be corrupted. Please try again."
    exit 1
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Download complete!"
echo ""
echo "ISO: $(pwd)/$ISO_NAME"
echo "Size: $(du -h "$ISO_NAME" | cut -f1)"
echo ""
echo "Next steps:"
echo "  1. Extract packages from ISO:"
echo "     ./extract-iso.sh $(pwd)/$ISO_NAME"
echo ""
echo "  2. Or use the complete pipeline:"
echo "     ./quick-start.sh $(pwd)/$ISO_NAME"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
