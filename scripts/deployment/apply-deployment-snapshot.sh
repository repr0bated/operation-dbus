#!/bin/bash
# apply-deployment-snapshot.sh - Apply received snapshots on target deployment servers
#
# Usage: ./apply-deployment-snapshot.sh <component> [snapshot-name]
#
# This script runs on the target server after receiving snapshots via send-deployment-snapshot.sh
# It activates the deployed components and performs post-deployment configuration.

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SNAPSHOT_BASE="/var/lib/deployment-snapshots"

# Arguments
COMPONENT="$1"
SNAPSHOT_NAME="${2:-}"

# Component definitions
declare -A COMPONENT_PATHS=(
    ["proxmox"]="/var/lib/pve"
    ["blockchain-core"]="/var/lib/op-dbus/blockchain"
    ["privacy-network"]="/var/lib/op-dbus/@cache"
    ["op-dbus-config"]="/etc/op-dbus"
)

declare -A COMPONENT_SERVICES=(
    ["proxmox"]="pvedaemon pveproxy pvestatd"
    ["blockchain-core"]="op-dbus"
    ["privacy-network"]="op-dbus"
    ["op-dbus-config"]="op-dbus"
)

# Validate arguments
if [ $# -lt 1 ]; then
    echo -e "${RED}Usage: $0 <component> [snapshot-name]${NC}"
    echo ""
    echo "Components:"
    echo "  proxmox        - Proxmox system"
    echo "  blockchain-core - Blockchain core system"
    echo "  privacy-network - Privacy network components"
    echo "  op-dbus-config  - op-dbus configuration"
    echo ""
    echo "Examples:"
    echo "  $0 proxmox"
    echo "  $0 blockchain-core 'blockchain-core_20241116_120000_source'"
    exit 1
fi

echo -e "${BLUE}=== BTRFS Deployment Applier ===${NC}"
echo ""
echo "Component: $COMPONENT"
if [ -n "$SNAPSHOT_NAME" ]; then
    echo "Snapshot: $SNAPSHOT_NAME"
fi
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}✗${NC} This script must be run as root"
    exit 1
fi

# Find the snapshot to apply
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

# Backup existing component
backup_existing_component() {
    local component="$1"
    local target_path="$2"
    local timestamp=$(date +%Y%m%d_%H%M%S)

    echo -e "${CYAN}Backing up existing $component...${NC}"

    if [ -d "$target_path" ]; then
        local backup_path="${target_path}.backup.${timestamp}"

        # If it's a subvolume, create a snapshot backup
        if btrfs subvolume show "$target_path" &>/dev/null; then
            echo "Creating subvolume snapshot backup: $backup_path"
            btrfs subvolume snapshot "$target_path" "$backup_path"
        else
            echo "Creating directory backup: $backup_path"
            cp -a "$target_path" "$backup_path"
        fi

        echo -e "${GREEN}✓${NC} Backup created: $backup_path"
        echo "$backup_path"
    else
        echo -e "${BLUE}ℹ️${NC}  No existing component to backup"
        echo ""
    fi
}

# Apply snapshot
apply_snapshot() {
    local snapshot_path="$1"
    local target_path="$2"
    local component="$3"

    echo -e "${CYAN}Applying $component snapshot...${NC}"

    # Remove existing target if it exists
    if [ -d "$target_path" ]; then
        if btrfs subvolume show "$target_path" &>/dev/null; then
            echo "Removing existing subvolume: $target_path"
            btrfs subvolume delete "$target_path"
        else
            echo "Removing existing directory: $target_path"
            rm -rf "$target_path"
        fi
    fi

    # Create parent directories
    mkdir -p "$(dirname "$target_path")"

    # Create writable snapshot from read-only snapshot
    echo "Creating writable snapshot: $target_path"
    if btrfs subvolume snapshot "$snapshot_path" "$target_path"; then
        echo -e "${GREEN}✓${NC} Snapshot applied successfully"

        # Get size info
        local size_info=$(du -sh "$target_path" | cut -f1)
        echo -e "${BLUE}ℹ️${NC}  Size: $size_info"

        return 0
    else
        echo -e "${RED}✗${NC} Failed to apply snapshot"
        return 1
    fi
}

# Component-specific post-deployment configuration
configure_component() {
    local component="$1"
    local target_path="$2"

    echo -e "${CYAN}Configuring $component...${NC}"

    case "$component" in
        "proxmox")
            echo "Configuring Proxmox system..."

            # Ensure PVE services are enabled
            systemctl enable pvedaemon pveproxy pvestatd spiceproxy 2>/dev/null || true

            # Set correct permissions
            chown -R www-data:www-data /var/lib/pve 2>/dev/null || true
            chmod -R 755 /var/lib/pve 2>/dev/null || true

            echo -e "${GREEN}✓${NC} Proxmox configured"
            ;;

        "blockchain-core")
            echo "Configuring blockchain core system..."

            # Ensure op-dbus service is enabled
            systemctl enable op-dbus 2>/dev/null || true

            # Set correct permissions
            chown -R op-dbus:op-dbus "$target_path" 2>/dev/null || true
            chmod -R 755 "$target_path" 2>/dev/null || true

            echo -e "${GREEN}✓${NC} Blockchain core configured"
            ;;

        "privacy-network")
            echo "Configuring privacy network..."

            # Ensure cache permissions
            chown -R op-dbus:op-dbus "$target_path" 2>/dev/null || true
            chmod -R 755 "$target_path" 2>/dev/null || true

            echo -e "${GREEN}✓${NC} Privacy network configured"
            ;;

        "op-dbus-config")
            echo "Configuring op-dbus configuration..."

            # Ensure config permissions
            chown -R root:root "$target_path" 2>/dev/null || true
            chmod -R 755 "$target_path" 2>/dev/null || true

            # Reload systemd if config changed
            systemctl daemon-reload 2>/dev/null || true

            echo -e "${GREEN}✓${NC} op-dbus config configured"
            ;;
    esac
}

