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
NET_BRIDGE_SETUP=false

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
        --net-bridge-setup)
            NET_BRIDGE_SETUP=true
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
    echo -e "${RED}Error: Please run as root ( ./install.sh)${NC}"
    exit 1
fi

# Variables
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/op-dbus"
STATE_FILE="$CONFIG_DIR/state.json"
SYSTEMD_DIR="/etc/systemd/system"
OVSDB_SOCK="/var/run/openvswitch/db.sock"

# === D-Bus (busctl) helpers for net plugin (defined early for use below) ===
dbus_apply_net_state_file() {
    local json_path="$1"
    if command -v busctl >/dev/null 2>&1; then
        busctl --system call org.opdbus /org/opdbus/state/net org.opdbus.StatePlugin ApplyState s "$json_path" >/dev/null 2>&1 || return 1
    else
        return 1
    fi
}

dbus_create_or_update_bridge() {
    local name="$1"; shift
    local if_type="$1"; shift
    local ipv4_enabled="$1"; shift
    local ip="$1"; shift
    local prefix="$1"; shift
    local gw="$1"; shift
    local ports_json="${1:-[]}"; shift || true

    local tmp_json
    tmp_json="$(mktemp)"
    cat > "$tmp_json" <<EOF
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "$name",
          "type": "$if_type",
          "ports": $ports_json,
          "ipv4": {
            "enabled": ${ipv4_enabled:-false}$(
              if [ "$ipv4_enabled" = true ] && [ -n "$ip" ] && [ -n "$prefix" ]; then
                printf ',\n            "address": [{"ip": "%s", "prefix": %s}]' "$ip" "$prefix"
              fi
            )$(
              if [ "$ipv4_enabled" = true ] && [ -n "$gw" ]; then
                printf ',\n            "gateway": "%s"' "$gw"
              fi
            )
          }
        }
      ]
    }
  }
}
EOF

    if dbus_apply_net_state_file "$tmp_json"; then
        echo -e "${GREEN}✓${NC} Applied bridge via D-Bus: $name"
    else
        echo -e "${YELLOW}⚠${NC}  D-Bus apply failed for $name (service may not be ready)"
    fi
    rm -f "$tmp_json"
}

# Ensure D-Bus policy and activation for org.opdbus (early definition)
ensure_dbus_policy() {
    local policy_dir="/etc/dbus-1/system.d"
    local services_dir="/usr/share/dbus-1/system-services"
    mkdir -p "$policy_dir" "$services_dir"

    cat > "$policy_dir/org.opdbus.conf" <<'EOF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
  "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="org.opdbus"/>
    <allow send_destination="org.opdbus"/>
  </policy>
  <policy context="default">
    <allow send_destination="org.opdbus"/>
  </policy>
  <policy context="default">
    <allow receive_sender="org.opdbus"/>
  </policy>
  <limit name="max_match_rules_per_connection">10240</limit>
</busconfig>
EOF

    cat > "$services_dir/org.opdbus.service" <<'EOF'
[D-BUS Service]
Name=org.opdbus
Exec=/usr/local/bin/op-dbus run
SystemdService=op-dbus.service
EOF

    if command -v busctl >/dev/null 2>&1; then
        busctl --system call org.freedesktop.DBus / org.freedesktop.DBus ReloadConfig >/dev/null 2>&1 || true
    fi
}


# OVSDB JSON-RPC helper functions (NO ovs-vsctl!)
ovsdb_rpc() {
    local method="$1"
    local params="$2"
    echo "{\"method\":\"$method\",\"params\":$params,\"id\":0}" | \
        socat - UNIX-CONNECT:"$OVSDB_SOCK" 2>/dev/null | head -1
}

ovsdb_list_bridges() {
    local result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[],\"columns\":[\"name\"]}]]")
    echo "$result" | jq -r '.result[0].rows[].name' 2>/dev/null
}

