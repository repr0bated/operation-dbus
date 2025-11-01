#!/bin/bash
# Sync netmaker interfaces to mesh bridge
# Run this after joining netmaker to automatically connect interfaces

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

OVSDB_SOCK="/var/run/openvswitch/db.sock"

# OVSDB helper (no ovs-vsctl!)
ovsdb_bridge_exists() {
    local bridge="$1"
    local result=$(echo "{\"method\":\"transact\",\"params\":[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$bridge\"]],\"columns\":[\"name\"]}]],\"id\":0}" | socat - UNIX-CONNECT:"$OVSDB_SOCK" 2>/dev/null)
    local count=$(echo "$result" | jq -r '.result[0].rows | length' 2>/dev/null)
    [ "$count" -gt 0 ]
}

ovsdb_list_ports() {
    local bridge="$1"
    local bridge_result=$(echo "{\"method\":\"transact\",\"params\":[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$bridge\"]],\"columns\":[\"ports\"]}]],\"id\":0}" | socat - UNIX-CONNECT:"$OVSDB_SOCK" 2>/dev/null)
    local port_uuids=$(echo "$bridge_result" | jq -r '.result[0].rows[0].ports[1][]?[1]' 2>/dev/null)
    
    for port_uuid in $port_uuids; do
        local port_result=$(echo "{\"method\":\"transact\",\"params\":[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Port\",\"where\":[[\"_uuid\",\"==\",[\"uuid\",\"$port_uuid\"]]],\"columns\":[\"name\"]}]],\"id\":0}" | socat - UNIX-CONNECT:"$OVSDB_SOCK" 2>/dev/null)
        echo "$port_result" | jq -r '.result[0].rows[].name' 2>/dev/null
    done
}

echo "=== Netmaker Mesh Bridge Sync ==="

# Check if mesh bridge exists
if ! ovsdb_bridge_exists mesh; then
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
if ! netclient list >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC}  Host not joined to netmaker network"
    echo "Join first: netclient join -t \$NETMAKER_TOKEN"
    exit 1
fi

# Check if netclient returned any networks (empty array or null means not joined)
NETWORK_COUNT=$(netclient list 2>/dev/null | jq -r 'if type == "array" then length else 0 end' || echo "0")
if [ "$NETWORK_COUNT" = "0" ]; then
    echo -e "${YELLOW}⚠${NC}  Host not joined to netmaker network"
    echo "Join first: netclient join -t \$NETMAKER_TOKEN"
    exit 1
fi

echo -e "${GREEN}✓${NC} Host joined to netmaker"

# Find and add netmaker interfaces
echo ""
echo "Scanning for netmaker interfaces (nm-* or netmaker)..."

FOUND_INTERFACES=false
# Look for interfaces starting with nm- or exact name "netmaker"
for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-") or . == "netmaker") | .ifname'); do
    FOUND_INTERFACES=true

    # Check if already added
    if ovsdb_list_ports mesh 2>/dev/null | grep -q "^${iface}$"; then
        echo -e "${GREEN}✓${NC} $iface already in mesh bridge"
    else
        echo "Adding $iface to mesh bridge..."
        echo -e "${YELLOW}⚠${NC}  Port addition via native OVSDB not yet implemented in this script"
        echo -e "${YELLOW}⚠${NC}  Use op-dbus CLI once implemented"
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
ovsdb_list_ports mesh | sed 's/^/  - /'

echo ""
echo -e "${GREEN}✓${NC} Sync complete"