# Restart component services
restart_services() {
    local component="$1"

    echo -e "${CYAN}Restarting $component services...${NC}"

    local services="${COMPONENT_SERVICES[$component]}"

    if [ -n "$services" ]; then
        for service in $services; do
            echo "Restarting service: $service"
            if systemctl restart "$service" 2>/dev/null; then
                echo -e "${GREEN}✓${NC} Service restarted: $service"
            else
                echo -e "${YELLOW}⚠${NC}  Service restart failed: $service"
            fi
        done
    else
        echo -e "${BLUE}ℹ️${NC}  No services to restart for $component"
    fi
}

# Verify deployment
verify_deployment() {
    local component="$1"
    local target_path="$2"

    echo -e "${CYAN}Verifying $component deployment...${NC}"

    # Check if target exists and is a subvolume
    if ! btrfs subvolume show "$target_path" &>/dev/null; then
        echo -e "${RED}✗${NC} Target is not a valid BTRFS subvolume: $target_path"
        return 1
    fi

    # Component-specific verification
    case "$component" in
        "proxmox")
            if [ -f "$target_path/.version" ] && [ -d "$target_path/local-btrfs" ]; then
                echo -e "${GREEN}✓${NC} Proxmox structure verified"
            else
                echo -e "${YELLOW}⚠${NC}  Proxmox structure may be incomplete"
            fi
            ;;

        "blockchain-core")
            if [ -d "$target_path/timing" ] && [ -d "$target_path/vectors" ] && [ -d "$target_path/state" ]; then
                echo -e "${GREEN}✓${NC} Blockchain core structure verified"
            else
                echo -e "${YELLOW}⚠${NC}  Blockchain core structure may be incomplete"
            fi
            ;;

        "privacy-network")
            if [ -d "$target_path/embeddings" ] && [ -d "$target_path/blocks" ]; then
                echo -e "${GREEN}✓${NC} Privacy network structure verified"
            else
                echo -e "${YELLOW}⚠${NC}  Privacy network structure may be incomplete"
            fi
            ;;

        "op-dbus-config")
            if [ -f "$target_path/state.json" ]; then
                echo -e "${GREEN}✓${NC} op-dbus config structure verified"
            else
                echo -e "${YELLOW}⚠${NC}  op-dbus config structure may be incomplete"
            fi
            ;;
    esac

    # Check disk usage
    local usage=$(df -h "$target_path" | tail -1 | awk '{print $5}')
    echo -e "${BLUE}ℹ️${NC}  Disk usage: $usage"

    echo -e "${GREEN}✓${NC} Deployment verification complete"
}

# Main execution
echo -e "${CYAN}Starting deployment application for $COMPONENT...${NC}"

# Get component paths
TARGET_PATH="${COMPONENT_PATHS[$component]}"
if [ -z "$TARGET_PATH" ]; then
    echo -e "${RED}✗${NC} Unknown component: $component"
    exit 1
fi

echo "Target path: $TARGET_PATH"

# Find snapshot to apply
SNAPSHOT_PATH=$(find_snapshot "$COMPONENT" "$SNAPSHOT_NAME")
echo -e "${BLUE}ℹ️${NC}  Using snapshot: $(basename "$SNAPSHOT_PATH")"

# Backup existing component
BACKUP_PATH=$(backup_existing_component "$COMPONENT" "$TARGET_PATH")

# Apply snapshot
if apply_snapshot "$SNAPSHOT_PATH" "$TARGET_PATH" "$COMPONENT"; then
    # Configure component
    configure_component "$COMPONENT" "$TARGET_PATH"

    # Restart services
    restart_services "$COMPONENT"

    # Verify deployment
    verify_deployment "$COMPONENT" "$TARGET_PATH"

    echo ""
    echo -e "${GREEN}✓${NC} Deployment application complete!"
    echo ""
    echo -e "${CYAN}Component Status:${NC}"
    echo "  Component: $COMPONENT"
    echo "  Location: $TARGET_PATH"
    echo "  Backup: ${BACKUP_PATH:-None}"
    echo "  Snapshot: $(basename "$SNAPSHOT_PATH")"
    echo ""
    echo -e "${YELLOW}Note: You may need to manually start services or reboot for full activation${NC}"
    echo ""

else
    echo -e "${RED}✗${NC} Deployment application failed!"
    echo ""
    echo -e "${YELLOW}To rollback:${NC}"
    if [ -n "${BACKUP_PATH:-}" ]; then
        echo "  rm -rf $TARGET_PATH"
        echo "  mv $BACKUP_PATH $TARGET_PATH  # Restore backup"
    fi
    exit 1
fi