ovsdb_list_ports() {
    local bridge="$1"
    # Get bridge UUID
    local bridge_result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$bridge\"]],\"columns\":[\"_uuid\",\"ports\"]}]]")
    local bridge_uuid=$(echo "$bridge_result" | jq -r '.result[0].rows[0]._uuid[1]' 2>/dev/null)
    
    if [ -z "$bridge_uuid" ] || [ "$bridge_uuid" = "null" ]; then
        return 1
    fi
    
    # Get port UUIDs from bridge
    local port_uuids=$(echo "$bridge_result" | jq -r '.result[0].rows[0].ports[1][]?[1]' 2>/dev/null)
    
    # Get port names
    for port_uuid in $port_uuids; do
        local port_result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Port\",\"where\":[[\"_uuid\",\"==\",[\"uuid\",\"$port_uuid\"]]],\"columns\":[\"name\"]}]]")
        echo "$port_result" | jq -r '.result[0].rows[].name' 2>/dev/null
    done
}

ovsdb_bridge_exists() {
    local bridge="$1"
    local result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$bridge\"]],\"columns\":[\"name\"]}]]")
    local count=$(echo "$result" | jq -r '.result[0].rows | length' 2>/dev/null)
    [ "$count" -gt 0 ]
}

ovsdb_create_bridge() {
    local bridge="$1"
    # Check if bridge already exists
    if ovsdb_bridge_exists "$bridge"; then
        echo "Bridge $bridge already exists"
        return 0
    fi

    # Create bridge via OVSDB JSON-RPC
    # Step 1: Insert new bridge row
    # Step 2: Add bridge to Open_vSwitch.bridges set
    local result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",{\"op\":\"insert\",\"table\":\"Bridge\",\"row\":{\"name\":\"$bridge\"},\"uuid-name\":\"new_bridge\"},{\"op\":\"mutate\",\"table\":\"Open_vSwitch\",\"where\":[],\"mutations\":[[\"bridges\",\"insert\",[\"set\",[[\"named-uuid\",\"new_bridge\"]]]]]}]")

    # Check for success (no error in result)
    if echo "$result" | jq -e '.error and .error != null' >/dev/null 2>&1 || \
       echo "$result" | jq -e '.result[]? | select(.error != null)' >/dev/null 2>&1; then
        echo "Failed to create bridge $bridge: $(echo "$result" | jq -r '.error // (.result[]? | select(.error!=null) | .error)')"
        return 1
    else
        echo "Successfully created bridge $bridge"
        return 0
    fi
}

ovsdb_add_port() {
    local bridge="$1"
    local port="$2"

    # Check if bridge exists
    if ! ovsdb_bridge_exists "$bridge"; then
        echo "Bridge $bridge does not exist"
        return 1
    fi

    # Check if port already exists on bridge
    if ovsdb_list_ports "$bridge" | grep -q "^${port}$"; then
        echo "Port $port already exists on bridge $bridge"
        return 0
    fi

    # Get bridge UUID
    local bridge_result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",[{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"$bridge\"]],\"columns\":[\"_uuid\"]}]]")
    local bridge_uuid=$(echo "$bridge_result" | jq -r '.result[0].rows[0]._uuid[1]' 2>/dev/null)

    if [ -z "$bridge_uuid" ] || [ "$bridge_uuid" = "null" ]; then
        echo "Failed to get bridge UUID for $bridge"
        return 1
    fi

    # Create port if it doesn't exist
    local port_result=$(ovsdb_rpc "transact" "[\"Open_vSwitch\",{\"op\":\"insert\",\"table\":\"Interface\",\"row\":{\"name\":\"$port\",\"type\":\"system\"},\"uuid-name\":\"new_interface\"},{\"op\":\"insert\",\"table\":\"Port\",\"row\":{\"name\":\"$port\",\"interfaces\":[\"set\",[[\"named-uuid\",\"new_interface\"]]]},\"uuid-name\":\"new_port\"},{\"op\":\"mutate\",\"table\":\"Bridge\",\"where\":[[\"_uuid\",\"==\",[\"uuid\",\"$bridge_uuid\"]]],\"mutations\":[[\"ports\",\"insert\",[\"set\",[[\"named-uuid\",\"new_port\"]]]]]}]")

    # Check for success
    if echo "$port_result" | jq -e '.error and .error != null' >/dev/null 2>&1 || \
       echo "$port_result" | jq -e '.result[]? | select(.error != null)' >/dev/null 2>&1; then
        echo "Failed to add port $port to bridge $bridge: $(echo "$port_result" | jq -r '.error // (.result[]? | select(.error!=null) | .error)')"
        return 1
    else
        echo "Successfully added port $port to bridge $bridge"
        return 0
    fi
}

