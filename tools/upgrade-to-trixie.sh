#!/bin/bash
# upgrade-to-trixie.sh - Upgrade Bookworm to Trixie for Proxmox VE 9
#
# WARNING: This upgrades your entire system to Debian testing!
# Only do this if you want the latest Proxmox VE 9.

set -euo pipefail

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ⚠️  Upgrade Bookworm → Trixie"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "This will upgrade Debian Bookworm (12) to Trixie (13)."
echo "This is required for Proxmox VE 9."
echo ""
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    exit 1
fi

# 1. Backup sources.list
sudo cp /etc/apt/sources.list /etc/apt/sources.list.bookworm-backup

# 2. Update sources to Trixie
sudo sed -i 's/bookworm/trixie/g' /etc/apt/sources.list

# 3. Update package lists
sudo apt-get update

# 4. Upgrade to Trixie
echo "Starting upgrade (this may take 30-60 minutes)..."
sudo apt-get upgrade -y
sudo apt-get dist-upgrade -y

# 5. Verify
echo ""
echo "Checking Debian version..."
cat /etc/debian_version

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ Upgrade complete!"
echo ""
echo "You're now on Debian Trixie and can install Proxmox VE 9."
echo ""
echo "Next step:"
echo "  ./quick-test-pve9.sh"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
