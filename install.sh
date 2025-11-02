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

# Create systemd service file
echo ""
echo "=========================="
echo "📦 Creating systemd service"
echo "=========================="

SYSTEMD_DIR="/etc/systemd/system"

# Check if op-dbus binary exists
if [ ! -f "/usr/local/bin/op-dbus" ]; then
    echo "⚠️  op-dbus binary not found at /usr/local/bin/op-dbus"
    echo "⚠️  Service file will be created but service won't work until binary is installed"
fi

# Check if service file already exists
if [ -f "$SYSTEMD_DIR/op-dbus.service" ]; then
    echo "ℹ️  Service file already exists, updating..."
else
    echo "🆕 Creating new service file..."
fi

# Create op-dbus.service file
cat > "$SYSTEMD_DIR/op-dbus.service" <<'SERVICE_EOF'
[Unit]
Description=op-dbus - Declarative system state management
Documentation=https://github.com/ghostbridge/op-dbus
After=network-online.target openvswitch-switch.service
Wants=network-online.target
Requires=openvswitch-switch.service

[Service]
Type=simple
ExecStart=/usr/local/bin/op-dbus run --state-file /etc/op-dbus/state.json
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=false
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/etc/network/interfaces /run /var/run /etc/dnsmasq.d

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
SERVICE_EOF

echo "✅ Created: $SYSTEMD_DIR/op-dbus.service"

# Reload systemd
echo "🔄 Reloading systemd..."
systemctl daemon-reload
echo "✅ Systemd reloaded"

# Enable services for boot
echo ""
echo "=========================="
echo "🔧 Enabling services for boot"
echo "=========================="

# Enable openvswitch-switch.service (idempotent - safe to run multiple times)
if systemctl is-enabled openvswitch-switch.service >/dev/null 2>&1; then
    echo "ℹ️  openvswitch-switch.service already enabled"
else
    if systemctl enable openvswitch-switch.service 2>/dev/null; then
        echo "✅ Enabled: openvswitch-switch.service"
    else
        echo "❌ Failed to enable openvswitch-switch.service"
    fi
fi

# Enable op-dbus.service (idempotent - safe to run multiple times)
if systemctl is-enabled op-dbus.service >/dev/null 2>&1; then
    echo "ℹ️  op-dbus.service already enabled"
else
    if systemctl enable op-dbus.service 2>/dev/null; then
        echo "✅ Enabled: op-dbus.service"
    else
        echo "⚠️  Failed to enable op-dbus.service (binary may be missing)"
    fi
fi

# Verify services are enabled
echo ""
echo "=========================="
echo "🔍 Service Status"
echo "=========================="

if systemctl is-enabled openvswitch-switch.service >/dev/null 2>&1; then
    echo "✅ openvswitch-switch.service: $(systemctl is-enabled openvswitch-switch.service)"
else
    echo "❌ openvswitch-switch.service: not enabled"
fi

if systemctl is-enabled op-dbus.service >/dev/null 2>&1; then
    echo "✅ op-dbus.service: $(systemctl is-enabled op-dbus.service)"
else
    echo "❌ op-dbus.service: not enabled"
fi

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
echo ""
echo "ℹ️  This script is idempotent - safe to run multiple times"
echo "ℹ️  Services will start automatically at boot"
echo ""
echo "To verify services after reboot:"
echo "  systemctl status openvswitch-switch"
echo "  systemctl status op-dbus"
echo "  ovs-vsctl show"
