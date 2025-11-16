#!/bin/bash
# create-ovs-bridge.sh - Create OVS bridge directly in OVSDB

set -euo pipefail

BRIDGE_NAME="${1:-ovsbr0}"
IP_ADDRESS="${2:-10.0.0.1/24}"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Creating OVS Bridge: $BRIDGE_NAME"
echo "  IP Address: $IP_ADDRESS"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Error: Must run as root"
    exit 1
fi

# Install OpenVSwitch if not present
if ! command -v ovs-vsctl &> /dev/null; then
    echo "Installing OpenVSwitch..."
    apt-get update
    apt-get install -y openvswitch-switch
    echo "✓ OpenVSwitch installed"
    echo ""
fi

# Start OVS services
echo "Starting OVS services..."
systemctl enable --now openvswitch-switch
systemctl start ovsdb-server 2>/dev/null || true
systemctl start ovs-vswitchd 2>/dev/null || true
sleep 2
echo "✓ OVS services running"
echo ""

# Check if bridge already exists
if ovs-vsctl br-exists "$BRIDGE_NAME" 2>/dev/null; then
    echo "⚠️  Bridge $BRIDGE_NAME already exists"
    ovs-vsctl show
    exit 0
fi

# Create bridge
echo "Creating OVS bridge..."
ovs-vsctl add-br "$BRIDGE_NAME" -- set bridge "$BRIDGE_NAME" datapath_type=system
echo "✓ Bridge $BRIDGE_NAME created in OVSDB"
echo ""

# Bring interface up
echo "Bringing interface up..."
ip link set "$BRIDGE_NAME" up
echo "✓ Interface $BRIDGE_NAME up"
echo ""

# Add IP address if specified
if [ -n "$IP_ADDRESS" ] && [ "$IP_ADDRESS" != "none" ]; then
    echo "Adding IP address $IP_ADDRESS..."
    ip addr add "$IP_ADDRESS" dev "$BRIDGE_NAME" 2>/dev/null || {
        echo "⚠️  IP address may already exist"
    }
    echo "✓ IP address configured"
    echo ""
fi

# Update /etc/network/interfaces for persistence
echo "Updating /etc/network/interfaces..."
INTERFACES_FILE="/etc/network/interfaces"

# Extract IP and netmask from CIDR
IFS='/' read -r IP PREFIX <<< "$IP_ADDRESS"
NETMASK=$(python3 -c "import ipaddress; print(ipaddress.IPv4Network('0.0.0.0/$PREFIX', strict=False).netmask)" 2>/dev/null || echo "255.255.255.0")

# Check if our block already exists
if ! grep -q "# BEGIN op-dbus-managed" "$INTERFACES_FILE" 2>/dev/null; then
    cat >> "$INTERFACES_FILE" <<EOF

# BEGIN op-dbus-managed
# Managed by op-dbus. Do not edit manually.

allow-ovs $BRIDGE_NAME
iface $BRIDGE_NAME inet static
    address $IP
    netmask $NETMASK
    ovs_type OVSBridge

# END op-dbus-managed
EOF
    echo "✓ Network interfaces file updated"
else
    echo "✓ Network interfaces already configured"
fi
echo ""

# Show final state
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✓ OVS Bridge Created Successfully!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

echo "OVSDB State:"
ovs-vsctl show
echo ""

echo "Interface State:"
ip addr show "$BRIDGE_NAME"
echo ""

echo "Next steps:"
echo "  - Add ports: ovs-vsctl add-port $BRIDGE_NAME eth0"
echo "  - View OVSDB: ovs-vsctl show"
echo "  - Apply state: cargo run --bin operation-dbus apply states/network-ovs-bridge.json"
echo ""
