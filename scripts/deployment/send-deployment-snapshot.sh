#!/bin/bash
# send-deployment-snapshot.sh - Send BTRFS snapshots to target deployment servers
#
# Usage: ./send-deployment-snapshot.sh <component> <target-server> [snapshot-name]
#
# This script uses btrfs send/receive to efficiently deploy system components
# to target servers for golden image deployment.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SNAPSHOT_BASE="/var/lib/deployment-snapshots"
SSH_USER="root"
SSH_KEY="${SSH_KEY:-/root/.ssh/id_rsa}"

# Arguments
COMPONENT="$1"
TARGET_SERVER="$2"
SNAPSHOT_NAME="${3:-}"

# Validate arguments
if [ $# -lt 2 ]; then
    echo -e "${RED}Usage: $0 <component> <target-server> [snapshot-name]${NC}"
    echo ""
    echo "Components:"
    echo "  proxmox        - Proxmox system (/var/lib/pve)"
    echo "  blockchain-core - Blockchain core system (/var/lib/op-dbus/blockchain)"
    echo "  privacy-network - Privacy network cache (/var/lib/op-dbus/@cache)"
    echo "  op-dbus-config  - op-dbus configuration (/etc/op-dbus)"
    echo ""
    echo "Examples:"
    echo "  $0 proxmox target-server.example.com"
    echo "  $0 blockchain-core server2 'blockchain-core_20241116_120000_source'"
    exit 1
fi

echo -e "${BLUE}=== BTRFS Deployment Sender ===${NC}"
echo ""
echo "Component: $COMPONENT"
echo "Target: $TARGET_SERVER"
if [ -n "$SNAPSHOT_NAME" ]; then
    echo "Snapshot: $SNAPSHOT_NAME"
fi
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}✗${NC} This script must be run as root"
    exit 1
fi

# Find the latest snapshot for the component
find_snapshot() {
    local component="$1"
    local requested_name="$2"

    if [ -n "$requested_name" ]; then
        # Use specific snapshot name
        local snapshot_path="$SNAPSHOT_BASE/$requested_name"
        if [ -d "$snapshot_path" ]; then
            echo "$snapshot_path"
            return 0
        else
            echo -e "${RED}✗${NC} Requested snapshot not found: $requested_name"
            exit 1
        fi
    else
        # Find latest snapshot for component
        local latest=$(ls -dt "$SNAPSHOT_BASE/${component}_"* 2>/dev/null | head -1)
        if [ -z "$latest" ]; then
            echo -e "${RED}✗${NC} No snapshots found for component: $component"
            exit 1
        fi
        echo "$latest"
    fi
}

# Verify target server is accessible
verify_target_server() {
    local server="$1"

    echo -e "${CYAN}Verifying target server access...${NC}"
    if ! ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -i "$SSH_KEY" "${SSH_USER}@${server}" "echo 'SSH access confirmed'" &>/dev/null; then
        echo -e "${RED}✗${NC} Cannot access target server: $server"
        echo "  Make sure SSH key is set up and server is reachable"
        exit 1
    fi
    echo -e "${GREEN}✓${NC} Target server accessible: $server"
}

# Prepare target server for receive
prepare_target_server() {
    local server="$1"
    local component="$2"

    echo -e "${CYAN}Preparing target server for $component...${NC}"

    # Define target paths based on component
    local target_paths=(
        "proxmox:/var/lib/pve"
        "blockchain-core:/var/lib/op-dbus/blockchain"
        "privacy-network:/var/lib/op-dbus/@cache"
        "op-dbus-config:/etc/op-dbus"
    )

    local target_path=""
    for path_def in "${target_paths[@]}"; do
        if [[ $path_def == ${component}:* ]]; then
            target_path="${path_def#*:}"
            break
        fi
    done

    if [ -z "$target_path" ]; then
        echo -e "${RED}✗${NC} Unknown component: $component"
        exit 1
    fi

    echo "Target path: $target_path"

    # Run preparation commands on target server
    ssh -i "$SSH_KEY" "${SSH_USER}@${server}" << EOF
set -e

echo "Preparing target server for $component deployment..."

# Check if BTRFS filesystem
if ! df -T / | grep -q btrfs; then
    echo "ERROR: Target server root filesystem must be BTRFS"
    exit 1
fi

# Ensure target directory exists (but not as subvolume yet)
mkdir -p "$target_path"

# If target path exists and is a subvolume, delete it for clean deployment
if btrfs subvolume show "$target_path" &>/dev/null; then
    echo "Removing existing subvolume: $target_path"
    btrfs subvolume delete "$target_path"
fi

# Create parent directories
mkdir -p "$(dirname "$target_path")"

echo "Target server prepared for $component deployment"
EOF

    echo -e "${GREEN}✓${NC} Target server prepared"
}

