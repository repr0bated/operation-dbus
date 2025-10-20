#!/bin/bash
# Build and install op-dbus in one shot

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== op-dbus Build & Install ===${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root or with sudo${NC}"
    exit 1
fi

# Step 1: Build as the user who owns the directory
REPO_OWNER=$(stat -c '%U' .)
echo "Building as user: $REPO_OWNER"

if [ "$REPO_OWNER" != "root" ]; then
    echo "Building op-dbus binary..."
    su - "$REPO_OWNER" -c "cd $(pwd) && cargo build --release" || {
        echo -e "${RED}Build failed. Check cargo installation.${NC}"
        exit 1
    }
else
    cargo build --release || {
        echo -e "${RED}Build failed. Check cargo installation.${NC}"
        exit 1
    }
fi

echo -e "${GREEN}✓${NC} Build complete"
echo ""

# Step 2: Run install script
echo "Running installation..."
./install.sh

echo ""
echo -e "${GREEN}=== Build & Install Complete ===${NC}"
echo ""
echo "Service is installed but NOT started yet."
echo ""
echo -e "${YELLOW}IMPORTANT: Configure your network settings first!${NC}"
echo "  sudo nano /etc/op-dbus/state.json"
echo ""
echo "Then test safely:"
echo "  sudo op-dbus query"
echo "  sudo op-dbus diff /etc/op-dbus/state.json"
echo "  sudo op-dbus apply /etc/op-dbus/state.json"
echo ""
echo "Enable at boot (after successful manual test):"
echo "  sudo systemctl enable op-dbus"
echo "  sudo systemctl start op-dbus"
