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
    echo "Usage: $0 [OPTIONS] [BINARY_PATH]"
    echo ""
    echo "Options:"
    echo "  --no-proxmox      Skip Proxmox/LXC features (enterprise standalone mode)"
    echo "  --agent-only      Minimal install (D-Bus plugins only, no blockchain)"
    echo "  --help, -h        Show this help message"
    echo ""
    echo "Arguments:"
    echo "  BINARY_PATH       Path to op-dbus binary (default: target/release/op-dbus)"
    echo ""
    echo "Examples:"
    echo "  $0                           # Full install (Proxmox mode)"
    echo "  $0 --no-proxmox              # Enterprise standalone (no containers)"
    echo "  $0 --agent-only              # Minimal agent (no blockchain)"
    echo "  $0 --no-proxmox target/release/op-dbus  # Custom binary path"
    echo ""
    echo "Deployment Modes:"
    echo "  Full (default):   D-Bus + Blockchain + LXC/Proxmox + Netmaker"
    echo "  Standalone:       D-Bus + Blockchain (skip LXC/Proxmox features)"
    echo "  Agent:            D-Bus only (minimal footprint)"
    exit 0
fi

# Parse flags
NO_PROXMOX=false
AGENT_ONLY=false
BINARY_PATH=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-proxmox)
            NO_PROXMOX=true
            shift
            ;;
        --agent-only)
            AGENT_ONLY=true
            NO_PROXMOX=true  # Agent mode implies no Proxmox
            shift
            ;;
        *)
            BINARY_PATH="$1"
            shift
            ;;
    esac
done

# Set defaults
BINARY_PATH="${BINARY_PATH:-target/release/op-dbus}"

echo -e "${GREEN}=== op-dbus Installation ===${NC}"

# Show deployment mode
if [ "$AGENT_ONLY" = true ]; then
    echo -e "${YELLOW}Deployment Mode: Agent Only${NC} (D-Bus plugins only)"
elif [ "$NO_PROXMOX" = true ]; then
    echo -e "${YELLOW}Deployment Mode: Standalone${NC} (D-Bus + Blockchain, no Proxmox)"
else
    echo "Deployment Mode: Full (D-Bus + Blockchain + LXC/Proxmox)"
fi
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./install.sh)${NC}"
    exit 1
fi

# Variables
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

# Step 3.5: Setup BTRFS subvolumes for blockchain storage (if on BTRFS)
# Skip if agent-only mode
if [ "$AGENT_ONLY" = false ]; then
    BLOCKCHAIN_DIR="/var/lib/op-dbus/blockchain"
    echo "Setting up blockchain storage..."
else
    echo -e "${YELLOW}Skipping blockchain setup (agent-only mode)${NC}"
fi

if [ "$AGENT_ONLY" = false ]; then

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

    # Create cache subvolume
    CACHE_DIR="/var/lib/op-dbus/@cache"
    if ! sudo btrfs subvolume show "$CACHE_DIR" >/dev/null 2>&1; then
        # Remove empty directory if exists
        if [ -d "$CACHE_DIR" ]; then
            sudo rmdir "$CACHE_DIR" 2>/dev/null || true
        fi
        sudo mkdir -p "$(dirname $CACHE_DIR)"
        sudo btrfs subvolume create "$CACHE_DIR"
        sudo btrfs property set "$CACHE_DIR" compression zstd
        echo -e "${GREEN}✓${NC} Created cache subvolume with zstd compression"
    else
        # Ensure compression is enabled
        sudo btrfs property set "$CACHE_DIR" compression zstd
        echo -e "${GREEN}✓${NC} Cache subvolume exists, ensured zstd compression"
    fi

    # Create cache directory structure
    sudo mkdir -p "$CACHE_DIR"/{embeddings/vectors,blocks/{by-number,by-hash},queries,diffs}
    echo -e "${GREEN}✓${NC} Created cache directory structure"

    # Create snapshot directory
    sudo mkdir -p "/var/lib/op-dbus/@cache-snapshots"

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
            # Remove empty directory if it exists
            if [ -d "$BLOCKCHAIN_DIR" ]; then
                sudo rmdir "$BLOCKCHAIN_DIR" 2>/dev/null || true
            fi

            # Ensure parent directory exists
            sudo mkdir -p "$(dirname $BLOCKCHAIN_DIR)"

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

