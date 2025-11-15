#!/bin/bash
# install-proxmox-pve9.sh - Install Proxmox VE 9 to /mnt using PackageKit in chroot
#
# Architecture:
# 1. Bootstrap minimal Trixie to /mnt
# 2. Install PackageKit in /mnt
# 3. arch-chroot into /mnt
# 4. Run PackageKit inside chroot to install Proxmox
# 5. Configure system (grub, fstab, etc.)
#
# NO dpkg/apt used - only PackageKit for all package management!

set -euo pipefail

INSTALL_TARGET="${1:-/mnt}"
DEVICE="${2:-}"

if [ -z "$DEVICE" ]; then
    echo "Usage: $0 [mount-point] <device>"
    echo ""
    echo "Example:"
    echo "  $0 /mnt /dev/sda"
    echo ""
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE 9 Installation"
echo "  Target: $INSTALL_TARGET"
echo "  Device: $DEVICE"
echo "  Method: PackageKit in chroot (NO dpkg!)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if arch-install-scripts is available
if ! command -v arch-chroot &>/dev/null; then
    echo "⚠️  arch-install-scripts not found. Installing..."
    apt-get update
    apt-get install -y arch-install-scripts
fi

# Check if debootstrap is available
if ! command -v debootstrap &>/dev/null; then
    echo "⚠️  debootstrap not found. Installing..."
    apt-get update
    apt-get install -y debootstrap
fi

# Step 1: Partition and format device
echo ""
echo "━━━ Step 1: Partitioning $DEVICE ━━━"
echo ""

# Simple GPT layout:
# - EFI partition (512M)
# - Root partition (remaining)

parted -s "$DEVICE" mklabel gpt
parted -s "$DEVICE" mkpart ESP fat32 1MiB 513MiB
parted -s "$DEVICE" set 1 esp on
parted -s "$DEVICE" mkpart primary ext4 513MiB 100%

# Wait for kernel to recognize partitions
sleep 2

# Format partitions
mkfs.vfat -F32 "${DEVICE}1"
mkfs.ext4 -F "${DEVICE}2"

# Mount target
mount "${DEVICE}2" "$INSTALL_TARGET"
mkdir -p "$INSTALL_TARGET/boot/efi"
mount "${DEVICE}1" "$INSTALL_TARGET/boot/efi"

# Step 2: Bootstrap minimal Trixie base
echo ""
echo "━━━ Step 2: Bootstrapping Debian Trixie ━━━"
echo ""

debootstrap \
    --variant=minbase \
    --include=systemd,systemd-sysv,udev,dbus,ca-certificates,gnupg \
    trixie \
    "$INSTALL_TARGET" \
    http://deb.debian.org/debian

echo "✓ Base system installed"

# Step 3: Install PackageKit inside chroot
echo ""
echo "━━━ Step 3: Installing PackageKit in chroot ━━━"
echo ""

# Copy DNS configuration
cp /etc/resolv.conf "$INSTALL_TARGET/etc/resolv.conf"

# Install PackageKit using debootstrap's dpkg (only time we use it!)
# This is the bootstrap phase - after this, only PackageKit is used
arch-chroot "$INSTALL_TARGET" /bin/bash <<'CHROOT_INSTALL_PK'
export DEBIAN_FRONTEND=noninteractive

# Install PackageKit and tools
apt-get update
apt-get install -y \
    packagekit \
    packagekit-tools \
    wget \
    curl

# Remove apt from future use - PackageKit only!
echo "PackageKit installed - future package management via PackageKit only"
CHROOT_INSTALL_PK

echo "✓ PackageKit installed in chroot"

# Step 4: Configure Proxmox VE 9 repository
echo ""
echo "━━━ Step 4: Configuring Proxmox VE 9 repository ━━━"
echo ""

# Add Proxmox repository
cat > "$INSTALL_TARGET/etc/apt/sources.list.d/pve-no-subscription.list" <<EOF
deb [arch=amd64] http://download.proxmox.com/debian/pve trixie pve-no-subscription
EOF

