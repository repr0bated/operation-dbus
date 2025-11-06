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
PartOf=openvswitch-switch.service

[Service]
Type=simple
ExecStart=/usr/local/bin/op-dbus --state-file /etc/op-dbus/state.json run
ExecStartPost=/bin/sleep 2
ExecStartPost=/usr/local/bin/op-dbus restore-flows
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

# Container setup with socket networking
echo ""
echo "=========================="
echo "📦 Container Setup (Optional)"
echo "=========================="

read -rp "Create test container with socket networking? [y/N]: " CREATE_CONTAINER
if [[ "$CREATE_CONTAINER" =~ ^[Yy]$ ]]; then
    read -rp "Container ID [101]: " CONTAINER_ID
    CONTAINER_ID="${CONTAINER_ID:-101}"

    read -rp "Container hostname [socket-test-${CONTAINER_ID}]: " CONTAINER_HOSTNAME
    CONTAINER_HOSTNAME="${CONTAINER_HOSTNAME:-socket-test-${CONTAINER_ID}}"

    read -rp "Storage pool [local-btrfs]: " STORAGE_POOL
    STORAGE_POOL="${STORAGE_POOL:-local-btrfs}"

    read -rp "Template [local:vztmpl/debian-12-standard_12.12-1_amd64.tar.zst]: " TEMPLATE
    TEMPLATE="${TEMPLATE:-local:vztmpl/debian-12-standard_12.12-1_amd64.tar.zst}"

    read -rp "Bridge for socket networking [mesh]: " SOCKET_BRIDGE
    SOCKET_BRIDGE="${SOCKET_BRIDGE:-mesh}"

    echo ""
    echo "🔨 Creating container $CONTAINER_ID..."

    # Create container
    if pct status "$CONTAINER_ID" >/dev/null 2>&1; then
        echo "⚠️  Container $CONTAINER_ID already exists, skipping creation"
    else
        pct create "$CONTAINER_ID" "$TEMPLATE" \
            --hostname "$CONTAINER_HOSTNAME" \
            --memory 512 \
            --swap 512 \
            --rootfs "${STORAGE_POOL}:8" \
            --unprivileged 1 \
            --features nesting=1 \
            --onboot 0
        echo "✅ Container created"
    fi

    # Create socket port
    SOCKET_PORT="internal_${CONTAINER_ID}"
    echo "🔨 Creating socket port $SOCKET_PORT on bridge $SOCKET_BRIDGE..."

    if ovs-vsctl list-ports "$SOCKET_BRIDGE" | grep -q "^${SOCKET_PORT}$"; then
        echo "ℹ️  Socket port already exists"
    else
        ovs-vsctl add-port "$SOCKET_BRIDGE" "$SOCKET_PORT" -- set interface "$SOCKET_PORT" type=internal
        echo "✅ Socket port created"
    fi

    # Get OpenFlow port number
    sleep 1
    OFPORT=$(ovs-ofctl show "$SOCKET_BRIDGE" | grep -A1 "$SOCKET_PORT" | grep -oP '^\s*\K\d+' || echo "")

    if [ -n "$OFPORT" ]; then
        echo "✅ Socket port OpenFlow number: $OFPORT"

        # Add OpenFlow rules
        echo "🔨 Adding OpenFlow flows for container $CONTAINER_ID..."

        # Ingress: match on socket port, load container ID into reg0, resubmit to table 1
        ovs-ofctl add-flow "$SOCKET_BRIDGE" \
            "table=0,priority=200,in_port=${OFPORT},actions=load:${CONTAINER_ID}->NXM_NX_REG0[],resubmit(,1)"

        # Egress: match reg0, output to LOCAL
        ovs-ofctl add-flow "$SOCKET_BRIDGE" \
            "table=1,priority=100,reg0=${CONTAINER_ID},actions=output:LOCAL"

        echo "✅ OpenFlow flows added"

        # Show flows
        echo ""
        echo "Flows for container $CONTAINER_ID:"
        ovs-ofctl dump-flows "$SOCKET_BRIDGE" | grep -E "(in_port=${OFPORT}|reg0=${CONTAINER_ID})"
    else
        echo "⚠️  Could not determine OpenFlow port number"
    fi

    # Netmaker enrollment
    echo ""
    read -rp "Configure Netmaker enrollment? [y/N]: " SETUP_NETMAKER
    if [[ "$SETUP_NETMAKER" =~ ^[Yy]$ ]]; then
        read -rp "Netmaker enrollment token: " NETMAKER_TOKEN

        if [ -n "$NETMAKER_TOKEN" ]; then
            echo "🔧 Configuring Netmaker enrollment..."

            # Create netmaker directory in container
            CONTAINER_ROOT="/var/lib/lxc/${CONTAINER_ID}/rootfs"
            mkdir -p "${CONTAINER_ROOT}/etc/netmaker"

            # Write token
            echo "$NETMAKER_TOKEN" > "${CONTAINER_ROOT}/etc/netmaker/enrollment-token"

            # Copy firstboot script
            FIRSTBOOT_SCRIPT="/git/operation-dbus/netmaker-firstboot.sh"
            if [ -f "$FIRSTBOOT_SCRIPT" ]; then
                cp "$FIRSTBOOT_SCRIPT" "${CONTAINER_ROOT}/usr/local/bin/netmaker-firstboot.sh"
                chmod +x "${CONTAINER_ROOT}/usr/local/bin/netmaker-firstboot.sh"

                # Create systemd oneshot service for firstboot
                cat > "${CONTAINER_ROOT}/etc/systemd/system/netmaker-firstboot.service" <<'FIRSTBOOT_EOF'
[Unit]
Description=Netmaker First Boot Setup
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/etc/netmaker/.enrollment-complete

[Service]
Type=oneshot
ExecStart=/usr/local/bin/netmaker-firstboot.sh
ExecStartPost=/usr/bin/touch /etc/netmaker/.enrollment-complete
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
FIRSTBOOT_EOF

                # Enable the service (will run on first boot)
                if [ -d "${CONTAINER_ROOT}/etc/systemd/system/multi-user.target.wants" ]; then
                    mkdir -p "${CONTAINER_ROOT}/etc/systemd/system/multi-user.target.wants"
                fi
                ln -sf /etc/systemd/system/netmaker-firstboot.service \
                    "${CONTAINER_ROOT}/etc/systemd/system/multi-user.target.wants/netmaker-firstboot.service"

                echo "✅ Netmaker enrollment configured"
                echo "ℹ️  Container will enroll on first boot"
            else
                echo "⚠️  Firstboot script not found: $FIRSTBOOT_SCRIPT"
            fi
        else
            echo "⚠️  No token provided, skipping Netmaker setup"
        fi
    fi

    echo ""
    echo "✅ Container setup complete"
    echo ""
    echo "Container details:"
    echo "  ID: $CONTAINER_ID"
    echo "  Socket port: $SOCKET_PORT (ofport: $OFPORT)"
    echo "  Bridge: $SOCKET_BRIDGE"
    echo ""
    echo "To start: pct start $CONTAINER_ID"
    echo "To access: pct enter $CONTAINER_ID"
fi

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
