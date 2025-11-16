#!/bin/bash
# create-deployment-snapshots.sh - Create BTRFS snapshots of system components for deployment
#
# This script creates snapshots of:
# 1. Proxmox system (/var/lib/pve)
# 2. Blockchain core system (/var/lib/op-dbus/blockchain)
# 3. Privacy network components (/var/lib/op-dbus/@cache, /etc/op-dbus)
#
# Snapshots are stored in /var/lib/deployment-snapshots and can be sent to target servers

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SNAPSHOT_BASE="/var/lib/deployment-snapshots"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
HOSTNAME=$(hostname)

# Component definitions
declare -A COMPONENTS=(
    ["proxmox"]="/var/lib/pve"
    ["blockchain-core"]="/var/lib/op-dbus/blockchain"
    ["privacy-network"]="/var/lib/op-dbus/@cache"
    ["op-dbus-config"]="/etc/op-dbus"
)

echo -e "${BLUE}=== BTRFS Deployment Snapshot Creator ===${NC}"
echo ""
echo "Host: $HOSTNAME"
echo "Timestamp: $TIMESTAMP"
echo "Snapshot Base: $SNAPSHOT_BASE"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}✗${NC} This script must be run as root"
    exit 1
fi

# Check if BTRFS filesystem
if ! df -T / | grep -q btrfs; then
    echo -e "${RED}✗${NC} Root filesystem must be BTRFS for snapshots"
    exit 1
fi

# Create snapshot base directory
echo -e "${CYAN}Creating snapshot directory...${NC}"
mkdir -p "$SNAPSHOT_BASE"
echo -e "${GREEN}✓${NC} Snapshot base: $SNAPSHOT_BASE"

# Function to create component snapshot
create_component_snapshot() {
    local component_name="$1"
    local source_path="$2"
    local snapshot_name="${component_name}_${TIMESTAMP}_${HOSTNAME}"
    local snapshot_path="$SNAPSHOT_BASE/$snapshot_name"

    echo ""
    echo -e "${CYAN}=== Creating $component_name snapshot ===${NC}"

    # Check if source exists
    if [ ! -d "$source_path" ]; then
        echo -e "${YELLOW}⚠${NC}  Source path does not exist: $source_path"
        return 1
    fi

    # Check if source is a BTRFS subvolume
    if ! btrfs subvolume show "$source_path" &>/dev/null; then
        echo -e "${YELLOW}⚠${NC}  Source is not a BTRFS subvolume: $source_path"
        echo "  Converting to subvolume first..."

        # Create temporary subvolume and copy data
        local temp_subvol="${source_path}.temp_subvol"
        btrfs subvolume create "$temp_subvol"
        cp -a "$source_path"/* "$temp_subvol/" 2>/dev/null || true
        mv "$source_path" "${source_path}.backup"
        mv "$temp_subvol" "$source_path"

        echo -e "${GREEN}✓${NC}  Converted to BTRFS subvolume"
    fi

    # Create snapshot
    echo "Creating snapshot: $snapshot_path"
    if btrfs subvolume snapshot -r "$source_path" "$snapshot_path"; then
        echo -e "${GREEN}✓${NC}  Snapshot created: $snapshot_path"

        # Get snapshot size
        local size_info=$(du -sh "$snapshot_path" 2>/dev/null || echo "unknown size")
        echo -e "${BLUE}ℹ️${NC}  Size: $size_info"

        return 0
    else
        echo -e "${RED}✗${NC}  Failed to create snapshot: $snapshot_path"
        return 1
    fi
}

# Function to clean old snapshots
cleanup_old_snapshots() {
    local component_name="$1"
    local keep_count="${2:-5}"

    echo ""
    echo -e "${CYAN}=== Cleaning old $component_name snapshots (keeping $keep_count) ===${NC}"

    # Find and sort snapshots for this component
    local snapshots=($(ls -dt "$SNAPSHOT_BASE/${component_name}_"* 2>/dev/null || true))
    local count=${#snapshots[@]}

    if [ $count -le $keep_count ]; then
        echo -e "${BLUE}ℹ️${NC}  Only $count snapshots exist, keeping all"
        return 0
    fi

    # Delete old snapshots
    local to_delete=$((count - keep_count))
    for ((i=keep_count; i<count; i++)); do
        local snapshot="${snapshots[$i]}"
        echo "Deleting old snapshot: $snapshot"
        if btrfs subvolume delete "$snapshot"; then
            echo -e "${GREEN}✓${NC}  Deleted: $snapshot"
        else
            echo -e "${YELLOW}⚠${NC}  Failed to delete: $snapshot"
        fi
    done
}

# Function to show snapshot info
show_snapshot_info() {
    echo ""
    echo -e "${CYAN}=== Deployment Snapshot Summary ===${NC}"
    echo ""

    local total_size=0
    local snapshot_count=0

    for component in "${!COMPONENTS[@]}"; do
        echo -e "${BLUE}$component:${NC}"

        # List snapshots for this component
        local snapshots=($(ls -dt "$SNAPSHOT_BASE/${component}_"* 2>/dev/null || true))
        local count=${#snapshots[@]}

        if [ $count -eq 0 ]; then
            echo "  No snapshots"
            continue
        fi

        # Show latest snapshot
        local latest="${snapshots[0]}"
        local size_info=$(du -sh "$latest" 2>/dev/null | cut -f1 || echo "unknown")
        local mtime=$(stat -c %y "$latest" 2>/dev/null | cut -d'.' -f1 || echo "unknown")

        echo "  Latest: $(basename "$latest")"
        echo "  Size: $size_info"
        echo "  Created: $mtime"
        echo "  Total snapshots: $count"

        # Add to totals
        local size_bytes=$(du -b "$latest" 2>/dev/null | cut -f1 || echo "0")
        total_size=$((total_size + size_bytes))
        snapshot_count=$((snapshot_count + count))

        echo ""
    done

    # Show totals
    local total_size_human=$(numfmt --to=iec-i --suffix=B $total_size 2>/dev/null || echo "${total_size}B")
    echo -e "${CYAN}Total snapshots: $snapshot_count${NC}"
    echo -e "${CYAN}Total size: $total_size_human${NC}"
}

# Main execution
echo -e "${CYAN}Starting deployment snapshot creation...${NC}"

# Create snapshots for each component
for component in "${!COMPONENTS[@]}"; do
    if create_component_snapshot "$component" "${COMPONENTS[$component]}"; then
        # Clean up old snapshots for this component
        cleanup_old_snapshots "$component" 3  # Keep last 3 snapshots
    fi
done

# Show final status
show_snapshot_info

echo ""
echo -e "${GREEN}✓${NC} Deployment snapshot creation complete!"
echo ""
echo -e "${CYAN}Next steps:${NC}"
echo "1. Review snapshots in: $SNAPSHOT_BASE"
echo "2. Use send-deployment-snapshot.sh to deploy to target servers"
echo "3. Example: ./send-deployment-snapshot.sh proxmox target-server.example.com"
echo ""