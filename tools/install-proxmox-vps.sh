#!/bin/bash
# install-proxmox-vps.sh - Complete VPS Proxmox installer
#
# This script:
# 1. Partitions the target drive
# 2. Creates ESP (2GB) and root partition
# 3. Formats partitions (FAT32 for ESP, BTRFS for root)
# 4. Installs GRUB
# 5. Copies Proxmox installer ISO to ESP
# 6. Copies netboot.xyz to ESP
# 7. Creates GRUB configuration
# 8. Reboots into Proxmox installer

set -euo pipefail

DEVICE="${1:-}"
ISO_FILE="${2:-proxmox-ve_9.0-1-packagekit.iso}"

if [ -z "$DEVICE" ]; then
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  Proxmox VPS Complete Installer"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    echo "Usage: $0 <device> [iso-file]"
    echo ""
    echo "Example:"
    echo "  $0 /dev/sda"
    echo "  $0 /dev/nvme0n1 custom-proxmox.iso"
    echo ""
    echo "⚠️  WARNING: This will DESTROY all data on the device!"
    echo ""
    echo "Available devices:"
    lsblk -d -n -o NAME,SIZE,TYPE | grep disk | awk '{print "  /dev/" $1 " (" $2 ")"}'
    echo ""
    exit 1
fi

if [ ! -b "$DEVICE" ]; then
    echo "❌ Error: $DEVICE is not a block device"
    exit 1
fi

if [ ! -f "$ISO_FILE" ]; then
    echo "❌ Error: ISO file not found: $ISO_FILE"
    echo ""
    echo "Expected location: proxmox-ve_9.0-1-packagekit.iso"
    echo "Run ./tools/patch-proxmox-iso.sh first to create it"
    exit 1
fi

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Error: This script must be run as root"
    echo "Run: sudo $0 $*"
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VPS Complete Installer"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Device:     $DEVICE"
echo "ISO:        $ISO_FILE"
echo "ESP size:   2GB"
echo "Root:       Remaining space (BTRFS)"
echo ""
echo "⚠️  WARNING: ALL DATA ON $DEVICE WILL BE DESTROYED!"
echo ""
read -p "Type 'yes' to continue: " confirm

if [ "$confirm" != "yes" ]; then
    echo "Aborted."
    exit 0
fi

# Detect device naming (sda vs nvme0n1)
if [[ "$DEVICE" == *"nvme"* ]] || [[ "$DEVICE" == *"mmcblk"* ]]; then
    PART_PREFIX="${DEVICE}p"
else
    PART_PREFIX="${DEVICE}"
fi

ESP_PART="${PART_PREFIX}1"
ROOT_PART="${PART_PREFIX}2"

echo ""
echo "━━━ Step 1: Partitioning $DEVICE ━━━"
echo ""

# Unmount any existing partitions
umount -R /mnt 2>/dev/null || true
swapoff -a 2>/dev/null || true

# Wipe existing partition table
wipefs -a "$DEVICE"

# Create GPT partition table
parted -s "$DEVICE" mklabel gpt

# Create ESP partition (2GB)
parted -s "$DEVICE" mkpart ESP fat32 1MiB 2049MiB
parted -s "$DEVICE" set 1 esp on

# Create root partition (remaining space)
parted -s "$DEVICE" mkpart primary btrfs 2049MiB 100%

# Inform kernel of partition changes
partprobe "$DEVICE"
sleep 2

echo "✓ Partitions created:"
echo "  $ESP_PART  - 2GB ESP (FAT32)"
echo "  $ROOT_PART - Root (BTRFS)"

echo ""
echo "━━━ Step 2: Formatting Partitions ━━━"
echo ""

# Format ESP as FAT32
mkfs.vfat -F32 -n ESP "$ESP_PART"
echo "✓ ESP formatted as FAT32"

# Format root as BTRFS
mkfs.btrfs -f -L PROXMOX "$ROOT_PART"
echo "✓ Root formatted as BTRFS"

echo ""
echo "━━━ Step 3: Mounting Filesystems ━━━"
echo ""

# Mount ESP
mkdir -p /boot/efi
mount "$ESP_PART" /boot/efi
echo "✓ ESP mounted at /boot/efi"

# Mount root (for future use)
mkdir -p /mnt/proxmox
mount "$ROOT_PART" /mnt/proxmox
echo "✓ Root mounted at /mnt/proxmox"

echo ""
echo "━━━ Step 4: Installing GRUB ━━━"
echo ""

# Install GRUB to ESP
grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=GRUB --removable

echo "✓ GRUB installed"

echo ""
echo "━━━ Step 5: Copying Proxmox ISO to ESP ━━━"
echo ""

# Copy ISO to ESP (GRUB will boot it directly)
mkdir -p /boot/efi/iso
cp "$ISO_FILE" /boot/efi/iso/proxmox-installer.iso

echo "✓ Proxmox ISO copied to ESP"

echo ""
echo "━━━ Step 6: Installing netboot.xyz ━━━"
echo ""

NETBOOT_DIR="/boot/efi/netboot.xyz"
mkdir -p "$NETBOOT_DIR"

# Copy from repo if available
if [ -f "boot/netboot.xyz/netboot.xyz.efi" ]; then
    cp boot/netboot.xyz/netboot.xyz.efi "$NETBOOT_DIR/"
    echo "✓ netboot.xyz copied from repo"
else
    # Download if not in repo
    echo "Downloading netboot.xyz..."
    wget -q -O "$NETBOOT_DIR/netboot.xyz.efi" \
        https://boot.netboot.xyz/ipxe/netboot.xyz.efi
    echo "✓ netboot.xyz downloaded"
fi

echo ""
echo "━━━ Step 7: Creating GRUB Configuration ━━━"
echo ""

# Create GRUB config
cat > /boot/efi/grub/grub.cfg <<'EOF'
set timeout=10
set default=0

menuentry "Proxmox VE 9 Installer (PackageKit)" {
    set isofile="/iso/proxmox-installer.iso"
    loopback loop $isofile
    linux (loop)/boot/linux26 boot=live noprompt noeject splash quiet vga=791 findiso=$isofile
    initrd (loop)/boot/initrd.img
}

menuentry "netboot.xyz" {
    chainloader /netboot.xyz/netboot.xyz.efi
}
EOF

echo "✓ GRUB configuration created"

# Sync to disk
sync

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✓ Installation Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Summary:"
echo "  Device:     $DEVICE"
echo "  ESP:        $ESP_PART (2GB, FAT32)"
echo "  Root:       $ROOT_PART (BTRFS)"
echo "  Bootloader: GRUB"
echo "  Default:    Proxmox VE 9 Installer"
echo ""
echo "The system will now reboot into the Proxmox installer."
echo "The installer will use PackageKit (no dpkg/apt)."
echo ""
read -p "Press Enter to reboot now, or Ctrl+C to cancel..."

# Unmount and reboot
umount /mnt/proxmox
rmdir /mnt/proxmox
sync

reboot
