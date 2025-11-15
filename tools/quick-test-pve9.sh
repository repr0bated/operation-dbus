#!/bin/bash
# quick-test-pve9.sh - Install Proxmox VE 9 on Trixie via PackageKit
#
# This matches: Trixie (Debian 13) + Proxmox VE 9.x

set -euo pipefail

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE 9 Installation (Trixie)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check we're on Trixie
if ! grep -q "trixie" /etc/os-release 2>/dev/null; then
    echo "⚠️  WARNING: This script is for Debian Trixie (13)"
    echo "   Your system appears to be a different version."
    echo ""
    echo "   For Bookworm (Debian 12), use: ./quick-test-pve8.sh"
    echo ""
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# 1. Install prerequisites
echo ""
echo "📦 Installing prerequisites..."
sudo apt-get update
sudo apt-get install -y git cargo rustc packagekit packagekit-tools wget

# 2. Download Proxmox VE 9 ISO
cd ~/operation-dbus/tools/proxmox-extractor

if [ ! -f ./iso/proxmox-ve_9.0-1.iso ]; then
    echo ""
    echo "📥 Downloading Proxmox VE 9 ISO..."
    ./download-pve9.sh ./iso
fi

# 3. Configure Proxmox VE 9 repository (Trixie)
echo ""
echo "🔧 Configuring Proxmox VE 9 repository..."
echo "deb [arch=amd64] http://download.proxmox.com/debian/pve trixie pve-no-subscription" | \
    sudo tee /etc/apt/sources.list.d/pve-no-subscription.list

# Download GPG key for Trixie
# Note: Proxmox VE 9 uses the same key as PVE 8 for now
wget -O- https://enterprise.proxmox.com/debian/proxmox-release-bookworm.gpg | \
    sudo tee /etc/apt/trusted.gpg.d/proxmox-release-trixie.gpg >/dev/null

sudo apt-get update

# 4. Build extractor
echo ""
echo "🔨 Building Proxmox extractor..."
cargo build --release

# 5. Extract from PVE 9 ISO
echo ""
echo "📦 Extracting packages from ISO..."
./extract-iso.sh ./iso/proxmox-ve_9.0-1.iso ./extracted-pve9

# 6. Generate manifest
echo ""
echo "📋 Generating package manifest..."
cargo run --release --bin proxmox-manifest -- \
    --packages ./extracted-pve9/packages/Packages.txt \
    --output manifest-pve9.json \
    --version 9.0

# 7. Test dry-run
echo ""
echo "🧪 Testing installation (dry-run)..."
cargo run --release --bin proxmox-packagekit -- \
    manifest-pve9.json --dry-run

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Ready to install Proxmox VE 9!"
echo ""
echo "Manifest: $(pwd)/manifest-pve9.json"
echo ""
echo "To install, run:"
echo "  sudo cargo run --release --bin proxmox-packagekit -- manifest-pve9.json"
echo ""
echo "Or use via op-dbus state.json:"
echo "  {\"plugins\": {\"packagekit\": {\"manifest\": \"$(pwd)/manifest-pve9.json\"}}}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
