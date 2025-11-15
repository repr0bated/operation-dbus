#!/bin/bash
# install-portable.sh - Portable installation for any Linux system
set -euo pipefail

echo "═══════════════════════════════════════"
echo "  op-dbus - Portable Installation"
echo "═══════════════════════════════════════"
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ This script must be run as root"
    echo "   Please run: sudo $0"
    exit 1
fi

# Check for required binaries
echo "🔍 Checking system requirements..."

# Check if binary exists
if [ ! -f "target/release/op-dbus" ]; then
    echo "❌ op-dbus binary not found"
    echo "   Please build first: cargo build --release"
    exit 1
fi

# Check for systemd
if ! command -v systemctl &> /dev/null; then
    echo "❌ systemd not found - this tool requires systemd"
    exit 1
fi
echo "  ✅ systemd found"

# Check for D-Bus
if [ ! -S /var/run/dbus/system_bus_socket ]; then
    echo "❌ D-Bus system bus not found"
    echo "   Please install: apt install dbus"
    exit 1
fi
echo "  ✅ D-Bus found"

# Optional: Check for OVS (not required)
if command -v ovs-vsctl &> /dev/null; then
    echo "  ✅ OpenVSwitch found (optional)"
    HAS_OVS=true
else
    echo "  ℹ️  OpenVSwitch not found (optional - some features disabled)"
    HAS_OVS=false
fi

echo

# Install binary
echo "📦 Installing binary..."
install -m 755 target/release/op-dbus /usr/local/bin/op-dbus
echo "  ✅ Installed to /usr/local/bin/op-dbus"

# Create configuration directory
echo
echo "📁 Creating configuration directory..."
mkdir -p /etc/op-dbus
chmod 755 /etc/op-dbus
echo "  ✅ Created /etc/op-dbus"

# Create data directory for blockchain
echo
echo "📁 Creating data directory..."
mkdir -p /var/lib/op-dbus/blockchain/{timing,vectors,snapshots}
chmod 700 /var/lib/op-dbus
echo "  ✅ Created /var/lib/op-dbus"

# Create runtime directory
echo
echo "📁 Creating runtime directory..."
mkdir -p /run/op-dbus
chmod 755 /run/op-dbus
echo "  ✅ Created /run/op-dbus"

# Generate initial state file by introspecting the system
echo
echo "🔍 Introspecting current system state..."
if /usr/local/bin/op-dbus init --introspect --output /etc/op-dbus/state.json 2>/dev/null; then
    echo "  ✅ Generated /etc/op-dbus/state.json"
    echo "  ℹ️  This captures your current system configuration"
else
    # If introspection fails, create a minimal template
    echo "  ⚠️  Introspection failed, creating minimal template"
    cat > /etc/op-dbus/state.json <<'EOF'
{
  "version": 1,
  "plugins": {
    "systemd": {
      "units": {}
    }
  }
}
EOF
    echo "  ✅ Created minimal /etc/op-dbus/state.json"
fi

# Create systemd service
echo
echo "📝 Creating systemd service..."
cat > /etc/systemd/system/op-dbus.service <<'EOF'
[Unit]
Description=op-dbus - Declarative system state management
Documentation=https://github.com/ghostbridge/op-dbus
After=network-online.target dbus.service
Wants=network-online.target

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
ReadWritePaths=/var/lib/op-dbus /run/op-dbus

# Capabilities for network management (if OVS is used)
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF

echo "  ✅ Created /etc/systemd/system/op-dbus.service"

# Reload systemd
echo
echo "🔄 Reloading systemd..."
systemctl daemon-reload
echo "  ✅ Systemd reloaded"

# Display usage information
echo
echo "═══════════════════════════════════════"
echo "  ✅ Installation Complete!"
echo "═══════════════════════════════════════"
echo
echo "📋 Quick Start:"
echo
echo "  1. View current system state:"
echo "     op-dbus query"
echo
echo "  2. Edit desired state:"
echo "     nano /etc/op-dbus/state.json"
echo
echo "  3. Preview changes:"
echo "     op-dbus diff /etc/op-dbus/state.json"
echo
echo "  4. Apply changes:"
echo "     op-dbus apply /etc/op-dbus/state.json"
echo
echo "  5. Enable automatic state management:"
echo "     systemctl enable op-dbus"
echo "     systemctl start op-dbus"
echo
echo "📚 Documentation:"
echo "  - Run: op-dbus --help"
echo "  - Check system: op-dbus doctor"
echo "  - View blockchain: op-dbus blockchain list"
echo
echo "⚠️  Note: op-dbus service is NOT started automatically"
echo "   Test manually first, then enable if desired"
echo
