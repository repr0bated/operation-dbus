#!/bin/bash
# install-proxmox-to-esp.sh - Install Proxmox installer to ESP for VPS boot
#
# For VPS environments where you can't boot from ISO:
# 1. Extract Proxmox ISO to ESP (/boot/efi)
# 2. Configure systemd-boot to boot installer from ESP
# 3. Reboot into Proxmox installer
#
# The installer runs from ESP, installs to main disk with BTRFS

set -euo pipefail

ISO_FILE="${1:-}"
ESP_MOUNT="${2:-/boot/efi}"

if [ -z "$ISO_FILE" ]; then
    echo "Usage: $0 <proxmox-iso> [esp-mount-point]"
    echo ""
    echo "Example:"
    echo "  $0 proxmox-ve_9.0-1-packagekit.iso"
    echo "  $0 proxmox-ve_9.0-1-packagekit.iso /boot/efi"
    echo ""
    echo "This extracts the Proxmox installer to ESP and configures"
    echo "systemd-boot to boot it on next reboot (for VPS environments)."
    echo ""
    exit 1
fi

if [ ! -f "$ISO_FILE" ]; then
    echo "Error: ISO file not found: $ISO_FILE"
    echo ""
    echo "First create patched ISO:"
    echo "  ./tools/patch-proxmox-iso.sh proxmox-ve_9.0-1.iso output.iso"
    echo ""
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VPS Installer Setup"
echo "  ISO: $ISO_FILE"
echo "  ESP: $ESP_MOUNT"
echo "  Bootloader: systemd-boot"
echo "  Method: Extract to ESP + boot entry"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Verify ESP is mounted
if ! mountpoint -q "$ESP_MOUNT"; then
    echo "Error: ESP not mounted at $ESP_MOUNT"
    echo ""
    echo "Mount ESP first:"
    echo "  mount /dev/sda1 /boot/efi  # Adjust device as needed"
    echo ""
    exit 1
fi

# Step 1: Extract ISO to ESP
echo ""
echo "━━━ Step 1: Extracting ISO to ESP ━━━"
echo ""

INSTALLER_DIR="$ESP_MOUNT/proxmox-installer"
MOUNT_DIR="/tmp/proxmox-iso-mount"

# Clean old installer if exists
if [ -d "$INSTALLER_DIR" ]; then
    echo "Removing old installer..."
    rm -rf "$INSTALLER_DIR"
fi

mkdir -p "$INSTALLER_DIR"
mkdir -p "$MOUNT_DIR"

# Mount ISO
mount -o loop,ro "$ISO_FILE" "$MOUNT_DIR"

# Copy installer files
echo "Copying installer files to ESP..."
rsync -a --info=progress2 "$MOUNT_DIR/" "$INSTALLER_DIR/"

# Unmount ISO
umount "$MOUNT_DIR"
rmdir "$MOUNT_DIR"

echo "✓ Installer extracted to $INSTALLER_DIR"

# Step 2: Find kernel and initrd
echo ""
echo "━━━ Step 2: Locating kernel and initrd ━━━"
echo ""

# Proxmox kernel and initrd locations
KERNEL=$(find "$INSTALLER_DIR" -name "vmlinuz*" -o -name "linux*" | grep -v ".efi" | head -1)
INITRD=$(find "$INSTALLER_DIR" -name "initrd*" -o -name "initramfs*" | head -1)

if [ -z "$KERNEL" ] || [ -z "$INITRD" ]; then
    echo "Error: Could not find kernel or initrd in ISO"
    echo "Kernel: $KERNEL"
    echo "Initrd: $INITRD"
    exit 1
fi

