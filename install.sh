#!/bin/bash
# install.sh — OVS Bridge Installation with prompts
set -euo pipefail

echo "🎯 OVS Bridge Installation"
echo "=========================="

# Check root
[ "$EUID" -eq 0 ] || { echo "❌ Run as root"; exit 1; }

# Interactive prompts
read -rp "Bridges to create [ovsbr0,mesh]: " BRIDGES_INPUT
BRIDGES="${BRIDGES_INPUT:-ovsbr0,mesh}"

read -rp "OpenFlow controller [tcp:127.0.0.1:6653]: " OF_INPUT
OF_TARGET="${OF_INPUT:-tcp:127.0.0.1:6653}"

# Start OVS
echo "🔧 Starting OVS services..."
systemctl start ovsdb-server ovs-vswitchd 2>/dev/null || true
sleep 2

# Check OVS
if ! command -v ovs-vsctl >/dev/null 2>&1; then
    echo "❌ ovs-vsctl not found. Install: apt install openvswitch-switch"
    exit 1
fi

if ! ovs-vsctl show >/dev/null 2>&1; then
    echo "⚠️ OVS not responding. Restarting..."
    systemctl restart ovsdb-server ovs-vswitchd
    sleep 3
    if ! ovs-vsctl show >/dev/null 2>&1; then
        echo "❌ OVS still not working"
        exit 1
    fi
fi

echo "✅ OVS is ready"

# Create bridges
echo ""
echo "📦 Creating bridges: ${BRIDGES}"
echo "🎮 Controller: $OF_TARGET"
echo "=========================="

IFS=',' read -r -a BRIDGE_ARRAY <<< "$BRIDGES"
for BRIDGE in "${BRIDGE_ARRAY[@]}"; do
    BRIDGE="${BRIDGE//[[:space:]]/}"
    [ -z "$BRIDGE" ] && continue
    
    echo ""
    echo "--- 🔨 $BRIDGE ---"
    
    # Create bridge
    if ovs-vsctl list-br | grep -q "^$BRIDGE$"; then
        echo "✅ Exists - updating"
        ovs-vsctl set bridge "$BRIDGE" datapath_type=system
    else
        echo "🆕 Creating"
        ovs-vsctl add-br "$BRIDGE" -- set bridge "$BRIDGE" datapath_type=system
    fi
    
    # Set controller
    ovs-vsctl set-controller "$BRIDGE" "$OF_TARGET"
    
    # Explicitly turn off STP (Spanning Tree Protocol)
    echo "🔧 Disabling STP"
    ovs-vsctl set bridge "$BRIDGE" stp_enable=false
    
    # Check kernel
    echo "🔍 Checking kernel..."
    for i in {1..5}; do
        if ip link show "$BRIDGE" >/dev/null 2>&1; then
            echo "✅ Kernel visible"
            break
        fi
        sleep 1
    done
done

# Final status
echo ""
echo "=========================="
echo "📊 FINAL STATUS"
echo "=========================="

echo "Bridges:"
ovs-vsctl list-br

echo ""
echo "Kernel interfaces:"
ip link show | grep -E "$(echo "${BRIDGE_ARRAY[@]}" | tr ' ' '|')" || echo "None found"

echo ""
echo "Controllers:"
for BRIDGE in "${BRIDGE_ARRAY[@]}"; do
    BRIDGE="${BRIDGE//[[:space:]]/}"
    [ -z "$BRIDGE" ] && continue
    echo -n "$BRIDGE: "
    ovs-vsctl get-controller "$BRIDGE" 2>/dev/null || echo "None"
done

echo ""
echo "STP Status:"
for BRIDGE in "${BRIDGE_ARRAY[@]}"; do
    BRIDGE="${BRIDGE//[[:space:]]/}"
    [ -z "$BRIDGE" ] && continue
    echo -n "$BRIDGE STP: "
    ovs-vsctl get bridge "$BRIDGE" stp_enable 2>/dev/null || echo "Not set"
done

echo ""
echo "=========================="
echo "✅ INSTALLATION COMPLETE"
echo "=========================="