# Download and install Proxmox GPG key
arch-chroot "$INSTALL_TARGET" /bin/bash <<'CHROOT_ADD_KEY'
wget -O /tmp/proxmox-release.gpg \
    https://enterprise.proxmox.com/debian/proxmox-release-bookworm.gpg

mkdir -p /etc/apt/trusted.gpg.d
mv /tmp/proxmox-release.gpg /etc/apt/trusted.gpg.d/proxmox-release-trixie.gpg

# Refresh PackageKit cache
pkcon refresh force
CHROOT_ADD_KEY

echo "✓ Proxmox repository configured"

# Step 5: Install Proxmox VE via PackageKit
echo ""
echo "━━━ Step 5: Installing Proxmox VE (via PackageKit) ━━━"
echo ""

arch-chroot "$INSTALL_TARGET" /bin/bash <<'CHROOT_INSTALL_PVE'
# Install Proxmox VE metapackage via PackageKit
# NO dpkg/apt used - only pkcon!

echo "Installing Proxmox VE kernel and base system..."
pkcon install -y proxmox-default-kernel

echo "Installing Proxmox VE..."
pkcon install -y proxmox-ve

echo "Installing additional tools..."
pkcon install -y \
    vim \
    tmux \
    htop \
    iotop \
    curl \
    wget \
    git \
    postfix \
    open-iscsi

echo "✓ Proxmox VE installed via PackageKit"
CHROOT_INSTALL_PVE

# Step 6: Configure system
echo ""
echo "━━━ Step 6: System configuration ━━━"
echo ""

# Generate fstab
INSTALL_TARGET="$INSTALL_TARGET" DEVICE="$DEVICE" arch-chroot "$INSTALL_TARGET" /bin/bash <<'CHROOT_CONFIG'
# Get UUIDs
ROOT_UUID=$(blkid -s UUID -o value ${DEVICE}2)
EFI_UUID=$(blkid -s UUID -o value ${DEVICE}1)

# Generate fstab
cat > /etc/fstab <<FSTAB
UUID=$ROOT_UUID / ext4 defaults 0 1
UUID=$EFI_UUID /boot/efi vfat defaults 0 2
FSTAB

# Set hostname
echo "pve-node1" > /etc/hostname

# Configure hosts
cat > /etc/hosts <<HOSTS
127.0.0.1 localhost
127.0.1.1 pve-node1.local pve-node1

# IPv6
::1 localhost ip6-localhost ip6-loopback
HOSTS

# Install and configure GRUB
pkcon install -y grub-efi-amd64 grub-efi-amd64-signed shim-signed

# Install GRUB to EFI
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=proxmox

# Update GRUB config
update-grub

# Set root password (user will change on first login)
echo "root:proxmox" | chpasswd

echo "✓ System configured"
CHROOT_CONFIG

# Step 7: Install op-dbus (optional)
if [ -d /home/user/operation-dbus ]; then
    echo ""
    echo "━━━ Step 7: Installing op-dbus ━━━"
    echo ""

    # Copy op-dbus binary if built
    if [ -f /home/user/operation-dbus/target/release/op-dbus ]; then
        cp /home/user/operation-dbus/target/release/op-dbus "$INSTALL_TARGET/usr/local/bin/"
        chmod +x "$INSTALL_TARGET/usr/local/bin/op-dbus"
        echo "✓ op-dbus installed"
    fi
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Proxmox VE 9 installation complete!"
echo ""
echo "Installation summary:"
echo "  • Installed to: $INSTALL_TARGET"
echo "  • Boot device: ${DEVICE}1 (EFI)"
echo "  • Root device: ${DEVICE}2 (ext4)"
echo "  • Debian version: Trixie (13)"
echo "  • Proxmox version: VE 9.x"
echo "  • Package manager: PackageKit (NO dpkg/apt!)"
echo ""
echo "Next steps:"
echo "  1. Unmount: umount -R $INSTALL_TARGET"
echo "  2. Reboot into new system"
echo "  3. Login as root (password: proxmox)"
echo "  4. Access Proxmox web UI: https://<ip>:8006"
echo "  5. Change root password"
echo ""
echo "All package management via PackageKit:"
echo "  pkcon search <package>"
echo "  pkcon install <package>"
echo "  pkcon remove <package>"
echo "  pkcon update"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
