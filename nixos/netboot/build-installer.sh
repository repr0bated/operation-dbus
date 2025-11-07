#!/usr/bin/env bash
# Build operation-dbus NixOS installer for netboot.xyz
# Usage: ./build-installer.sh

set -e

OUTPUT_DIR="/tmp/opdbus-installer"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Building operation-dbus NixOS Installer"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if we're in the right directory
if [ ! -f "nixos/netboot/configs/installer.nix" ]; then
  echo "❌ Error: Must run from operation-dbus repository root"
  echo "   cd /path/to/operation-dbus"
  echo "   ./nixos/netboot/build-installer.sh"
  exit 1
fi

echo "📋 Configuration:"
echo "   Input:  nixos/netboot/configs/installer.nix"
echo "   Output: $OUTPUT_DIR"
echo ""

# Build the netboot image
echo "🔨 Building installer image (this may take 5-15 minutes)..."
echo ""

nix-build '<nixpkgs/nixos>' \
  -A config.system.build.netbootRamdisk \
  -I nixos-config="$(pwd)/nixos/netboot/configs/installer.nix" \
  -o "$OUTPUT_DIR" \
  --show-trace

# Check build results
echo ""
echo "✅ Build complete!"
echo ""

if [ ! -f "$OUTPUT_DIR/bzImage" ] || [ ! -f "$OUTPUT_DIR/initrd" ]; then
  echo "❌ Error: Expected files not found in $OUTPUT_DIR"
  ls -la "$OUTPUT_DIR/"
  exit 1
fi

# Calculate sizes
KERNEL_SIZE=$(du -h "$OUTPUT_DIR/bzImage" | cut -f1)
INITRD_SIZE=$(du -h "$OUTPUT_DIR/initrd" | cut -f1)

echo "📦 Generated files:"
echo "   Kernel (bzImage): $KERNEL_SIZE"
echo "   Initrd:           $INITRD_SIZE"
echo ""

# Generate checksums
cd "$OUTPUT_DIR"
sha256sum bzImage initrd > SHA256SUMS

echo "🔐 Checksums:"
cat SHA256SUMS | sed 's/^/   /'
echo ""

# Calculate total size
TOTAL_SIZE=$(du -sh "$OUTPUT_DIR" | cut -f1)
echo "💾 Total size: $TOTAL_SIZE"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Next steps:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1️⃣  Test with local HTTP server:"
echo "    cd $OUTPUT_DIR && python3 -m http.server 8080"
echo ""
echo "2️⃣  Copy to your web server:"
echo "    WEB_SERVER=user@webserver.example.com"
echo "    WEB_PATH=/var/www/netboot/installer"
echo "    ssh \$WEB_SERVER 'mkdir -p \$WEB_PATH'"
echo "    scp $OUTPUT_DIR/* \$WEB_SERVER:\$WEB_PATH/"
echo ""
echo "3️⃣  Add to netboot.xyz custom menu (custom.ipxe):"
echo ""
echo "    :opdbus-installer"
echo "    kernel http://YOUR-SERVER/netboot/installer/bzImage init=/nix/store/.../init"
echo "    initrd http://YOUR-SERVER/netboot/installer/initrd"
echo "    boot"
echo ""
echo "4️⃣  Boot target machine via netboot.xyz:"
echo "    - Select: Custom → operation-dbus Installer"
echo "    - Wait for boot (1-2 minutes)"
echo "    - SSH: ssh root@<ip>  (password: nixos)"
echo "    - Install: sudo /etc/opdbus-install.sh /dev/sda hostname"
echo ""
echo "5️⃣  After installation:"
echo "    - Reboot the machine"
echo "    - SSH into new system"
echo "    - Enable operation-dbus: vim /etc/nixos/configuration.nix"
echo "    - Apply: nixos-rebuild switch"
echo ""
echo "See NETBOOT-TO-DISK-INSTALL.md for detailed instructions."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
