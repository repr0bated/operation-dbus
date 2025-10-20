#!/bin/bash
# Sync netmaker interfaces to mesh bridge
# Run this after joining netmaker to automatically connect interfaces

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Netmaker Mesh Bridge Sync ==="

# Check if mesh bridge exists
if ! sudo ovs-vsctl br-exists mesh 2>/dev/null; then
    echo -e "${RED}✗${NC} mesh bridge not found"
    echo "Run install.sh first to create the mesh bridge"
    exit 1
fi

echo -e "${GREEN}✓${NC} mesh bridge exists"

# Check if netclient is installed
if ! command -v netclient >/dev/null 2>&1; then
    echo -e "${RED}✗${NC} netclient not found"
    echo "Install netclient first: sudo apt install netclient"
    exit 1
fi

echo -e "${GREEN}✓${NC} netclient installed"

# Check if host is joined to netmaker
if ! netclient list >/dev/null 2>&1 || ! netclient list | grep -q "Connected networks:"; then
    echo -e "${YELLOW}⚠${NC}  Host not joined to netmaker network"
    echo "Join first: netclient join -t \$NETMAKER_TOKEN"
    exit 1
fi

echo -e "${GREEN}✓${NC} Host joined to netmaker"

# Find and add netmaker interfaces
echo ""
echo "Scanning for netmaker interfaces (nm-*)..."

FOUND_INTERFACES=false
for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-")) | .ifname'); do
    FOUND_INTERFACES=true

    # Check if already added
    if sudo ovs-vsctl list-ports mesh 2>/dev/null | grep -q "^${iface}$"; then
        echo -e "${GREEN}✓${NC} $iface already in mesh bridge"
    else
        echo "Adding $iface to mesh bridge..."
        if sudo ovs-vsctl add-port mesh "$iface"; then
            echo -e "${GREEN}✓${NC} Added $iface to mesh bridge"
        else
            echo -e "${RED}✗${NC} Failed to add $iface"
        fi
    fi
done

if [ "$FOUND_INTERFACES" = false ]; then
    echo -e "${YELLOW}⚠${NC}  No netmaker interfaces found"
    echo "Check if netclient created wireguard interfaces"
    exit 1
fi

echo ""
echo "=== Current mesh bridge configuration ==="
echo "Ports on mesh bridge:"
sudo ovs-vsctl list-ports mesh | sed 's/^/  - /'

echo ""
echo -e "${GREEN}✓${NC} Sync complete"
