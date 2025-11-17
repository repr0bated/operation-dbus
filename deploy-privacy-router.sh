#!/usr/bin/env bash
# Deploy vps-privacy-router.nix to remote server
# Usage: ./deploy-privacy-router.sh

set -e

echo "Starting deployment of vps-privacy-router.nix..."

sshpass -p 'O52131o4' ssh -o StrictHostKeyChecking=no -o ConnectTimeout=30 root@80.200.240.244 << 'EOF'
echo "Starting deployment..."

# Create deployment directory
mkdir -p /tmp/op-dbus-deploy

# Copy nix files from the mounted git repo
cp -r /mnt/git/operation-dbus/nix/* /tmp/op-dbus-deploy/

# Change to deployment directory
cd /tmp/op-dbus-deploy

# Backup existing configuration
echo "Backing up existing NixOS config..."
if [ -f /etc/nixos/configuration.nix ]; then
    cp /etc/nixos/configuration.nix /etc/nixos/configuration.nix.backup.$(date +%Y%m%d-%H%M%S)
    echo "Backup created"
else
    echo "No existing config to backup"
fi

# Install the vps-privacy-router.nix configuration
echo "Installing NixOS configuration..."
cp vps-privacy-router.nix /etc/nixos/configuration.nix
cp module.nix /etc/nixos/
cp package.nix /etc/nixos/
# Don't copy flake.nix - using classic mode instead

# Create minimal hardware configuration if it doesn't exist
if [ ! -f /etc/nixos/hardware-configuration.nix ]; then
    echo "Creating minimal hardware configuration..."
    cat > /etc/nixos/hardware-configuration.nix << 'HWEOF'
{ config, lib, pkgs, modulesPath, ... }:

{
  imports = [ (modulesPath + "/profiles/qemu-guest.nix") ];

  boot.initrd.availableKernelModules = [ "ata_piix" "virtio_pci" "virtio_scsi" "xhci_pci" "sd_mod" "sr_mod" ];
  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [ ];
  boot.extraModulePackages = [ ];

  fileSystems."/" = {
    device = "/dev/vda1";
    fsType = "ext4";
  };

  swapDevices = [ ];

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
}
HWEOF
fi

# Build and switch configuration (classic mode, no flakes)
echo "Building NixOS configuration (this may take several minutes)..."
nixos-rebuild switch --no-flake

# Check deployment status
echo "Checking deployment status..."
echo "=== op-dbus status ==="
systemctl status op-dbus --no-pager || echo "op-dbus service not found"

echo "=== OVS bridges ==="
ovs-vsctl show || echo "OVS not ready yet"

echo "=== LXC containers ==="
lxc-ls -f || echo "No containers yet"

echo "=== Recent op-dbus logs ==="
journalctl -u op-dbus -n 20 --no-pager || echo "No logs yet"

echo "Deployment complete!"
EOF

echo "Deployment command sent to server!"