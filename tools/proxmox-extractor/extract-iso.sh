#!/bin/bash
# extract-iso.sh - Extract package lists from Proxmox VE 9 ISO
set -euo pipefail

ISO_PATH="${1:-}"
OUTPUT_DIR="${2:-./extracted}"

if [ -z "$ISO_PATH" ]; then
    echo "Usage: $0 <proxmox-ve.iso> [output-dir]"
    exit 1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE ISO Package Extractor"
echo "  ISO: $ISO_PATH"
echo "  Output: $OUTPUT_DIR"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

mkdir -p "$OUTPUT_DIR"/{mount,packages}

echo "📀 Mounting ISO..."
sudo mount -o loop,ro "$ISO_PATH" "$OUTPUT_DIR/mount"

echo "📦 Extracting package lists..."
find "$OUTPUT_DIR/mount" -name "Packages.gz" -exec cp {} "$OUTPUT_DIR/packages/" \; 2>/dev/null || true

for pkg_gz in "$OUTPUT_DIR/packages"/Packages.gz*; do
    if [ -f "$pkg_gz" ]; then
        gunzip -c "$pkg_gz" > "$OUTPUT_DIR/packages/Packages.txt" 2>/dev/null || true
    fi
done

echo "✓ Package lists extracted"

echo "🔓 Unmounting ISO..."
sudo umount "$OUTPUT_DIR/mount"
rmdir "$OUTPUT_DIR/mount"

echo ""
echo "✅ Extraction complete!"
echo "Package list: $OUTPUT_DIR/packages/Packages.txt"
echo ""
echo "Next step:"
echo "  cargo run --release --bin proxmox-manifest -- \\"
echo "    --packages $OUTPUT_DIR/packages/Packages.txt \\"
echo "    --output manifest.json"
