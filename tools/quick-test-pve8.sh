#!/bin/bash
# quick-test-pve8.sh - Install Proxmox VE 8 on Bookworm via PackageKit
#
# This matches: Bookworm (Debian 12) + Proxmox VE 8.x

set -euo pipefail

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Proxmox VE 8 Installation (Bookworm)"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# 1. Install prerequisites
sudo apt-get update
sudo apt-get install -y git cargo rustc packagekit packagekit-tools wget

# 2. Download Proxmox VE 8 ISO (NOT VE 9!)
if [ ! -f ~/proxmox-ve_8.2-1.iso ]; then
    echo "Downloading Proxmox VE 8.2 ISO..."
    wget -O ~/proxmox-ve_8.2-1.iso \
        https://enterprise.proxmox.com/iso/proxmox-ve_8.2-1.iso
fi

# 3. Configure Proxmox VE 8 repository (Bookworm)
echo "deb [arch=amd64] http://download.proxmox.com/debian/pve bookworm pve-no-subscription" | \
    sudo tee /etc/apt/sources.list.d/pve-no-subscription.list

wget -O- https://enterprise.proxmox.com/debian/proxmox-release-bookworm.gpg | \
    sudo tee /etc/apt/trusted.gpg.d/proxmox-release-bookworm.gpg

sudo apt-get update

# 4. Build extractor
cd ~/operation-dbus/tools/proxmox-extractor
cargo build --release

# 5. Extract from PVE 8 ISO
./extract-iso.sh ~/proxmox-ve_8.2-1.iso ./extracted-pve8

# 6. Generate manifest
cargo run --release --bin proxmox-manifest -- \
    --packages ./extracted-pve8/packages/Packages.txt \
    --output manifest-pve8.json \
    --version 8.2

# 7. Test dry-run
cargo run --release --bin proxmox-packagekit -- \
    manifest-pve8.json --dry-run

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Ready to install Proxmox VE 8!"
echo ""
echo "To install, run:"
echo "  sudo cargo run --release --bin proxmox-packagekit -- manifest-pve8.json"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
