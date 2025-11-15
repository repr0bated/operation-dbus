#!/bin/bash
# install-netboot-xyz.sh - Install netboot.xyz to ESP with systemd-boot

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

# Create directories
mkdir -p "$ESP_MOUNT/netboot.xyz"
mkdir -p "$ESP_MOUNT/loader/entries"

# Download netboot.xyz.efi
echo "Downloading netboot.xyz.efi..."
wget -O "$ESP_MOUNT/netboot.xyz/netboot.xyz.efi" \
    https://boot.netboot.xyz/ipxe/netboot.xyz.efi

echo "✓ Downloaded netboot.xyz.efi"

# Copy boot entry
cp boot/netboot.xyz/netboot.xyz.conf "$ESP_MOUNT/loader/entries/"

echo "✓ Boot entry installed"

# Update loader.conf
LOADER_CONF="$ESP_MOUNT/loader/loader.conf"

cat > "$LOADER_CONF" <<EOF
default  netboot.xyz.conf
timeout  5
console-mode max
editor   no
EOF

echo "✓ Updated loader.conf (netboot.xyz as default)"

echo ""
echo "✅ netboot.xyz installed!"
echo ""
echo "  Boot entry: $ESP_MOUNT/loader/entries/netboot.xyz.conf"
echo "  EFI binary: $ESP_MOUNT/netboot.xyz/netboot.xyz.efi"
echo "  Default: netboot.xyz"
echo ""
echo "Reboot to use netboot.xyz"
