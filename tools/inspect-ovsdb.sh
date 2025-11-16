#!/bin/bash
# inspect-ovsdb.sh - Inspect OVSDB state and configuration

set -euo pipefail

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  OVSDB Inspection"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if OVS is installed
if ! command -v ovs-vsctl &> /dev/null; then
    echo "❌ OpenVSwitch not installed"
    echo "   Install with: apt install openvswitch-switch"
    exit 1
fi

# Check if OVSDB is running
if ! systemctl is-active --quiet ovsdb-server 2>/dev/null; then
    echo "❌ OVSDB server not running"
    echo "   Start with: systemctl start ovsdb-server"
    exit 1
fi

echo "✓ OVSDB server is running"
echo ""

# Show OVS version
echo "OVS Version:"
ovs-vsctl --version | head -1
echo ""

# List databases
echo "Available Databases:"
ovsdb-client list-dbs unix:/var/run/openvswitch/db.sock 2>/dev/null || echo "  (Unable to query)"
echo ""

# Show all bridges
echo "━━━ Bridges ━━━"
if ovs-vsctl list-br 2>/dev/null | grep -q .; then
    for bridge in $(ovs-vsctl list-br); do
        echo "Bridge: $bridge"
        echo "  Datapath: $(ovs-vsctl get bridge "$bridge" datapath_type 2>/dev/null || echo 'unknown')"
        echo "  Ports:"
        ovs-vsctl list-ports "$bridge" 2>/dev/null | sed 's/^/    - /' || echo "    (none)"

        # Show controller if configured
        CONTROLLER=$(ovs-vsctl get-controller "$bridge" 2>/dev/null || echo "")
        if [ -n "$CONTROLLER" ]; then
            echo "  Controller: $CONTROLLER"
        fi
        echo ""
    done
else
    echo "  No bridges configured"
    echo ""
fi

# Show full OVSDB structure
echo "━━━ Full OVSDB State ━━━"
ovs-vsctl show
echo ""

# Check database file
echo "━━━ Database Files ━━━"
DB_DIR="/etc/openvswitch"
if [ -d "$DB_DIR" ]; then
    echo "Database directory: $DB_DIR"
    ls -lh "$DB_DIR"/*.db 2>/dev/null || echo "  No .db files found"
else
    echo "  Database directory not found"
fi
echo ""

# Show interface states
echo "━━━ OVS Interface States ━━━"
if ovs-vsctl list-br 2>/dev/null | grep -q .; then
    for bridge in $(ovs-vsctl list-br); do
        if ip link show "$bridge" &>/dev/null; then
            echo "$bridge:"
            ip addr show "$bridge" | grep -E "^\s*(inet|link)" | sed 's/^/  /'
        fi
    done
else
    echo "  No OVS bridges to show"
fi
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Inspection Complete"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