# Send snapshot to target server
send_snapshot() {
    local snapshot_path="$1"
    local server="$2"
    local component="$3"

    local snapshot_name=$(basename "$snapshot_path")

    echo -e "${CYAN}Sending snapshot $snapshot_name to $server...${NC}"

    # Use btrfs send/receive for efficient transfer
    if btrfs send "$snapshot_path" | ssh -i "$SSH_KEY" "${SSH_USER}@${server}" btrfs receive "$(dirname "$snapshot_path")"; then
        echo -e "${GREEN}✓${NC} Snapshot sent successfully"

        # Get snapshot size info
        local size_info=$(du -sh "$snapshot_path" | cut -f1)
        echo -e "${BLUE}ℹ️${NC}  Transferred: $size_info"

        return 0
    else
        echo -e "${RED}✗${NC} Failed to send snapshot"
        return 1
    fi
}

# Verify deployment on target server
verify_deployment() {
    local server="$1"
    local snapshot_path="$2"
    local component="$3"

    echo -e "${CYAN}Verifying deployment on target server...${NC}"

    ssh -i "$SSH_KEY" "${SSH_USER}@${server}" << EOF
set -e

echo "Verifying $component deployment..."

# Check if snapshot was received
if ! btrfs subvolume show "$snapshot_path" &>/dev/null; then
    echo "ERROR: Snapshot not found on target server: $snapshot_path"
    exit 1
fi

# Get snapshot info
SNAPSHOT_INFO=\$(btrfs subvolume show "$snapshot_path")
SIZE_INFO=\$(du -sh "$snapshot_path" | cut -f1)

echo "Snapshot verified:"
echo "  Path: $snapshot_path"
echo "  Size: \$SIZE_INFO"
echo "  Component: $component"

echo "Deployment verification complete"
EOF

    echo -e "${GREEN}✓${NC} Deployment verified on target server"
}

# Main execution
echo -e "${CYAN}Starting deployment of $COMPONENT to $TARGET_SERVER...${NC}"

# Find snapshot to send
SNAPSHOT_PATH=$(find_snapshot "$COMPONENT" "$SNAPSHOT_NAME")
echo -e "${BLUE}ℹ️${NC}  Using snapshot: $(basename "$SNAPSHOT_PATH")"

# Verify target server access
verify_target_server "$TARGET_SERVER"

# Prepare target server
prepare_target_server "$TARGET_SERVER" "$COMPONENT"

# Send snapshot
if send_snapshot "$SNAPSHOT_PATH" "$TARGET_SERVER" "$COMPONENT"; then
    # Verify deployment
    verify_deployment "$TARGET_SERVER" "$SNAPSHOT_PATH" "$COMPONENT"

    echo ""
    echo -e "${GREEN}✓${NC} Deployment complete!"
    echo ""
    echo -e "${CYAN}Post-deployment steps on target server:${NC}"
    case "$COMPONENT" in
        "proxmox")
            echo "1. Restart pve services: systemctl restart pvedaemon pveproxy"
            echo "2. Check VM configs: qm list"
            ;;
        "blockchain-core")
            echo "1. Restart op-dbus: systemctl restart op-dbus"
            echo "2. Check blockchain status: op-dbus query --plugin netmaker"
            ;;
        "privacy-network")
            echo "1. Clear cache: op-dbus cache clear --all"
            echo "2. Restart privacy services"
            ;;
        "op-dbus-config")
            echo "1. Reload configuration: op-dbus apply /etc/op-dbus/state.json"
            ;;
    esac
    echo ""

else
    echo -e "${RED}✗${NC} Deployment failed!"
    exit 1
fi