fi  # End blockchain setup (agent-only check)

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

# Step 4.5: Create LXC template with netclient (Proxmox mode only)
if [ "$NO_PROXMOX" = false ]; then
    echo "Checking for netmaker-ready LXC template..."
    
    TEMPLATE_NAME="debian-13-netmaker_custom.tar.zst"
    TEMPLATE_PATH="/var/lib/pve/local-btrfs/template/cache/$TEMPLATE_NAME"
    
    if [ -f "$TEMPLATE_PATH" ]; then
        echo -e "${GREEN}✓${NC} Template already exists: $TEMPLATE_NAME"
    else
        echo -e "${YELLOW}⚠${NC}  Template not found, creating it now..."
        
        # Run template creation script inline
        if [ -f "./create-netmaker-template.sh" ]; then
            echo "Running create-netmaker-template.sh..."
            bash ./create-netmaker-template.sh || {
                echo -e "${YELLOW}⚠${NC}  Template creation failed, continuing without it"
                echo -e "${YELLOW}⚠${NC}  You can create it later with: sudo ./create-netmaker-template.sh"
            }
        else
            echo -e "${YELLOW}⚠${NC}  create-netmaker-template.sh not found"
            echo -e "${YELLOW}⚠${NC}  Create template manually or containers won't have netclient"
        fi
    fi
else
    echo -e "${YELLOW}Skipping LXC template creation (not in Proxmox mode)${NC}"
fi

# Step 5: Create mesh bridge for netmaker containers (Proxmox mode only)
if [ "$NO_PROXMOX" = false ]; then
    echo "Creating mesh bridge for netmaker containers..."
else
    echo -e "${YELLOW}Skipping mesh bridge creation (standalone mode)${NC}"
fi

if [ "$NO_PROXMOX" = false ]; then

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

# Check if netclient is installed, if not, install it
NETCLIENT_INSTALLED=false
if ! command -v netclient >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC}  netclient not found, installing..."
    
    # Install netclient via direct binary download
    wget -O /tmp/netclient https://fileserver.netmaker.io/releases/download/v1.1.0/netclient-linux-amd64
    chmod +x /tmp/netclient
    /tmp/netclient install
    
    if command -v netclient >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} netclient installed successfully"
        NETCLIENT_INSTALLED=true
    else
        echo -e "${RED}✗${NC} netclient installation failed"
        NETCLIENT_INSTALLED=false
    fi
else
    echo -e "${GREEN}✓${NC} netclient already installed at $(which netclient)"
    NETCLIENT_INSTALLED=true
fi

# Install first-boot systemd service for netmaker join
if [ "$NETCLIENT_INSTALLED" = true ]; then
    echo "Installing first-boot netmaker join service..."
    
    cat > /usr/local/bin/netmaker-first-boot.sh <<'FIRSTBOOT_EOF'
#!/bin/bash
# First boot script to join netmaker network on host
# Runs once on first boot, then disables itself

NETMAKER_TOKEN_FILE="/etc/op-dbus/netmaker.env"
MARKER_FILE="/var/lib/op-dbus/netmaker-joined"

# Exit if already joined
if [ -f "$MARKER_FILE" ]; then
    exit 0
fi

# Wait for network
sleep 5

# Read token from env file
if [ ! -f "$NETMAKER_TOKEN_FILE" ]; then
    echo "No netmaker env file found"
    exit 0
fi

source "$NETMAKER_TOKEN_FILE"

if [ -z "$NETMAKER_TOKEN" ]; then
    echo "NETMAKER_TOKEN not set"
    exit 0
fi

# Join netmaker
echo "Joining netmaker network..."
if netclient join -t "$NETMAKER_TOKEN"; then
    echo "Successfully joined netmaker network"
    mkdir -p /var/lib/op-dbus
    touch "$MARKER_FILE"
else
    echo "Failed to join netmaker network"
    exit 1
fi
FIRSTBOOT_EOF
    
    chmod +x /usr/local/bin/netmaker-first-boot.sh
    
    cat > /etc/systemd/system/netmaker-first-boot.service <<'SERVICE_EOF'
[Unit]
Description=Netmaker First Boot Join (Host)
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/var/lib/op-dbus/netmaker-joined

