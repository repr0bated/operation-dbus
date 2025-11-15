#!/bin/bash
# quick-start.sh - Complete Proxmox extraction pipeline
set -euo pipefail

ISO_PATH="${1:-}"

if [ -z "$ISO_PATH" ]; then
    echo "Usage: $0 <proxmox-ve.iso>"
    exit 1
fi

OUTPUT_DIR="./extracted-$(date +%Y%m%d-%H%M%S)"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE → PackageKit Pipeline"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 1: Extract ISO
echo ""
echo "STEP 1/2: Extracting ISO..."
./extract-iso.sh "$ISO_PATH" "$OUTPUT_DIR"

# Step 2: Generate manifest
echo ""
echo "STEP 2/2: Generating manifest..."
cargo run --release --bin proxmox-manifest -- \
    --packages "$OUTPUT_DIR/packages/Packages.txt" \
    --output "$OUTPUT_DIR/manifest.json"

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Pipeline complete!"
echo ""
echo "Manifest: $OUTPUT_DIR/manifest.json"
echo ""
echo "Next steps:"
echo "  1. Dry-run:"
echo "     cargo run --release --bin proxmox-packagekit -- \\"
echo "       $OUTPUT_DIR/manifest.json --dry-run"
echo ""
echo "  2. Install (requires root):"
echo "     sudo cargo run --release --bin proxmox-packagekit -- \\"
echo "       $OUTPUT_DIR/manifest.json"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
