#!/bin/bash
# op-dbus installation script
# Installs binary, creates config directories, sets up systemd service

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== op-dbus Installation ===${NC}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./install.sh)${NC}"
    exit 1
fi

# Variables
BINARY_PATH="${1:-target/release/op-dbus}"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/op-dbus"
STATE_FILE="$CONFIG_DIR/state.json"
SYSTEMD_DIR="/etc/systemd/system"

# Step 1: Check binary exists
echo "Checking binary..."
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    echo "Build first with: cargo build --release"
    exit 1
fi
echo -e "${GREEN}✓${NC} Found binary: $BINARY_PATH"

# Step 2: Install binary
echo "Installing binary to $INSTALL_DIR..."
cp "$BINARY_PATH" "$INSTALL_DIR/op-dbus"
chmod +x "$INSTALL_DIR/op-dbus"
echo -e "${GREEN}✓${NC} Installed: $INSTALL_DIR/op-dbus"

# Step 3: Create config directory
echo "Creating config directory..."
mkdir -p "$CONFIG_DIR"
echo -e "${GREEN}✓${NC} Created: $CONFIG_DIR"

# Step 4: Install example state file if doesn't exist
if [ ! -f "$STATE_FILE" ]; then
    if [ -f "example-state.json" ]; then
        echo "Installing example state file..."
        cp example-state.json "$STATE_FILE"
        echo -e "${YELLOW}⚠${NC}  Installed example state to $STATE_FILE"
        echo -e "${YELLOW}⚠${NC}  IMPORTANT: Edit $STATE_FILE before starting service!"
    else
        echo "Creating minimal state file..."
        cat > "$STATE_FILE" <<'EOF'
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": []
    },
    "systemd": {
      "units": {}
    }
  }
}
EOF
        echo -e "${YELLOW}⚠${NC}  Created minimal state file at $STATE_FILE"
    fi
else
    echo -e "${GREEN}✓${NC} State file already exists: $STATE_FILE"
fi

# Step 5: Create systemd service
echo "Creating systemd service..."
cat > "$SYSTEMD_DIR/op-dbus.service" <<'EOF'
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
ReadWritePaths=/etc/network/interfaces /run /var/run

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF
echo -e "${GREEN}✓${NC} Created: $SYSTEMD_DIR/op-dbus.service"

# Step 6: Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload
echo -e "${GREEN}✓${NC} Systemd reloaded"

# Step 7: Show installation summary
echo ""
echo -e "${GREEN}=== Installation Complete ===${NC}"
echo ""
echo "Binary:        $INSTALL_DIR/op-dbus"
echo "Config:        $CONFIG_DIR/state.json"
echo "Service:       $SYSTEMD_DIR/op-dbus.service"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "1. Edit state file:    nano $STATE_FILE"
echo "2. Test query:         op-dbus query"
echo "3. Test diff:          op-dbus diff $STATE_FILE"
echo "4. Test apply (safe):  op-dbus apply $STATE_FILE"
echo "5. Enable service:     systemctl enable op-dbus"
echo "6. Start service:      systemctl start op-dbus"
echo "7. Check status:       systemctl status op-dbus"
echo "8. View logs:          journalctl -u op-dbus -f"
echo ""
echo -e "${YELLOW}⚠  WARNING:${NC} Test manually before enabling service!"
echo -e "${YELLOW}⚠  WARNING:${NC} Network changes can cause 20min downtime on failure!"
echo ""