[Service]
Type=oneshot
ExecStart=/usr/local/bin/netmaker-first-boot.sh
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
SERVICE_EOF
    
    systemctl enable netmaker-first-boot.service
    echo -e "${GREEN}✓${NC} First-boot netmaker service installed and enabled"
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

        # Auto-add netmaker interfaces to mesh bridge
        echo "Checking for netmaker interfaces to add to mesh bridge..."
        for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-")) | .ifname'); do
            if sudo ovs-vsctl list-ports mesh 2>/dev/null | grep -q "^${iface}$"; then
                echo -e "${GREEN}✓${NC} Interface $iface already in mesh bridge"
            else
                echo "Adding netmaker interface $iface to mesh bridge..."
                if sudo ovs-vsctl add-port mesh "$iface" 2>/dev/null; then
                    echo -e "${GREEN}✓${NC} Added $iface to mesh bridge"
                else
                    echo -e "${YELLOW}⚠${NC}  Could not add $iface to mesh bridge"
                fi
            fi
        done

    elif [ -n "$NETMAKER_TOKEN" ]; then
        echo "Joining host to netmaker network..."
        if netclient join -t "$NETMAKER_TOKEN"; then
            echo -e "${GREEN}✓${NC} Successfully joined netmaker network"
            echo -e "${GREEN}✓${NC} Containers will automatically have mesh networking"

            # Wait a moment for interface to appear
            sleep 2

            # Auto-add netmaker interfaces to mesh bridge
            echo "Checking for netmaker interfaces to add to mesh bridge..."
            for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-")) | .ifname'); do
                if sudo ovs-vsctl list-ports mesh 2>/dev/null | grep -q "^${iface}$"; then
                    echo -e "${GREEN}✓${NC} Interface $iface already in mesh bridge"
                else
                    echo "Adding netmaker interface $iface to mesh bridge..."
                    if sudo ovs-vsctl add-port mesh "$iface" 2>/dev/null; then
                        echo -e "${GREEN}✓${NC} Added $iface to mesh bridge"
                    else
                        echo -e "${YELLOW}⚠${NC}  Could not add $iface to mesh bridge"
                    fi
                fi
            done

        else
            echo -e "${YELLOW}⚠${NC}  Failed to join netmaker (check token)"
            echo -e "${YELLOW}⚠${NC}  Containers will use bridge mode until host joins"
        fi
    else
        echo -e "${YELLOW}⚠${NC}  No NETMAKER_TOKEN set in $NETMAKER_ENV"
        echo -e "${YELLOW}⚠${NC}  Add token and run: netclient join -t \$NETMAKER_TOKEN"
        echo -e "${YELLOW}⚠${NC}  Or containers will use traditional bridge networking"
    fi

    # Install LXC hook for automatic container netmaker join
    echo "Installing LXC netmaker hook..."
    HOOK_DIR="/usr/share/lxc/hooks"
    sudo mkdir -p "$HOOK_DIR"

    # Create hook script inline
    sudo tee "$HOOK_DIR/netmaker-join" > /dev/null <<'HOOK_EOF'
#!/bin/bash
# LXC hook to automatically join container to netmaker on start
# Installed by op-dbus install.sh

# Get container ID from LXC environment
CT_ID="${LXC_NAME##*-}"

# Paths
NETMAKER_ENV="/etc/op-dbus/netmaker.env"
LOG_FILE="/var/log/lxc-netmaker-hook.log"

# Logging
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [CT$CT_ID] $1" >> "$LOG_FILE"
}

log "Hook triggered for container $CT_ID"

# Check if container uses mesh bridge
CONTAINER_CONFIG="/etc/pve/lxc/${CT_ID}.conf"
if [ ! -f "$CONTAINER_CONFIG" ] || ! grep -q "bridge=mesh" "$CONTAINER_CONFIG" 2>/dev/null; then
    log "Container not using mesh bridge, skipping"
    exit 0
fi

log "Container uses mesh bridge, proceeding with netmaker join"

# Load token
if [ ! -f "$NETMAKER_ENV" ]; then
    log "ERROR: Netmaker env file not found"
    exit 0
fi

source "$NETMAKER_ENV"

if [ -z "$NETMAKER_TOKEN" ]; then
    log "WARNING: NETMAKER_TOKEN not set"
    exit 0