# Step 1: Check binary exists
echo "Checking binary..."
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    echo "Build first with: cargo build --release"
    exit 1
fi
echo -e "${GREEN}✓${NC} Found binary: $BINARY_PATH"

# Step 1.5: Stop service if running (to allow binary replacement)
if systemctl is-active --quiet op-dbus 2>/dev/null; then
    echo "Stopping op-dbus service..."
    systemctl stop op-dbus
    echo -e "${GREEN}✓${NC} Service stopped"
fi

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
    EXISTING_SUBVOLS=$( btrfs subvolume list / 2>/dev/null | grep -E "@var/lib/op-dbus/blockchain|@blockchain/op-dbus" || true)

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
                     rm -rf "$BLOCKCHAIN_DIR"/*
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
    if !  btrfs subvolume show "$CACHE_DIR" >/dev/null 2>&1; then
        # Remove empty directory if exists
        if [ -d "$CACHE_DIR" ]; then
             rmdir "$CACHE_DIR" 2>/dev/null || true
        fi
         mkdir -p "$(dirname $CACHE_DIR)"
         btrfs subvolume create "$CACHE_DIR"
         btrfs property set "$CACHE_DIR" compression zstd
        echo -e "${GREEN}✓${NC} Created cache subvolume with zstd compression"
    else
        # Ensure compression is enabled
         btrfs property set "$CACHE_DIR" compression zstd
        echo -e "${GREEN}✓${NC} Cache subvolume exists, ensured zstd compression"
    fi

    # Create cache directory structure
     mkdir -p "$CACHE_DIR"/{embeddings/vectors,blocks/{by-number,by-hash},queries,diffs}
    echo -e "${GREEN}✓${NC} Created cache directory structure"

    # Create snapshot directory
     mkdir -p "/var/lib/op-dbus/@cache-snapshots"

    # Create subvolumes if they don't exist
    if !  btrfs subvolume show "$BLOCKCHAIN_DIR" >/dev/null 2>&1; then
        # Check if it's already a regular directory with files
        if [ -d "$BLOCKCHAIN_DIR" ] && [ "$(ls -A $BLOCKCHAIN_DIR 2>/dev/null)" ]; then
            echo -e "${YELLOW}⚠${NC}  $BLOCKCHAIN_DIR exists as regular directory with files"
            echo -e "${YELLOW}⚠${NC}  Converting to BTRFS subvolume..."

            # Move data temporarily
            TEMP_BACKUP="/tmp/op-dbus-blockchain-backup-$$"
             mv "$BLOCKCHAIN_DIR" "$TEMP_BACKUP"
             mkdir -p "$(dirname $BLOCKCHAIN_DIR)"

            # Create subvolume
             btrfs subvolume create "$BLOCKCHAIN_DIR"

            # Restore data
             mv "$TEMP_BACKUP"/* "$BLOCKCHAIN_DIR/" 2>/dev/null || true
             rm -rf "$TEMP_BACKUP"

            echo -e "${GREEN}✓${NC} Converted to BTRFS subvolume with data preserved"
        else
            # Remove empty directory if it exists
            if [ -d "$BLOCKCHAIN_DIR" ]; then
                 rmdir "$BLOCKCHAIN_DIR" 2>/dev/null || true
            fi

            # Ensure parent directory exists
             mkdir -p "$(dirname $BLOCKCHAIN_DIR)"

            # Create fresh subvolume
             btrfs subvolume create "$BLOCKCHAIN_DIR"
            echo -e "${GREEN}✓${NC} Created blockchain BTRFS subvolume"
        fi
    else
        echo -e "${GREEN}✓${NC} Blockchain subvolume already exists"
    fi

    # Set permissions
     chown -R root:root "$BLOCKCHAIN_DIR"
     chmod 755 "$BLOCKCHAIN_DIR"

else
    # Not BTRFS, just use regular directory
    echo "Using regular directory (not BTRFS)"
     mkdir -p "$BLOCKCHAIN_DIR"
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

        # Check if OVSDB socket is available
        if [ -S "$OVSDB_SOCK" ]; then
            # Get OVS bridge information via OVSDB JSON-RPC
            local bridges=$(ovsdb_list_bridges || echo "")

            if [ -n "$bridges" ]; then
                echo -e "${GREEN}✓${NC} Found OVS bridges: $bridges"

                # For each bridge, get its configuration
                for bridge in $bridges; do
                    # Get ports via OVSDB JSON-RPC (excluding the bridge itself)
                    local ports=$(ovsdb_list_ports "$bridge" 2>/dev/null | grep -v "^$bridge$" | tr '\n' ' ')

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

# Step 4.5: Skip template creation (ovsbr0 networking is now default for all containers)
# Netclient can be installed in containers after creation if needed
if [ "$NO_PROXMOX" = false ]; then
    echo -e "${GREEN}✓${NC} Default networking: eth0 on ovsbr0 (internet via OVS flows)"
    echo -e "${GREEN}✓${NC} Use any OS template - networking is automatic"
fi

# Step 5: Configure LXC default settings (Proxmox mode only)
if [ "$NO_PROXMOX" = false ]; then
    echo "Configuring LXC default container settings..."

    # Create op-dbus LXC config for default container properties
    LXC_DEFAULTS="/usr/share/lxc/config/common.conf.d/99-op-dbus.conf"
    cat > "$LXC_DEFAULTS" <<'LXC_DEFAULTS_EOF'
# op-dbus default container configuration
# Applied to ALL containers created on this Proxmox host
#
# Network Configuration:
# - NO network interface by default
# - Add interface manually: --net0 name=eth0,bridge=ovsbr0
# - Containers use Unix sockets or add networking as needed
#
# Features:
# - Nesting enabled for Docker/nested containers
# - Unprivileged by default for security

# NO network interface by default
# Add --net0 when creating container if networking is needed

# Enable nesting for nested containers/Docker
lxc.apparmor.profile = unconfined
lxc.cgroup.devices.allow = a
lxc.cap.drop =

# Mount requirements for nesting
lxc.mount.auto = proc:rw sys:rw cgroup:rw
LXC_DEFAULTS_EOF

    echo -e "${GREEN}✓${NC} Created LXC defaults: $LXC_DEFAULTS"
    echo -e "${GREEN}✓${NC} Containers created WITHOUT network interface by default"
else
    echo -e "${YELLOW}Skipping LXC configuration (standalone mode)${NC}"
fi

# Step 6: Create OVS bridge (ovsbr0) and enslave uplink (portable: auto-detect uplink) via D-Bus
if [ "$NO_PROXMOX" = false ]; then
    # In bridge-setup mode, ensure D-Bus policy exists and service is running
    if [ "$NET_BRIDGE_SETUP" = true ]; then
        ensure_dbus_policy
        systemctl start op-dbus || true
        # Wait briefly for org.opdbus to appear
        for _i in 1 2 3 4 5; do
            if command -v busctl >/dev/null 2>&1 && busctl --system list | grep -q "org.opdbus"; then
                break
            fi
            sleep 0.5
        done
    fi

    # Detect uplink port (default route interface); portable, no ip(8)
    UPLINK_IF="$(awk '$1!="Iface" && $2=="00000000" {print $1; exit}' /proc/net/route 2>/dev/null)"
    if [ -z "$UPLINK_IF" ]; then
        # Fallback: first matching ethernet-style interface
        UPLINK_IF=$(ls -1 /sys/class/net 2>/dev/null | grep -E '^(en|eth)' | head -1)
    fi

    # Optional interactive prompt when running in a TTY
    if [ -t 0 ]; then
        echo ""
        echo "Detected uplink: ${UPLINK_IF:-<none>}"
        echo "Available interfaces:"
        ls -1 /sys/class/net | sed 's/^/  - /'
        read -r -p "Uplink interface to enslave into ovsbr0 [${UPLINK_IF:-none}]: " REPLY_UP
        if [ -n "$REPLY_UP" ]; then
            UPLINK_IF="$REPLY_UP"
        fi
    fi

    echo "Creating OVS bridge 'ovsbr0' and enslaving uplink (${UPLINK_IF:-none}) via D-Bus..."
    if [ -n "$UPLINK_IF" ] && [ -d "/sys/class/net/$UPLINK_IF" ]; then
        dbus_create_or_update_bridge "ovsbr0" "ovs-bridge" false "" "" "" "[\"$UPLINK_IF\"]"
    else
        dbus_create_or_update_bridge "ovsbr0" "ovs-bridge" false "" "" "" "[]"
        echo -e "${YELLOW}⚠${NC}  Could not determine uplink; created ovsbr0 without ports"
    fi

    # Apply desired state via D-Bus (third command), non-disruptive (skip in bridge-setup mode)
    if [ "$NET_BRIDGE_SETUP" != true ]; then
        if [ -f "$STATE_FILE" ]; then
            echo "Applying desired state via D-Bus (net plugin)..."
            if dbus_apply_net_state_file "$STATE_FILE"; then
                echo -e "${GREEN}✓${NC} State applied via D-Bus"
            else
                echo -e "${YELLOW}⚠${NC}  D-Bus apply did not complete (service may not be ready). It will apply after first start."
            fi
        else
            echo -e "${YELLOW}⚠${NC}  No state file at $STATE_FILE; skipping D-Bus apply"
        fi
    fi

    # If only bridge setup was requested, stop here (do not run later steps)
    if [ "$NET_BRIDGE_SETUP" = true ]; then
        echo -e "${GREEN}✓${NC} Bridge setup step complete"
        echo "Verify:"
        echo "  - Kernel: [ -d /sys/class/net/ovsbr0 ] && echo present || echo missing"
        echo "  - OVSDB:  echo '{\"method\":\"transact\",\"params\":[\"Open_vSwitch\",{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[[\"name\",\"==\",\"ovsbr0\"]],\"columns\":[\"name\",\"ports\"]}],\"id\":1}' | socat - UNIX-CONNECT:/var/run/openvswitch/db.sock"
        exit 0
    fi
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

# NOTE: Do NOT attempt to join netmaker during install
# Bridges must be created first via 'op-dbus apply'
# User will join netmaker manually after applying config

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
        for iface in $(ls -1 /sys/class/net | grep -E '^(nm-|netmaker)' || true); do
            if ovsdb_list_ports mesh 2>/dev/null | grep -q "^${iface}$"; then
                echo -e "${GREEN}✓${NC} Interface $iface already in mesh bridge"
            else
                echo "Adding netmaker interface $iface to mesh bridge..."
                if ovsdb_add_port mesh "$iface"; then
                    echo -e "${GREEN}✓${NC} Successfully added $iface to mesh bridge"
                else
                    echo -e "${YELLOW}⚠${NC}  Failed to add $iface to mesh bridge"
                fi
            fi
        done

    elif [ -n "$NETMAKER_TOKEN" ]; then
        echo "Netmaker token found, attempting to join..."

        # Ensure bridges exist (safe, non-disruptive): create if missing
        echo "Ensuring OVS bridges exist before netmaker join..."
        ovsdb_create_bridge "ovsbr0" || true
        ovsdb_create_bridge "mesh" || true

        # Now try to join netmaker
        echo "Joining host to netmaker network..."
        if netclient join -t "$NETMAKER_TOKEN"; then
            echo -e "${GREEN}✓${NC} Successfully joined netmaker network"
            echo -e "${GREEN}✓${NC} Containers will automatically have mesh networking"

            # Wait for netmaker interface to appear
            sleep 3

            # Check for netmaker interfaces (sysfs)
            NETMAKER_IFACES=$(ls -1 /sys/class/net 2>/dev/null | grep -E '^(nm-|netmaker)' || true)

            if [ -n "$NETMAKER_IFACES" ]; then
                echo -e "${GREEN}✓${NC} Netmaker interfaces created: $NETMAKER_IFACES"
                echo -e "${YELLOW}Note:${NC} Run 'op-dbus apply' to add netmaker interfaces to mesh bridge"
            fi
        else
            echo -e "${YELLOW}⚠${NC}  Failed to join netmaker (check token or network)"
            echo -e "${YELLOW}⚠${NC}  You can join manually later: netclient join -t \$NETMAKER_TOKEN"
        fi
    else
        echo -e "${YELLOW}⚠${NC}  No NETMAKER_TOKEN set in $NETMAKER_ENV"
        echo -e "${YELLOW}⚠${NC}  Add token and join manually:"
        echo -e "${YELLOW}   ${NC} 1. Edit $NETMAKER_ENV and add NETMAKER_TOKEN"
        echo -e "${YELLOW}   ${NC} 2. sudo op-dbus apply /etc/op-dbus/state.json"
        echo -e "${YELLOW}   ${NC} 3. sudo netclient join -t \$NETMAKER_TOKEN"
    fi

    # Install LXC hook for automatic container netmaker join
    echo "Installing LXC netmaker hook..."
    HOOK_DIR="/usr/share/lxc/hooks"
     mkdir -p "$HOOK_DIR"

    # Create hook script inline
     tee "$HOOK_DIR/netmaker-join" > /dev/null <<'HOOK_EOF'
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
    NETMAKER_IFACE=$(pct exec "$CT_ID" -- ls -1 /sys/class/net | grep -E '^nm-' | head -1)

    if [ -n "$NETMAKER_IFACE" ]; then
        log "Netmaker interface: $NETMAKER_IFACE"
    fi
else
    log "ERROR: Failed to join netmaker"
fi

log "Hook completed"
exit 0
HOOK_EOF

     chmod +x "$HOOK_DIR/netmaker-join"
    echo -e "${GREEN}✓${NC} LXC hook installed: $HOOK_DIR/netmaker-join"
    echo -e "${GREEN}✓${NC} Containers will auto-join netmaker on startup"

    # Create global LXC config to enable hook for all containers
    LXC_COMMON_CONF="/usr/share/lxc/config/common.conf.d"
     mkdir -p "$LXC_COMMON_CONF"

     tee "$LXC_COMMON_CONF/netmaker.conf" > /dev/null <<'LXC_CONF_EOF'
# Automatic netmaker join hook for op-dbus containers
# Only triggers for containers using mesh bridge
lxc.hook.start-host = /usr/share/lxc/hooks/netmaker-join
LXC_CONF_EOF

    echo -e "${GREEN}✓${NC} LXC config updated: $LXC_COMMON_CONF/netmaker.conf"

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
ExecStart=/usr/local/bin/op-dbus --state-file /etc/op-dbus/state.json $DHCP_FLAG run
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

# Step 9: Initialize D-Bus services
echo "Initializing D-Bus services..."
if command -v busctl >/dev/null 2>&1; then
    # Check if op-dbus D-Bus service is available
    if busctl --system list | grep -q "org.opdbus"; then
        echo -e "${GREEN}✓${NC} op-dbus D-Bus service is available"
    else
        echo -e "${YELLOW}⚠${NC}  op-dbus D-Bus service not yet available (will be after first run)"
    fi
else
    echo -e "${YELLOW}⚠${NC}  busctl not found, skipping D-Bus service check"
fi

# Step 10: Apply initial state via D-Bus (netcfg plugin)
if [ -f "$STATE_FILE" ] && command -v busctl >/dev/null 2>&1; then
    echo "Applying initial network configuration via D-Bus..."

    # Try to apply net plugin state via D-Bus
    if busctl --system call org.opdbus /org/opdbus/state/net org.opdbus.StatePlugin ApplyState s "$STATE_FILE" >/dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} Successfully applied network state via D-Bus"
    else
        echo -e "${YELLOW}⚠${NC}  D-Bus service not ready yet (will apply on first service start)"
        echo -e "${YELLOW}⚠${NC}  You can apply manually: op-dbus apply --plugin net $STATE_FILE"
    fi
else
    echo -e "${YELLOW}⚠${NC}  Skipping D-Bus state application (busctl not available or no state file)"
fi

# Step 11: Show installation summary
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
echo -e "${YELLOW}Next Steps (IN ORDER):${NC}"
STEP=1
echo "$STEP. Review state file:  nano $STATE_FILE"
STEP=$((STEP + 1))
echo "$STEP. Test query:         op-dbus query"
STEP=$((STEP + 1))
echo "$STEP. Test diff:          op-dbus diff $STATE_FILE"
STEP=$((STEP + 1))
echo "$STEP. ${GREEN}APPLY CONFIG${NC}:     op-dbus apply --plugin net $STATE_FILE  ${YELLOW}← Creates bridges!${NC}"
STEP=$((STEP + 1))
if [ "$NO_PROXMOX" = false ]; then
    echo "$STEP. ${GREEN}JOIN NETMAKER${NC}:    Add token to $CONFIG_DIR/netmaker.env"
    echo "                      Then: netclient join -t \$NETMAKER_TOKEN"
    STEP=$((STEP + 1))
fi
echo "$STEP. Enable service:     systemctl enable op-dbus"
STEP=$((STEP + 1))
echo "$STEP. Start service:      systemctl start op-dbus"
STEP=$((STEP + 1))
echo "$STEP. Check status:       systemctl status op-dbus"
STEP=$((STEP + 1))
echo "$STEP. View logs:          journalctl -u op-dbus -f"

if [ "$NO_PROXMOX" = false ]; then
echo ""
echo -e "${YELLOW}Creating Containers:${NC}"
echo "  Use ANY Proxmox OS template. No interface by default - add as needed:"
echo ""
echo "  # Socket-only container (NO network interface)"
echo "  pct create 100 local:vztmpl/debian-12-standard_12.7-1_amd64.tar.zst \\"
echo "    --hostname mycontainer --memory 512"
echo ""
echo "  # Container with internet (via ovsbr0)"
echo "  pct create 101 local:vztmpl/debian-12-standard_12.7-1_amd64.tar.zst \\"
echo "    --hostname mycontainer --memory 512 \\"
echo "    --net0 name=eth0,bridge=ovsbr0,type=veth"
echo ""
echo "  # Mesh networking container (for netmaker)"
echo "  pct create 102 local:vztmpl/debian-12-standard_12.7-1_amd64.tar.zst \\"
echo "    --hostname mesh-container --memory 512 \\"
echo "    --net0 name=eth0,bridge=mesh,type=veth"
echo ""
echo "Default container:"
echo "  ✓ NO network interface (socket networking only)"
echo "  ✓ Nesting enabled (for Docker)"
echo "  ✓ Add --net0 to get internet or mesh networking"
echo ""
echo "OVS bridges are configured in state.json and managed by op-dbus:"
echo "  - ovsbr0: Main bridge with internet (172.16.0.10/24)"
echo "  - mesh: Netmaker mesh bridge (no IP)"
echo "  - Created via OVSDB JSON-RPC (not ovs-vsctl commands)"
echo "  - Apply with: op-dbus apply --plugin net /etc/op-dbus/state.json"
echo ""
echo "To add netclient after creation:"
echo "  pct exec <ID> -- wget -O /tmp/netclient https://fileserver.netmaker.io/releases/download/v1.1.0/netclient-linux-amd64"
echo "  pct exec <ID> -- chmod +x /tmp/netclient && /tmp/netclient install"
fi  # End container setup instructions

echo ""
echo -e "${YELLOW}⚠  WARNING:${NC} Test manually before enabling service!"
echo -e "${YELLOW}⚠  WARNING:${NC} Network changes can cause 20min downtime on failure!"
echo ""

dbus_apply_net_state_file() {
    local json_path="$1"
    if command -v busctl >/dev/null 2>&1; then
        busctl --system call org.opdbus /org/opdbus/state/net org.opdbus.StatePlugin ApplyState s "$json_path" >/dev/null 2>&1 || return 1
    else
        return 1
    fi
}

dbus_create_or_update_bridge() {
    local name="$1"; shift
    local if_type="$1"; shift
    local ipv4_enabled="$1"; shift
    local ip="$1"; shift
    local prefix="$1"; shift
    local gw="$1"; shift
    local ports_json="${1:-[]}"; shift || true

    local tmp_json
    tmp_json="$(mktemp)"
    cat > "$tmp_json" <<EOF
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "$name",
          "type": "$if_type",
          "ports": $ports_json,
          "ipv4": {
            "enabled": ${ipv4_enabled:-false}$(
              if [ "$ipv4_enabled" = true ] && [ -n "$ip" ] && [ -n "$prefix" ]; then
                printf ',\n            "address": [{"ip": "%s", "prefix": %s}]' "$ip" "$prefix"
              fi
            )$(
              if [ "$ipv4_enabled" = true ] && [ -n "$gw" ]; then
                printf ',\n            "gateway": "%s"' "$gw"
              fi
            )
          }
        }
      ]
    }
  }
}
EOF

    if dbus_apply_net_state_file "$tmp_json"; then
        echo -e "${GREEN}✓${NC} Applied bridge via D-Bus: $name"
    else
        echo -e "${YELLOW}⚠${NC}  D-Bus apply failed for $name (service may not be ready)"
    fi
    rm -f "$tmp_json"
}
