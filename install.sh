#!/bin/bash
# op-dbus installation script
# Installs binary, creates config directories, sets up systemd service

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Show help if requested
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "op-dbus Installation Script"
    echo ""
    echo "Usage: $0 [BINARY_PATH] [DHCP_SERVER_FLAG]"
    echo ""
    echo "Arguments:"
    echo "  BINARY_PATH       Path to op-dbus binary (default: target/release/op-dbus)"
    echo "  DHCP_SERVER_FLAG  'true' to enable DHCP server setup (default: false)"
    echo ""
    echo "Examples:"
    echo "  $0                           # Standard install"
    echo "  $0 target/release/op-dbus    # Custom binary path"
    echo "  $0 target/release/op-dbus true  # Enable DHCP server"
    echo ""
    echo "The install script detects and manages existing OVS bridges."
    exit 0
fi

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
ENABLE_DHCP_SERVER="${2:-false}"

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

# Step 3.5: Setup BTRFS subvolumes for blockchain storage (if on BTRFS)
BLOCKCHAIN_DIR="/var/lib/op-dbus/blockchain"
echo "Setting up blockchain storage..."

# Check if we're on BTRFS
if df -T /var/lib 2>/dev/null | grep -q btrfs; then
    echo "Detected BTRFS filesystem, setting up subvolumes..."

    # Check for existing op-dbus blockchain subvolumes
    EXISTING_SUBVOLS=$(sudo btrfs subvolume list / 2>/dev/null | grep -E "@var/lib/op-dbus/blockchain|@blockchain/op-dbus" || true)

    if [ -n "$EXISTING_SUBVOLS" ]; then
        echo -e "${YELLOW}⚠${NC}  Found existing blockchain subvolumes:"
        echo "$EXISTING_SUBVOLS"

        # Ask user what to do
        if [ -t 0 ]; then
            read -p "Reuse existing subvolumes? [Y/n] " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Yy]$ ]] && [[ -n $REPLY ]]; then
                echo "Cleaning up old subvolumes..."

                # Delete old blockchain data
                if [ -d "$BLOCKCHAIN_DIR" ]; then
                    sudo rm -rf "$BLOCKCHAIN_DIR"/*
                    echo -e "${GREEN}✓${NC} Cleared old blockchain data"
                fi
            else
                echo -e "${GREEN}✓${NC} Reusing existing blockchain subvolumes"
            fi
        else
            # Non-interactive: reuse existing
            echo -e "${GREEN}✓${NC} Reusing existing blockchain subvolumes (non-interactive mode)"
        fi
    fi

    # Create base directory
    sudo mkdir -p "$BLOCKCHAIN_DIR"

    # Create subvolumes if they don't exist
    if ! sudo btrfs subvolume show "$BLOCKCHAIN_DIR" >/dev/null 2>&1; then
        # Check if it's already a regular directory with files
        if [ -d "$BLOCKCHAIN_DIR" ] && [ "$(ls -A $BLOCKCHAIN_DIR 2>/dev/null)" ]; then
            echo -e "${YELLOW}⚠${NC}  $BLOCKCHAIN_DIR exists as regular directory with files"
            echo -e "${YELLOW}⚠${NC}  Converting to BTRFS subvolume..."

            # Move data temporarily
            TEMP_BACKUP="/tmp/op-dbus-blockchain-backup-$$"
            sudo mv "$BLOCKCHAIN_DIR" "$TEMP_BACKUP"
            sudo mkdir -p "$(dirname $BLOCKCHAIN_DIR)"

            # Create subvolume
            sudo btrfs subvolume create "$BLOCKCHAIN_DIR"

            # Restore data
            sudo mv "$TEMP_BACKUP"/* "$BLOCKCHAIN_DIR/" 2>/dev/null || true
            sudo rm -rf "$TEMP_BACKUP"

            echo -e "${GREEN}✓${NC} Converted to BTRFS subvolume with data preserved"
        else
            # Create fresh subvolume
            sudo btrfs subvolume create "$BLOCKCHAIN_DIR"
            echo -e "${GREEN}✓${NC} Created blockchain BTRFS subvolume"
        fi
    else
        echo -e "${GREEN}✓${NC} Blockchain subvolume already exists"
    fi

    # Set permissions
    sudo chown -R root:root "$BLOCKCHAIN_DIR"
    sudo chmod 755 "$BLOCKCHAIN_DIR"

else
    # Not BTRFS, just use regular directory
    echo "Using regular directory (not BTRFS)"
    sudo mkdir -p "$BLOCKCHAIN_DIR"
    echo -e "${GREEN}✓${NC} Created blockchain directory: $BLOCKCHAIN_DIR"
fi

# Step 4: Install example state file if doesn't exist
if [ ! -f "$STATE_FILE" ]; then
    echo "Introspecting current network configuration..."

    # Function to introspect network configuration
    introspect_network() {
        local config='{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": []
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "active_state": "active",
          "enabled": true
        }
      }
    }
  }
}'

        # Check if OVS is available
        if command -v ovs-vsctl >/dev/null 2>&1; then
            # Get OVS bridge information
            local bridges=$(ovs-vsctl list-br 2>/dev/null || echo "")

            if [ -n "$bridges" ]; then
                echo -e "${GREEN}✓${NC} Found OVS bridges: $bridges"

                # For each bridge, get its configuration
                for bridge in $bridges; do
                    # Get ports (excluding the bridge itself)
                    local ports=$(ovs-vsctl list-ports "$bridge" 2>/dev/null | grep -v "^$bridge$" | tr '\n' ' ')

                    # Get IP configuration from ip command
                    local ip_info=$(ip -j addr show "$bridge" 2>/dev/null)

                    if [ -n "$ip_info" ]; then
                        # Extract IPv4 address and prefix
                        local ipv4_addr=$(echo "$ip_info" | jq -r '.[0].addr_info[] | select(.family=="inet") | .local' 2>/dev/null)
                        local ipv4_prefix=$(echo "$ip_info" | jq -r '.[0].addr_info[] | select(.family=="inet") | .prefixlen' 2>/dev/null)

                        # Get default gateway
                        local gateway=$(ip -j route show default 2>/dev/null | jq -r '.[0].gateway // empty' 2>/dev/null)

                        if [ -n "$ipv4_addr" ] && [ -n "$ipv4_prefix" ]; then
                            echo -e "${GREEN}✓${NC} Bridge $bridge: IP=$ipv4_addr/$ipv4_prefix, Gateway=$gateway, Ports=[$ports]"

                            # Build ports array for JSON
                            local ports_json="[]"
                            if [ -n "$ports" ]; then
                                ports_json=$(echo "$ports" | tr ' ' '\n' | grep -v '^$' | jq -R . | jq -s .)
                            fi

                            # Create interface configuration
                            local interface_config=$(cat <<EOF
{
  "name": "$bridge",
  "type": "ovs-bridge",
  "ports": $ports_json,
  "ipv4": {
    "enabled": true,
    "dhcp": false,
    "address": [
      {
        "ip": "$ipv4_addr",
        "prefix": $ipv4_prefix
      }
    ]$([ -n "$gateway" ] && echo ",
    \"gateway\": \"$gateway\"" || echo "")
  }
}
EOF
)
                            # Add interface to config using jq
                            config=$(echo "$config" | jq ".plugins.net.interfaces += [$interface_config]")
                        fi
                    fi
                done
            fi
        fi

        # Check if we found any interfaces
        local interface_count=$(echo "$config" | jq '.plugins.net.interfaces | length')

        if [ "$interface_count" -gt 0 ]; then
            echo -e "${GREEN}✓${NC} Generated configuration for $interface_count existing network interface(s)"
            echo "$config"
        else
            echo -e "${YELLOW}⚠${NC}  No existing OVS bridges found - will manage interfaces as they are added"
            echo "$config"
        fi
    }

    # Check if jq is available for introspection
    if command -v jq >/dev/null 2>&1; then
        echo "Generating state file from current system configuration..."
        INTROSPECTED_CONFIG=$(introspect_network)
        echo "$INTROSPECTED_CONFIG" > "$STATE_FILE"

        # Validate JSON
        if jq . "$STATE_FILE" >/dev/null 2>&1; then
            echo -e "${GREEN}✓${NC} Created introspected state file at $STATE_FILE"
            echo ""
            echo -e "${GREEN}Detected Configuration:${NC}"
            jq -C . "$STATE_FILE" 2>/dev/null || cat "$STATE_FILE"
            echo ""
            echo -e "${YELLOW}⚠${NC}  Review the generated configuration before starting the service"
        else
            echo -e "${RED}Error: Generated invalid JSON, falling back to minimal config${NC}"
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
        fi
    else
        echo -e "${YELLOW}⚠${NC}  jq not found, installing example state file"
        if [ -f "example-state.json" ]; then
            cp example-state.json "$STATE_FILE"
            echo -e "${YELLOW}⚠${NC}  Installed example state to $STATE_FILE"
            echo -e "${YELLOW}⚠${NC}  IMPORTANT: Edit $STATE_FILE before starting service!"
        else
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
    fi
else
    echo -e "${GREEN}✓${NC} State file already exists: $STATE_FILE"
fi

# Step 5: Create mesh bridge for netmaker containers
echo "Creating mesh bridge for netmaker containers..."

if command -v ovs-vsctl >/dev/null 2>&1; then
    if ! sudo ovs-vsctl br-exists mesh 2>/dev/null; then
        sudo ovs-vsctl add-br mesh
        sudo ip link set mesh up
        echo -e "${GREEN}✓${NC} Created 'mesh' bridge for netmaker containers"

        # Add to /etc/network/interfaces for persistence
        if [ -f /etc/network/interfaces ]; then
            if ! grep -q "^auto mesh" /etc/network/interfaces; then
                echo -e "\n# Netmaker mesh bridge" | sudo tee -a /etc/network/interfaces > /dev/null
                echo "auto mesh" | sudo tee -a /etc/network/interfaces > /dev/null
                echo "iface mesh inet manual" | sudo tee -a /etc/network/interfaces > /dev/null
                echo "    ovs_type OVSBridge" | sudo tee -a /etc/network/interfaces > /dev/null
                echo -e "${GREEN}✓${NC} Added mesh bridge to /etc/network/interfaces"
            fi
        fi
    else
        echo -e "${GREEN}✓${NC} 'mesh' bridge already exists"
    fi
else
    echo -e "${YELLOW}⚠${NC}  openvswitch-switch not found, skipping mesh bridge creation"
fi

# Step 6: Setup netmaker (one-time HOST enrollment)
echo "Setting up netmaker..."

# Check if netclient is installed
NETCLIENT_INSTALLED=false
if ! command -v netclient >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC}  netclient not found"
    echo -e "${YELLOW}⚠${NC}  To enable netmaker mesh networking, install netclient:"
    echo "     curl -sL https://apt.netmaker.org/gpg.key | sudo apt-key add -"
    echo "     curl -sL https://apt.netmaker.org/debian.deb.txt | sudo tee /etc/apt/sources.list.d/netmaker.list"
    echo "     sudo apt update && sudo apt install netclient"
else
    echo -e "${GREEN}✓${NC} netclient found at $(which netclient)"
    NETCLIENT_INSTALLED=true
fi

# Create netmaker environment file for join token
NETMAKER_ENV="$CONFIG_DIR/netmaker.env"
if [ ! -f "$NETMAKER_ENV" ]; then
    cat > "$NETMAKER_ENV" <<'EOF'
# Netmaker enrollment token for HOST
# Once host joins, all containers automatically get mesh networking
#
# Get token from: Netmaker Server > Networks > Access Keys > Enrollment Keys
# Then add here:
# NETMAKER_TOKEN=your-enrollment-token-here
#
# Or join manually: netclient join -t <token>
EOF
    chmod 600 "$NETMAKER_ENV"
    echo -e "${GREEN}✓${NC} Created netmaker token file: $NETMAKER_ENV"
else
    echo -e "${GREEN}✓${NC} Netmaker token file already exists: $NETMAKER_ENV"
fi

# Attempt to join netmaker if token is configured
if [ "$NETCLIENT_INSTALLED" = true ]; then
    # Load token from env file
    if [ -f "$NETMAKER_ENV" ]; then
        source "$NETMAKER_ENV"
    fi

    # Check if already joined
    if netclient list >/dev/null 2>&1 && netclient list | grep -q "Connected networks:"; then
        echo -e "${GREEN}✓${NC} Host already joined to netmaker network"
        netclient list | head -5
    elif [ -n "$NETMAKER_TOKEN" ]; then
        echo "Joining host to netmaker network..."
        if netclient join -t "$NETMAKER_TOKEN"; then
            echo -e "${GREEN}✓${NC} Successfully joined netmaker network"
            echo -e "${GREEN}✓${NC} Containers will automatically have mesh networking"
        else
            echo -e "${YELLOW}⚠${NC}  Failed to join netmaker (check token)"
            echo -e "${YELLOW}⚠${NC}  Containers will use bridge mode until host joins"
        fi
    else
        echo -e "${YELLOW}⚠${NC}  No NETMAKER_TOKEN set in $NETMAKER_ENV"
        echo -e "${YELLOW}⚠${NC}  Add token and run: netclient join -t \$NETMAKER_TOKEN"
        echo -e "${YELLOW}⚠${NC}  Or containers will use traditional bridge networking"
    fi
fi

# Step 7: Create systemd service
echo "Creating systemd service..."

# Set DHCP server flag if requested
DHCP_FLAG=""
if [ "$ENABLE_DHCP_SERVER" = "true" ]; then
    echo -e "${GREEN}✓${NC} DHCP server will be enabled"
    DHCP_FLAG="--enable-dhcp-server"
fi

cat > "$SYSTEMD_DIR/op-dbus.service" <<EOF
[Unit]
Description=op-dbus - Declarative system state management
Documentation=https://github.com/ghostbridge/op-dbus
After=network-online.target openvswitch-switch.service
Wants=network-online.target
Requires=openvswitch-switch.service

[Service]
Type=simple
ExecStart=/usr/local/bin/op-dbus run --state-file /etc/op-dbus/state.json $DHCP_FLAG
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
EOF
echo -e "${GREEN}✓${NC} Created: $SYSTEMD_DIR/op-dbus.service"

# Step 8: Reload systemd
echo "Reloading systemd..."
systemctl daemon-reload
echo -e "${GREEN}✓${NC} Systemd reloaded"

# Step 9: Show installation summary
echo ""
echo -e "${GREEN}=== Installation Complete ===${NC}"
echo ""
echo "Binary:        $INSTALL_DIR/op-dbus"
echo "Config:        $CONFIG_DIR/state.json"
echo "Netmaker:      $CONFIG_DIR/netmaker.env"
echo "Service:       $SYSTEMD_DIR/op-dbus.service"
echo ""
echo -e "${YELLOW}System Status:${NC}"
if [ -f "$STATE_FILE" ]; then
    INTERFACE_COUNT=$(jq '.plugins.net.interfaces | length' "$STATE_FILE" 2>/dev/null || echo "0")
    if [ "$INTERFACE_COUNT" -gt 0 ]; then
        echo "Detected:       $INTERFACE_COUNT existing network interface(s) under management"
    else
        echo "Status:         Ready to manage network interfaces as they are added"
    fi
fi
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "1. Add netmaker token: nano $CONFIG_DIR/netmaker.env"
echo "2. Review state file:  nano $STATE_FILE"
echo "3. Test query:         op-dbus query"
echo "4. Test diff:          op-dbus diff $STATE_FILE"
echo "5. Test apply (safe):  op-dbus apply $STATE_FILE"
echo "6. Enable service:     systemctl enable op-dbus"
echo "7. Start service:      systemctl start op-dbus"
echo "8. Check status:       systemctl status op-dbus"
echo "9. View logs:          journalctl -u op-dbus -f"
echo ""
echo -e "${YELLOW}Container Setup:${NC}"
echo "For netmaker-enabled containers, add to state.json:"
echo '  "lxc": {'
echo '    "containers": [{'
echo '      "id": "100",'
echo '      "veth": "vi100",'
echo '      "bridge": "vmbr0",'
echo '      "properties": {'
echo '        "network_type": "netmaker"'
echo '      }'
echo '    }]'
echo '  }'
echo ""
echo "For traditional bridge containers:"
echo '  "properties": { "network_type": "bridge" }'
echo ""
echo -e "${YELLOW}⚠  WARNING:${NC} Test manually before enabling service!"
echo -e "${YELLOW}⚠  WARNING:${NC} Network changes can cause 20min downtime on failure!"
echo ""