fi

# Wait for container
log "Waiting for container to be ready..."
sleep 3

# Check netclient
if ! pct exec "$CT_ID" -- which netclient >/dev/null 2>&1; then
    log "WARNING: netclient not found in container"
    exit 0
fi

# Check if already joined
if pct exec "$CT_ID" -- netclient list 2>/dev/null | grep -q "Connected networks:"; then
    log "Container already joined to netmaker"
    exit 0
fi

# Join netmaker
log "Joining container to netmaker..."
if pct exec "$CT_ID" -- netclient join -t "$NETMAKER_TOKEN" >> "$LOG_FILE" 2>&1; then
    log "SUCCESS: Container joined netmaker"

    sleep 2
    NETMAKER_IFACE=$(pct exec "$CT_ID" -- ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-")) | .ifname' | head -1)

    if [ -n "$NETMAKER_IFACE" ]; then
        log "Netmaker interface: $NETMAKER_IFACE"
    fi
else
    log "ERROR: Failed to join netmaker"
fi

log "Hook completed"
exit 0
HOOK_EOF

    sudo chmod +x "$HOOK_DIR/netmaker-join"
    echo -e "${GREEN}✓${NC} LXC hook installed: $HOOK_DIR/netmaker-join"
    echo -e "${GREEN}✓${NC} Containers will auto-join netmaker on startup"

    # Create global LXC config to enable hook for all containers
    LXC_COMMON_CONF="/usr/share/lxc/config/common.conf.d"
    sudo mkdir -p "$LXC_COMMON_CONF"

    sudo tee "$LXC_COMMON_CONF/netmaker.conf" > /dev/null <<'LXC_CONF_EOF'
# Automatic netmaker join hook for op-dbus containers
# Only triggers for containers using mesh bridge
lxc.hook.start-host = /usr/share/lxc/hooks/netmaker-join
LXC_CONF_EOF

    echo -e "${GREEN}✓${NC} LXC config updated: $LXC_COMMON_CONF/netmaker.conf"
fi

fi  # End Proxmox-specific setup (mesh bridge + netmaker)

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
echo "Deployment:    $([ "$AGENT_ONLY" = true ] && echo "Agent Only" || ([ "$NO_PROXMOX" = true ] && echo "Standalone" || echo "Full"))"
echo "Binary:        $INSTALL_DIR/op-dbus"
echo "Config:        $CONFIG_DIR/state.json"
if [ "$NO_PROXMOX" = false ]; then
    echo "Netmaker:      $CONFIG_DIR/netmaker.env"
fi
if [ "$AGENT_ONLY" = false ]; then
    echo "Blockchain:    $BLOCKCHAIN_DIR"
fi
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
STEP=1
if [ "$NO_PROXMOX" = false ]; then
    echo "$STEP. Add netmaker token: nano $CONFIG_DIR/netmaker.env"
    STEP=$((STEP + 1))
fi
echo "$STEP. Review state file:  nano $STATE_FILE"
STEP=$((STEP + 1))
echo "$STEP. Test query:         op-dbus query"
STEP=$((STEP + 1))
echo "$STEP. Test diff:          op-dbus diff $STATE_FILE"
STEP=$((STEP + 1))
echo "$STEP. Test apply (safe):  op-dbus apply $STATE_FILE"
STEP=$((STEP + 1))
echo "$STEP. Enable service:     systemctl enable op-dbus"
STEP=$((STEP + 1))
echo "$STEP. Start service:      systemctl start op-dbus"
STEP=$((STEP + 1))
echo "$STEP. Check status:       systemctl status op-dbus"
STEP=$((STEP + 1))
echo "$STEP. View logs:          journalctl -u op-dbus -f"

if [ "$NO_PROXMOX" = false ]; then
    echo ""
    echo -e "${YELLOW}Container Setup:${NC}"
    echo "For netmaker-enabled containers, add to state.json:"
fi

if [ "$NO_PROXMOX" = false ]; then
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
fi  # End container setup instructions

echo ""
echo -e "${YELLOW}⚠  WARNING:${NC} Test manually before enabling service!"
echo -e "${YELLOW}⚠  WARNING:${NC} Network changes can cause 20min downtime on failure!"
echo ""
