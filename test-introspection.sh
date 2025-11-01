#!/bin/bash
# Test the network introspection without installing
# This demonstrates what the install script will detect

set -e

echo "=== Testing Network Introspection ==="
echo ""

if ! command -v jq >/dev/null 2>&1; then
    echo "Error: jq is required for introspection"
    exit 1
fi

if [ ! -S "/var/run/openvswitch/db.sock" ]; then
    echo "Warning: OVSDB socket not found, OVS bridges won't be detected"
fi

echo "Detecting current network configuration..."
echo ""

# Source the introspection function from test script
source /tmp/test-introspect.sh

introspect_network 2>&1 | head -n -1  # Remove the last line (the JSON)
echo ""
echo "=== Generated Configuration ==="
introspect_network 2>/dev/null | jq .

echo ""
echo "=== Configuration Summary ==="
CONFIG=$(introspect_network 2>/dev/null)
IFACE_COUNT=$(echo "$CONFIG" | jq '.plugins.net.interfaces | length')
echo "Network interfaces detected: $IFACE_COUNT"

if [ "$IFACE_COUNT" -gt 0 ]; then
    echo "$CONFIG" | jq -r '.plugins.net.interfaces[] | "  - \(.name) (\(.type)): \(.ipv4.address[0].ip)/\(.ipv4.address[0].prefix) via \(.ipv4.gateway // "no gateway")"'
fi

echo ""
echo "This configuration will be automatically generated when you run:"
echo "  sudo ./install.sh"
