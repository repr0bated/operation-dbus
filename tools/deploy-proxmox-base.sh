#!/bin/bash
# deploy-proxmox-base.sh - Deploy Proxmox base image to target disk

set -euo pipefail

DEVICE="${1:-}"
IMAGE_FILE="${2:-deploy/vm-100-disk-1.raw}"

if [ -z "$DEVICE" ]; then
    echo "Usage: $0 <device> [image-file]"
    echo ""
    echo "Example:"
    echo "  $0 /dev/sda"
    echo "  $0 /dev/nvme0n1 custom-image.raw"
    echo ""
    echo "⚠️  WARNING: This will DESTROY all data on the device!"
    echo ""
    exit 1
fi

if [ ! -b "$DEVICE" ]; then
    echo "❌ Error: $DEVICE is not a block device"
    exit 1
fi

if [ ! -f "$IMAGE_FILE" ]; then
    echo "❌ Error: Image file not found: $IMAGE_FILE"
    echo ""
    echo "Run ./tools/download-proxmox-base.sh first"
    exit 1
fi

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Error: This script must be run as root"
    echo "Run: sudo $0 $*"
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox Base Image Deployment"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Device:     $DEVICE"
echo "Image:      $IMAGE_FILE"
echo "Method:     Rsync to @ subvolume"
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

# Install dependencies
echo ""
echo "━━━ Installing Dependencies ━━━"
echo ""
apt-get update
apt-get install -y \
    parted \
    dosfstools \
    btrfs-progs \
    grub-efi-amd64 \
    grub-efi-amd64-bin \
    rsync

echo "✓ Dependencies installed"

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
echo "━━━ Step 3: Creating BTRFS Subvolumes ━━━"
echo ""

# Mount root temporarily
mkdir -p /mnt/btrfs-root
mount "$ROOT_PART" /mnt/btrfs-root

# Create @ subvolume
btrfs subvolume create /mnt/btrfs-root/@

echo "✓ Created @ subvolume"

# Unmount root
umount /mnt/btrfs-root
rmdir /mnt/btrfs-root

echo ""
echo "━━━ Step 4: Mounting Filesystems ━━━"
echo ""

# Mount @ subvolume
mkdir -p /mnt/target
mount -o subvol=@ "$ROOT_PART" /mnt/target

# Mount ESP
mkdir -p /mnt/target/boot/efi
mount "$ESP_PART" /mnt/target/boot/efi

echo "✓ Mounted @ subvolume and ESP"

echo ""
echo "━━━ Step 5: Mounting Source Image ━━━"
echo ""

# Mount source image
mkdir -p /mnt/source
mount -o loop,ro "$IMAGE_FILE" /mnt/source

echo "✓ Source image mounted"

echo ""
echo "━━━ Step 6: Copying System (this will take several minutes) ━━━"
echo ""

# Rsync from source to target @ subvolume
rsync -aHAXv --info=progress2 \
    --exclude='/boot/efi/*' \
    --exclude='/dev/*' \
    --exclude='/proc/*' \
    --exclude='/sys/*' \
    --exclude='/tmp/*' \
    --exclude='/run/*' \
    --exclude='/mnt/*' \
    --exclude='/media/*' \
    /mnt/source/ /mnt/target/

echo "✓ System copied to @ subvolume"

# Unmount source
umount /mnt/source
rmdir /mnt/source

echo ""
echo "━━━ Step 7: Installing netboot.xyz ━━━"
echo ""

NETBOOT_DIR="/mnt/target/boot/efi/netboot.xyz"
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
echo "━━━ Step 8: Installing GRUB ━━━"
echo ""

# Bind mount necessary filesystems for chroot
mount --bind /dev /mnt/target/dev
mount --bind /proc /mnt/target/proc
mount --bind /sys /mnt/target/sys

# Install GRUB from chroot
chroot /mnt/target grub-install \
    --target=x86_64-efi \
    --efi-directory=/boot/efi \
    --boot-directory=/boot \
    --bootloader-id=Proxmox \
    --removable \
    --no-nvram

# Generate GRUB config with netboot.xyz entry
cat > /mnt/target/etc/grub.d/40_custom <<'GRUBEOF'
#!/bin/sh
exec tail -n +3 $0
# Custom entries

menuentry "netboot.xyz" {
    chainloader /netboot.xyz/netboot.xyz.efi
}
GRUBEOF

chmod +x /mnt/target/etc/grub.d/40_custom

# Generate GRUB config
chroot /mnt/target update-grub

echo "✓ GRUB installed with netboot.xyz entry"

# Unmount bind mounts
umount /mnt/target/dev
umount /mnt/target/proc
umount /mnt/target/sys

echo ""
echo "━━━ Step 9: Updating fstab ━━━"
echo ""

# Get UUID of root partition
ROOT_UUID=$(blkid -s UUID -o value "$ROOT_PART")
ESP_UUID=$(blkid -s UUID -o value "$ESP_PART")

# Create fstab
cat > /mnt/target/etc/fstab <<EOF
# <file system> <mount point> <type> <options> <dump> <pass>
UUID=$ROOT_UUID  /           btrfs  subvol=@,defaults,noatime  0  1
UUID=$ESP_UUID   /boot/efi   vfat   defaults                   0  2
EOF

echo "✓ fstab updated"

echo ""
echo "━━━ Step 10: Expanding Filesystem ━━━"
echo ""

# Resize BTRFS to use full partition
btrfs filesystem resize max /mnt/target

echo "✓ Filesystem expanded to use full disk"

# Sync to disk
sync

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✓ Deployment Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Summary:"
echo "  Device:        $DEVICE"
echo "  ESP:           $ESP_PART (UUID: $ESP_UUID)"
echo "  Root:          $ROOT_PART (UUID: $ROOT_UUID)"
echo "  Subvolume:     @"
echo "  Bootloader:    GRUB"
echo ""
echo "On first boot:"
echo "  1. Machine ID will be regenerated"
echo "  2. SSH host keys will be regenerated"
echo "  3. /etc/packagekit-setup.sh will run to install PackageKit"
echo ""
echo "The system is ready to boot."
echo ""
read -p "Press Enter to unmount and reboot, or Ctrl+C to stay in live environment..."

# Unmount and reboot
umount /mnt/target/boot/efi
umount /mnt/target
rmdir /mnt/target
sync

reboot
