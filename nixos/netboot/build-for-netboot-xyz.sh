#!/usr/bin/env bash
# Build NixOS images for netboot.xyz integration
# Usage: ./build-for-netboot-xyz.sh [config-name]

set -e

CONFIG="${1:-proxmox}"
OUTPUT_DIR="/tmp/netboot-opdbus-$CONFIG"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Building NixOS netboot image: $CONFIG"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check if configuration exists
CONFIG_FILE="./configs/$CONFIG.nix"
if [ ! -f "$CONFIG_FILE" ]; then
  echo "❌ Configuration not found: $CONFIG_FILE"
  echo ""
  echo "Available configurations:"
  ls -1 ./configs/*.nix | sed 's|./configs/||; s|\.nix||' | sed 's/^/  - /'
  exit 1
fi

echo "Configuration: $CONFIG_FILE"
echo "Output: $OUTPUT_DIR"
echo ""

# Build the netboot image
echo "🔨 Building image (this may take a while)..."
nix-build '<nixpkgs/nixos>' \
  -A config.system.build.netbootRamdisk \
  -I nixos-config="$CONFIG_FILE" \
  -o "$OUTPUT_DIR" \
  --show-trace

# Check what was built
echo ""
echo "✅ Build complete!"
echo ""
echo "Generated files:"
ls -lh "$OUTPUT_DIR"/ | grep -E "(bzImage|initrd)" || echo "No files found"

# Calculate sizes
KERNEL_SIZE=$(du -h "$OUTPUT_DIR/bzImage" | cut -f1)
INITRD_SIZE=$(du -h "$OUTPUT_DIR/initrd" | cut -f1)

echo ""
echo "📦 Image sizes:"
echo "  Kernel (bzImage): $KERNEL_SIZE"
echo "  Initrd: $INITRD_SIZE"

# Generate checksums
cd "$OUTPUT_DIR"
sha256sum bzImage initrd > SHA256SUMS
echo ""
echo "🔐 Checksums:"
cat SHA256SUMS

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Next steps:"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "1️⃣  Test with local HTTP server:"
echo "    cd $OUTPUT_DIR && python3 -m http.server 8080"
echo ""
echo "2️⃣  Copy to your web server:"
echo "    scp $OUTPUT_DIR/* user@webserver:/var/www/netboot/$CONFIG/"
echo ""
echo "3️⃣  Add to netboot.xyz custom menu (custom.ipxe):"
echo "    :nixos-$CONFIG"
echo "    kernel http://YOUR-SERVER/netboot/$CONFIG/bzImage init=/nix/store/.../init"
echo "    initrd http://YOUR-SERVER/netboot/$CONFIG/initrd"
echo "    boot"
echo ""
echo "4️⃣  Boot a machine via netboot.xyz and test!"
echo ""
echo "See NETBOOT-XYZ-INTEGRATION.md for full instructions."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
