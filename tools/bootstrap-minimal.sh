#!/bin/bash
# bootstrap-minimal.sh - Create minimal Debian base for op-dbus
#
# This creates the MINIMUM system needed to run `op-dbus apply`:
#   - Partitioned disk
#   - Minimal Debian (debootstrap)
#   - op-dbus binary installed
#   - PackageKit installed
#
# Then op-dbus apply state.json handles EVERYTHING else, including Proxmox!
#
# Usage: ./bootstrap-minimal.sh /dev/sda state.json

set -euo pipefail

DEVICE="${1:-}"
STATE_FILE="${2:-}"

if [ -z "$DEVICE" ] || [ -z "$STATE_FILE" ]; then
    echo "Usage: $0 <device> <state.json>"
    echo ""
    echo "Example:"
    echo "  ./bootstrap-minimal.sh /dev/sda complete-server.json"
    echo ""
    echo "This installs a MINIMAL base. Then op-dbus apply will install Proxmox!"
    exit 1
fi

if [ ! -f "$STATE_FILE" ]; then
    echo "Error: State file not found: $STATE_FILE"
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Minimal Bootstrap for op-dbus"
echo "  Device: $DEVICE"
echo "  State: $STATE_FILE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "⚠️  This creates MINIMAL base. Proxmox will be installed by op-dbus apply!"
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

MOUNT_POINT="/mnt/opdbus-bootstrap"

# ============================================================================
# PHASE 0.1: Partition Disk
# ============================================================================

echo ""
echo "PHASE 0.1: Partitioning $DEVICE..."

parted -s "$DEVICE" mklabel gpt
parted -s "$DEVICE" mkpart primary fat32 1MiB 513MiB
parted -s "$DEVICE" set 1 esp on
parted -s "$DEVICE" mkpart primary ext4 513MiB 100%

mkfs.vfat -F32 "${DEVICE}1"
mkfs.ext4 -F "${DEVICE}2"

echo "✓ Partitions created"

# ============================================================================
# PHASE 0.2: Install Minimal Debian
# ============================================================================

echo ""
echo "PHASE 0.2: Installing minimal Debian..."

mkdir -p "$MOUNT_POINT"
mount "${DEVICE}2" "$MOUNT_POINT"
mkdir -p "$MOUNT_POINT/boot/efi"
mount "${DEVICE}1" "$MOUNT_POINT/boot/efi"

# Install ONLY the essentials needed to run op-dbus
debootstrap \
    --variant=minbase \
    --include=systemd,systemd-sysv,udev,dbus,packagekit,packagekit-tools,curl,gnupg \
    bookworm \
    "$MOUNT_POINT" \
    http://deb.debian.org/debian

echo "✓ Minimal Debian installed"

# ============================================================================
# PHASE 0.3: Configure Base System
# ============================================================================

echo ""
echo "PHASE 0.3: Configuring base system..."

# Mount virtual filesystems
mount --bind /proc "$MOUNT_POINT/proc"
mount --bind /sys "$MOUNT_POINT/sys"
mount --bind /dev "$MOUNT_POINT/dev"
mount --bind /dev/pts "$MOUNT_POINT/dev/pts"

# Configure fstab
BOOT_UUID=$(blkid -s UUID -o value "${DEVICE}1")
ROOT_UUID=$(blkid -s UUID -o value "${DEVICE}2")

cat > "$MOUNT_POINT/etc/fstab" <<EOF
UUID=${ROOT_UUID}  /           ext4    errors=remount-ro  0  1
UUID=${BOOT_UUID}  /boot/efi   vfat    defaults           0  2
EOF

# Set hostname (will be overridden by state.json)
echo "opdbus-bootstrap" > "$MOUNT_POINT/etc/hostname"

# Configure network (minimal DHCP, will be configured by state.json)
cat > "$MOUNT_POINT/etc/network/interfaces" <<EOF
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

