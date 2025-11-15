#!/bin/bash
# first-stage-install.sh - Complete first-stage Proxmox installation via PackageKit
#
# This script bootstraps a minimal Debian system and uses PackageKit to install
# Proxmox VE from the extracted manifest.
#
# Usage: ./first-stage-install.sh /dev/sda proxmox-ve_9.0-1.iso

set -euo pipefail

DEVICE="${1:-}"
ISO_PATH="${2:-}"

if [ -z "$DEVICE" ] || [ -z "$ISO_PATH" ]; then
    echo "Usage: $0 <device> <proxmox-iso>"
    echo ""
    echo "Example:"
    echo "  ./first-stage-install.sh /dev/sda proxmox-ve_9.0-1.iso"
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE First-Stage Installation"
echo "  Device: $DEVICE"
echo "  ISO: $ISO_PATH"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# ============================================================================
# PHASE 1: Partition and Format
# ============================================================================

echo "PHASE 1: Partitioning $DEVICE..."

# Create partitions
parted -s "$DEVICE" mklabel gpt
parted -s "$DEVICE" mkpart primary fat32 1MiB 513MiB
parted -s "$DEVICE" set 1 esp on
parted -s "$DEVICE" mkpart primary ext4 513MiB 100%

# Format partitions
mkfs.vfat -F32 "${DEVICE}1"  # EFI
mkfs.ext4 -F "${DEVICE}2"     # Root

echo "✓ Partitions created"

# ============================================================================
# PHASE 2: Mount and Debootstrap
# ============================================================================

echo ""
echo "PHASE 2: Installing base Debian system..."

MOUNT_POINT="/mnt/proxmox-install"
mkdir -p "$MOUNT_POINT"

mount "${DEVICE}2" "$MOUNT_POINT"
mkdir -p "$MOUNT_POINT/boot/efi"
mount "${DEVICE}1" "$MOUNT_POINT/boot/efi"

# Install minimal Debian base
debootstrap --arch=amd64 bookworm "$MOUNT_POINT" http://deb.debian.org/debian

echo "✓ Base system installed"

# ============================================================================
# PHASE 3: Configure System
# ============================================================================

echo ""
echo "PHASE 3: Configuring base system..."

# Mount proc/sys/dev
mount --bind /proc "$MOUNT_POINT/proc"
mount --bind /sys "$MOUNT_POINT/sys"
mount --bind /dev "$MOUNT_POINT/dev"
mount --bind /dev/pts "$MOUNT_POINT/dev/pts"

# Configure fstab
cat > "$MOUNT_POINT/etc/fstab" <<EOF
${DEVICE}2  /           ext4    errors=remount-ro  0  1
${DEVICE}1  /boot/efi   vfat    defaults           0  2
EOF

# Set hostname
echo "proxmox" > "$MOUNT_POINT/etc/hostname"

# Configure network
cat > "$MOUNT_POINT/etc/network/interfaces" <<EOF
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

echo "✓ System configured"

# ============================================================================
# PHASE 4: Install PackageKit and Prerequisites
# ============================================================================

echo ""
echo "PHASE 4: Installing PackageKit..."

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
# Update package lists
apt-get update

# Install essential packages
apt-get install -y \
    packagekit \
    packagekit-tools \
    systemd-sysv \
    grub-efi-amd64 \
    linux-image-amd64

# Install Rust (for proxmox-packagekit binary)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Enable PackageKit
systemctl enable packagekit
CHROOT_EOF

echo "✓ PackageKit installed"

# ============================================================================
# PHASE 5: Add Proxmox Repositories
# ============================================================================

echo ""
echo "PHASE 5: Configuring Proxmox repositories..."

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
# Add Proxmox VE repository
echo "deb http://download.proxmox.com/debian/pve bookworm pve-no-subscription" \
    > /etc/apt/sources.list.d/pve-no-subscription.list

# Add Proxmox VE GPG key
wget https://enterprise.proxmox.com/debian/proxmox-release-bookworm.gpg \
    -O /etc/apt/trusted.gpg.d/proxmox-release-bookworm.gpg

# Update package lists
apt-get update
CHROOT_EOF

echo "✓ Proxmox repositories configured"

# ============================================================================
# PHASE 6: Extract Proxmox Packages from ISO
# ============================================================================

echo ""
echo "PHASE 6: Extracting Proxmox packages from ISO..."

# Copy extraction tools to chroot
cp -r tools/proxmox-extractor "$MOUNT_POINT/root/"
cp "$ISO_PATH" "$MOUNT_POINT/root/proxmox.iso"

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
cd /root/proxmox-extractor

# Extract package list from ISO
./extract-iso.sh /root/proxmox.iso ./extracted

# Generate manifest
cargo run --release --bin proxmox-manifest -- \
    --packages ./extracted/packages/Packages.txt \
    --output ./manifest.json

echo "✓ Manifest generated"
CHROOT_EOF

# ============================================================================
# PHASE 7: Install Proxmox via PackageKit
# ============================================================================

echo ""
echo "PHASE 7: Installing Proxmox packages via PackageKit..."

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
cd /root/proxmox-extractor

# Start PackageKit
systemctl start packagekit

# Install via PackageKit D-Bus
cargo run --release --bin proxmox-packagekit -- \
    ./manifest.json \
    --log-file /var/log/proxmox-packagekit-install.log

echo "✓ Proxmox installed via PackageKit"
CHROOT_EOF

# ============================================================================
# PHASE 8: Install Bootloader
# ============================================================================

echo ""
echo "PHASE 8: Installing bootloader..."

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
# Install GRUB
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=proxmox

# Update GRUB configuration
update-grub

echo "✓ Bootloader installed"
CHROOT_EOF

# ============================================================================
# PHASE 9: Install operation-dbus
# ============================================================================

echo ""
echo "PHASE 9: Installing operation-dbus..."

# Copy operation-dbus source to chroot
cp -r . "$MOUNT_POINT/root/operation-dbus"

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
cd /root/operation-dbus

# Build operation-dbus
cargo build --release --all-features

# Install binaries
cp target/release/op-dbus /usr/local/bin/
cp target/release/dbus-mcp /usr/local/bin/
cp target/release/dbus-mcp-web /usr/local/bin/

# Create systemd service
cat > /etc/systemd/system/op-dbus.service <<EOF
[Unit]
Description=Operation D-Bus - Declarative System Management
After=network.target dbus.service

[Service]
Type=simple
ExecStart=/usr/local/bin/op-dbus
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Enable op-dbus
systemctl enable op-dbus

echo "✓ operation-dbus installed"
CHROOT_EOF

# ============================================================================
# PHASE 10: Cleanup and Finalize
# ============================================================================

echo ""
echo "PHASE 10: Finalizing installation..."

# Unmount everything
umount "$MOUNT_POINT/dev/pts"
umount "$MOUNT_POINT/dev"
umount "$MOUNT_POINT/sys"
umount "$MOUNT_POINT/proc"
umount "$MOUNT_POINT/boot/efi"
umount "$MOUNT_POINT"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ INSTALLATION COMPLETE!"
echo ""
echo "System installed to: $DEVICE"
echo ""
echo "Next steps:"
echo "  1. Reboot the system"
echo "  2. Proxmox VE will be running"
echo "  3. operation-dbus is installed and enabled"
echo "  4. Access web UI: https://<ip>:8006"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
