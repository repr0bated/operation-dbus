#!/bin/bash
# install-netboot-xyz.sh - Install netboot.xyz to ESP with GRUB

set -euo pipefail

ESP_MOUNT="${1:-/boot/efi}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  netboot.xyz Installation"
echo "  ESP: $ESP_MOUNT"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Verify ESP is mounted
if ! mountpoint -q "$ESP_MOUNT"; then
    echo "Error: ESP not mounted at $ESP_MOUNT"
    exit 1
fi

# Create directory
mkdir -p "$ESP_MOUNT/netboot.xyz"

# Download netboot.xyz.efi
echo "Downloading netboot.xyz.efi..."
wget -O "$ESP_MOUNT/netboot.xyz/netboot.xyz.efi" \
    https://boot.netboot.xyz/ipxe/netboot.xyz.efi

echo "✓ Downloaded netboot.xyz.efi"

# Add to GRUB config if exists
GRUB_CFG="$ESP_MOUNT/grub/grub.cfg"

if [ -f "$GRUB_CFG" ]; then
    # Check if netboot.xyz entry already exists
    if ! grep -q "netboot.xyz" "$GRUB_CFG"; then
        echo "" >> "$GRUB_CFG"
        echo 'menuentry "netboot.xyz" {' >> "$GRUB_CFG"
        echo '    chainloader /netboot.xyz/netboot.xyz.efi' >> "$GRUB_CFG"
        echo '}' >> "$GRUB_CFG"
        echo "✓ Added netboot.xyz to GRUB menu"
    else
        echo "✓ netboot.xyz already in GRUB menu"
    fi
else
    echo "⚠️  GRUB config not found at $GRUB_CFG"
    echo "   netboot.xyz binary installed but not added to boot menu"
fi

echo ""
echo "✅ netboot.xyz installed!"
echo ""
echo "  EFI binary: $ESP_MOUNT/netboot.xyz/netboot.xyz.efi"
if [ -f "$GRUB_CFG" ]; then
    echo "  GRUB entry: Added to $GRUB_CFG"
fi
echo ""
echo "Reboot to use netboot.xyz"