# Make paths relative to ESP root (not /boot/efi)
KERNEL_REL=${KERNEL#$ESP_MOUNT/}
INITRD_REL=${INITRD#$ESP_MOUNT/}

echo "Kernel: /$KERNEL_REL"
echo "Initrd: /$INITRD_REL"

# Step 3: Configure systemd-boot
echo ""
echo "━━━ Step 3: Configuring systemd-boot ━━━"
echo ""

# Create loader entries directory if doesn't exist
LOADER_ENTRIES="$ESP_MOUNT/loader/entries"
mkdir -p "$LOADER_ENTRIES"

# Create Proxmox boot entry
ENTRY_FILE="$LOADER_ENTRIES/proxmox-installer.conf"

cat > "$ENTRY_FILE" <<EOF
title      Proxmox VE 9 Installer (PackageKit)
linux      /$KERNEL_REL
initrd     /$INITRD_REL
options    boot=live noprompt noeject splash quiet vga=791
EOF

echo "✓ Boot entry created: $ENTRY_FILE"

# Step 4: Boot configuration (PRESERVE existing config!)
echo ""
echo "━━━ Step 4: Checking boot configuration ━━━"
echo ""

LOADER_CONF="$ESP_MOUNT/loader/loader.conf"

# Check for netboot.xyz or other boot managers
if [ -d "$ESP_MOUNT/netboot.xyz" ] || grep -q "netboot" "$ESP_MOUNT/loader/entries/"*.conf 2>/dev/null; then
    echo "⚠️  netboot.xyz detected on ESP - preserving configuration"
    PRESERVE_DEFAULT=true
else
    PRESERVE_DEFAULT=false
fi

# Backup existing config
if [ -f "$LOADER_CONF" ]; then
    cp "$LOADER_CONF" "${LOADER_CONF}.backup"
    echo "✓ Backed up: ${LOADER_CONF}.backup"
fi

# Ask user about default boot
if [ "$PRESERVE_DEFAULT" = true ]; then
    echo ""
    echo "Existing boot configuration detected."
    echo "Choose how to configure default boot:"
    echo "  1) Keep existing default (netboot.xyz?) - Manual selection required"
    echo "  2) Set Proxmox installer as default - Auto-boot installer"
    echo ""
    read -p "Choice [1/2]: " -n 1 -r
    echo ""

    if [[ $REPLY =~ ^[2]$ ]]; then
        SET_DEFAULT=true
    else
        SET_DEFAULT=false
    fi
else
    # No existing boot manager, safe to set as default
    SET_DEFAULT=true
fi

if [ "$SET_DEFAULT" = true ]; then
    # Read current settings to preserve
    TIMEOUT="5"
    CONSOLE_MODE="max"
    EDITOR="no"

    if [ -f "$LOADER_CONF" ]; then
        TIMEOUT=$(grep "^timeout" "$LOADER_CONF" | awk '{print $2}' || echo "5")
        CONSOLE_MODE=$(grep "^console-mode" "$LOADER_CONF" | awk '{print $2}' || echo "max")
        EDITOR=$(grep "^editor" "$LOADER_CONF" | awk '{print $2}' || echo "no")
    fi

    # Update loader.conf with Proxmox as default
    cat > "$LOADER_CONF" <<EOF
default  proxmox-installer.conf
timeout  $TIMEOUT
console-mode $CONSOLE_MODE
editor   $EDITOR
EOF

    echo "✓ Updated: $LOADER_CONF"
    echo "  Default: proxmox-installer.conf (auto-boot)"
    echo "  Timeout: ${TIMEOUT}s"
else
    echo "✓ Preserved existing boot configuration"
    echo "  Boot entry added: proxmox-installer.conf"
    echo "  Default: (unchanged - manual selection required)"
    echo ""
    echo "  To boot Proxmox installer:"
    echo "    1. Reboot"
    echo "    2. Press Space in systemd-boot menu"
    echo "    3. Select 'Proxmox VE 9 Installer'"
fi

# Step 5: Create reboot helper script
echo ""
echo "━━━ Step 5: Creating reboot helper ━━━"
echo ""

cat > /usr/local/bin/boot-proxmox-installer <<'REBOOT_SCRIPT'
#!/bin/bash
# Boot into Proxmox installer on next reboot

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Rebooting into Proxmox VE Installer"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "systemd-boot is configured to boot Proxmox installer"
echo ""
echo "On reboot:"
echo "  1. Installer will boot automatically (default)"
echo "  2. Or press Space in boot menu to select entry"
echo "  3. Select 'Proxmox VE 9 Installer (PackageKit)'"
echo "  4. Installer boots from ESP"
echo "  5. Install Proxmox to main disk with BTRFS"
echo ""
echo "Press Enter to reboot now, or Ctrl+C to cancel..."
read

reboot
REBOOT_SCRIPT

chmod +x /usr/local/bin/boot-proxmox-installer

echo "✓ Reboot helper created: /usr/local/bin/boot-proxmox-installer"

# Step 6: Create restore script
echo ""
echo "━━━ Step 6: Creating restore script ━━━"
echo ""

cat > /usr/local/bin/restore-systemd-boot <<RESTORE_SCRIPT
#!/bin/bash
# Restore systemd-boot configuration after Proxmox installation

ESP_MOUNT="$ESP_MOUNT"
LOADER_CONF="\$ESP_MOUNT/loader/loader.conf"
LOADER_BACKUP="\${LOADER_CONF}.backup"

echo "Restoring systemd-boot configuration..."

if [ -f "\$LOADER_BACKUP" ]; then
    mv "\$LOADER_BACKUP" "\$LOADER_CONF"
    echo "✓ Restored: \$LOADER_CONF"
else
    echo "⚠️  No backup found at \$LOADER_BACKUP"
fi

# Remove Proxmox installer entry
rm -f "\$ESP_MOUNT/loader/entries/proxmox-installer.conf"
echo "✓ Removed Proxmox installer entry"

# Optionally remove installer files
read -p "Remove installer files from ESP? [y/N] " -n 1 -r
echo
if [[ \$REPLY =~ ^[Yy]$ ]]; then
    rm -rf "$INSTALLER_DIR"
    echo "✓ Removed: $INSTALLER_DIR"
fi

echo "✓ systemd-boot restored"
RESTORE_SCRIPT

chmod +x /usr/local/bin/restore-systemd-boot

echo "✓ Restore script created: /usr/local/bin/restore-systemd-boot"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Proxmox installer ready on ESP!"
echo ""
echo "Installation summary:"
echo "  • Installer location: $INSTALLER_DIR"
echo "  • Kernel: /$KERNEL_REL"
echo "  • Initrd: /$INITRD_REL"
echo "  • Boot entry: $ENTRY_FILE"
echo "  • systemd-boot: Configured (default entry)"
echo "  • Package backend: PackageKit (no dpkg/apt)"
echo ""
echo "Next steps:"
echo ""
echo "  Option 1: Reboot using helper script (RECOMMENDED)"
echo "    /usr/local/bin/boot-proxmox-installer"
echo ""
echo "  Option 2: Manual reboot"
echo "    systemctl reboot"
echo "    # Installer will boot automatically"
echo ""
echo "During installation:"
echo "  • Proxmox installer GUI appears"
echo "  • Configure BTRFS layout"
echo "  • Set network, hostname, passwords"
echo "  • PackageKit installs all packages (no dpkg/apt!)"
echo ""
echo "After installation:"
echo "  1. Boot into new Proxmox system"
echo "  2. Restore systemd-boot config:"
echo "     /usr/local/bin/restore-systemd-boot"
echo "  3. Remove installer from ESP (optional)"
echo ""
echo "Files created:"
echo "  • Boot entry: $ENTRY_FILE"
echo "  • Loader config: $ESP_MOUNT/loader/loader.conf"
echo "  • Reboot helper: /usr/local/bin/boot-proxmox-installer"
echo "  • Restore helper: /usr/local/bin/restore-systemd-boot"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