# Configure DNS (temporary, will be configured by state.json)
cat > "$MOUNT_POINT/etc/resolv.conf" <<EOF
nameserver 8.8.8.8
nameserver 1.1.1.1
EOF

echo "✓ Base system configured"

# ============================================================================
# PHASE 0.4: Install op-dbus
# ============================================================================

echo ""
echo "PHASE 0.4: Installing op-dbus..."

# Copy current op-dbus source to target
cp -r . "$MOUNT_POINT/root/operation-dbus"

chroot "$MOUNT_POINT" /bin/bash <<'CHROOT_EOF'
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Build op-dbus with all features
cd /root/operation-dbus
cargo build --release --all-features

# Install binaries
cp target/release/op-dbus /usr/local/bin/
cp target/release/dbus-mcp /usr/local/bin/ 2>/dev/null || true
cp target/release/dbus-mcp-web /usr/local/bin/ 2>/dev/null || true

# Make executable
chmod +x /usr/local/bin/op-dbus

echo "✓ op-dbus installed"
CHROOT_EOF

# ============================================================================
# PHASE 0.5: Install Bootloader
# ============================================================================

echo ""
echo "PHASE 0.5: Installing bootloader..."

chroot "$MOUNT_POINT" /bin/bash <<CHROOT_EOF
# Install GRUB
apt-get update
apt-get install -y grub-efi-amd64 linux-image-amd64

# Install to EFI
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=debian

# Update GRUB config
update-grub

echo "✓ Bootloader installed"
CHROOT_EOF

# ============================================================================
# PHASE 0.6: Copy State File
# ============================================================================

echo ""
echo "PHASE 0.6: Copying state file..."

cp "$STATE_FILE" "$MOUNT_POINT/root/state.json"

echo "✓ State file copied to /root/state.json"

# ============================================================================
# PHASE 0.7: Create First-Boot Service
# ============================================================================

echo ""
echo "PHASE 0.7: Creating first-boot service..."

# Create systemd service to run op-dbus apply on first boot
cat > "$MOUNT_POINT/etc/systemd/system/opdbus-firstboot.service" <<'EOF'
[Unit]
Description=op-dbus First Boot - Apply State
After=network-online.target
Wants=network-online.target
ConditionPathExists=/root/state.json
ConditionPathExists=!/root/.opdbus-applied

[Service]
Type=oneshot
ExecStart=/usr/local/bin/op-dbus apply /root/state.json
ExecStartPost=/bin/touch /root/.opdbus-applied
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

chroot "$MOUNT_POINT" systemctl enable opdbus-firstboot

echo "✓ First-boot service created"
echo "   This will run 'op-dbus apply /root/state.json' on first boot!"

# ============================================================================
# PHASE 0.8: Cleanup
# ============================================================================

echo ""
echo "PHASE 0.8: Finalizing..."

umount "$MOUNT_POINT/dev/pts"
umount "$MOUNT_POINT/dev"
umount "$MOUNT_POINT/sys"
umount "$MOUNT_POINT/proc"
umount "$MOUNT_POINT/boot/efi"
umount "$MOUNT_POINT"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ MINIMAL BOOTSTRAP COMPLETE!"
echo ""
echo "System installed to: $DEVICE"
echo "State file: /root/state.json"
echo ""
echo "🔥 IMPORTANT: On first boot, the system will automatically run:"
echo "   op-dbus apply /root/state.json"
echo ""
echo "This will install:"
echo "  - Proxmox VE (via PackageKit plugin)"
echo "  - Network configuration (bridges, etc.)"
echo "  - Storage configuration (BTRFS subvolumes)"
echo "  - Services, users, firewall, containers"
echo "  - EVERYTHING defined in state.json"
echo ""
echo "Next steps:"
echo "  1. Reboot the system"
echo "  2. Watch the installation: journalctl -fu opdbus-firstboot"
echo "  3. After completion, Proxmox will be fully installed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
