#!/usr/bin/env bash
# Single-file NixOS installer with operation-dbus
# Run from netboot.xyz or NixOS live USB
# No git, no network dependencies (except for nix packages)

set -e

DISK="${1:-}"
HOSTNAME="${2:-opdbus-laptop}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "NixOS + operation-dbus Installer"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

if [ -z "$DISK" ]; then
  echo "Usage: $0 <disk> [hostname]"
  echo ""
  echo "Available disks:"
  lsblk -d -o NAME,SIZE,TYPE,MODEL | grep disk
  echo ""
  echo "Example:"
  echo "  $0 /dev/sda my-laptop"
  echo "  $0 /dev/nvme0n1 my-laptop"
  exit 1
fi

if [ ! -b "$DISK" ]; then
  echo "❌ $DISK is not a valid block device"
  exit 1
fi

echo "Target: $DISK"
echo "Hostname: $HOSTNAME"
echo ""
lsblk "$DISK"
echo ""
read -p "⚠️  This will ERASE $DISK. Type 'yes' to continue: " confirm

if [ "$confirm" != "yes" ]; then
  echo "Cancelled"
  exit 0
fi

echo ""
echo "[1/6] Partitioning $DISK..."

# Detect partition naming
if [[ "$DISK" == *"nvme"* ]] || [[ "$DISK" == *"mmcblk"* ]]; then
  PART_PREFIX="${DISK}p"
else
  PART_PREFIX="${DISK}"
fi

# Wipe and partition
sgdisk --zap-all "$DISK" 2>/dev/null || true
parted "$DISK" --script -- \
  mklabel gpt \
  mkpart ESP fat32 1MiB 512MiB \
  set 1 esp on \
  mkpart primary 512MiB 100%

sleep 2
partprobe "$DISK" 2>/dev/null || true
sleep 1

ESP="${PART_PREFIX}1"
ROOT="${PART_PREFIX}2"

echo "  ESP: $ESP"
echo "  ROOT: $ROOT"

echo "[2/6] Formatting partitions..."
mkfs.fat -F 32 -n BOOT "$ESP"
mkfs.btrfs -f -L nixos "$ROOT"

echo "[3/6] Creating BTRFS subvolumes..."
mount "$ROOT" /mnt
btrfs subvolume create /mnt/@root
btrfs subvolume create /mnt/@home
btrfs subvolume create /mnt/@nix
btrfs subvolume create /mnt/@cache
btrfs subvolume create /mnt/@state
umount /mnt

echo "[4/6] Mounting filesystems..."
mount -o subvol=@root,compress=zstd,noatime "$ROOT" /mnt
mkdir -p /mnt/{boot,home,nix,var/cache,var/lib/op-dbus}
mount "$ESP" /mnt/boot
mount -o subvol=@home,compress=zstd,noatime "$ROOT" /mnt/home
mount -o subvol=@nix,compress=zstd,noatime "$ROOT" /mnt/nix
mount -o subvol=@cache,compress=zstd:3,noatime "$ROOT" /mnt/var/cache
mount -o subvol=@state,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus

echo "[5/6] Generating NixOS configuration..."
nixos-generate-config --root /mnt

# Create configuration.nix
cat > /mnt/etc/nixos/configuration.nix <<'NIXCONFIG'
{ config, pkgs, ... }:

{
  imports = [ ./hardware-configuration.nix ];

  # Boot
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.supportedFilesystems = [ "btrfs" ];

  # Network
  networking.hostName = "HOSTNAME_PLACEHOLDER";
  networking.networkmanager.enable = true;

  # Time
  time.timeZone = "America/New_York";

  # User
  users.users.user = {
    isNormalUser = true;
    extraGroups = [ "wheel" "networkmanager" ];
    # Default password: "password" - CHANGE THIS!
    password = "password";
  };

  # Enable sudo
  security.sudo.wheelNeedsPassword = false;

  # Packages
  environment.systemPackages = with pkgs; [
    vim
    git
    htop
    curl
    wget
    firefox
    gnome.gnome-terminal
    # Build tools for operation-dbus
    rustc
    cargo
    gcc
    pkg-config
    openssl
    dbus
    systemd
  ];

  # SSH
  services.openssh.enable = true;

  # Desktop (GNOME)
  services.xserver = {
    enable = true;
    displayManager.gdm.enable = true;
    desktopManager.gnome.enable = true;
  };

  # Sound
  sound.enable = true;
  hardware.pulseaudio.enable = true;

  # D-Bus (required for operation-dbus)
  services.dbus.enable = true;

  # Nix settings
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  system.stateVersion = "24.11";
}
NIXCONFIG

# Replace hostname placeholder
sed -i "s/HOSTNAME_PLACEHOLDER/$HOSTNAME/" /mnt/etc/nixos/configuration.nix

# Create operation-dbus build script
cat > /mnt/home/user/build-op-dbus.sh <<'BUILDSCRIPT'
#!/usr/bin/env bash
# Run this after first boot to build operation-dbus

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Building operation-dbus"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Clone repo
if [ ! -d ~/operation-dbus ]; then
  echo "Cloning repository..."
  git clone https://github.com/repr0bated/operation-dbus.git ~/operation-dbus
  cd ~/operation-dbus
  git checkout claude/this-is-no-011CUr6iUZZC7YKhhLSY2MK6
else
  echo "Repository already cloned"
  cd ~/operation-dbus
  git pull
fi

# Build
echo "Building..."
cargo build --release

echo ""
echo "✅ Build complete!"
echo ""
echo "Binary: ~/operation-dbus/target/release/op-dbus"
echo ""
echo "To install:"
echo "  cd ~/operation-dbus"
echo "  sudo cp target/release/op-dbus /usr/local/bin/"
echo ""
echo "To run:"
echo "  op-dbus --help"
echo "  op-dbus query"
BUILDSCRIPT

chmod +x /mnt/home/user/build-op-dbus.sh

# Create welcome message
cat > /mnt/etc/motd <<'MOTD'

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
          Welcome to NixOS + operation-dbus
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Login: user
Password: password (CHANGE THIS!)

To build operation-dbus:
  ~/build-op-dbus.sh

To change your password:
  passwd

To edit system configuration:
  sudo vim /etc/nixos/configuration.nix
  sudo nixos-rebuild switch

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

MOTD

echo "[6/6] Installing NixOS (this takes 10-30 minutes)..."
nixos-install --no-root-passwd

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Installation Complete!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Installed to: $DISK"
echo "Hostname: $HOSTNAME"
echo ""
echo "Next steps:"
echo "1. Reboot: sudo reboot"
echo "2. Login with:"
echo "   Username: user"
echo "   Password: password"
echo "3. Build operation-dbus:"
echo "   ~/build-op-dbus.sh"
echo "4. CHANGE YOUR PASSWORD:"
echo "   passwd"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

read -p "Reboot now? (yes/NO): " reboot_confirm
if [ "$reboot_confirm" = "yes" ]; then
  reboot
fi
