#!/bin/bash
# deploy-system.sh - Complete system deployment orchestration
#
# Usage: ./deploy-system.sh <target-server> [components...]
#
# This script orchestrates the complete deployment process:
# 1. Create snapshots of system components
# 2. Send snapshots to target server
# 3. Apply snapshots on target server
#
# Default components: proxmox blockchain-core privacy-network op-dbus-config

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_SERVER="$1"
shift
COMPONENTS=("$@")

# Default components if none specified
if [ ${#COMPONENTS[@]} -eq 0 ]; then
    COMPONENTS=("proxmox" "blockchain-core" "privacy-network" "op-dbus-config")
fi

echo -e "${BLUE}=== Complete System Deployment ===${NC}"
echo ""
echo "Target Server: $TARGET_SERVER"
echo "Components: ${COMPONENTS[*]}"
echo "Script Directory: $SCRIPT_DIR"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}✗${NC} This script must be run as root"
    exit 1
fi

# Validate target server
validate_target_server() {
    local server="$1"

    echo -e "${CYAN}Validating target server: $server${NC}"

    # Check SSH access
    if ! ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no -o BatchMode=yes "$server" "echo 'SSH access confirmed'" &>/dev/null; then
        echo -e "${RED}✗${NC} Cannot access target server via SSH: $server"
        echo "  Ensure SSH key authentication is set up"
        exit 1
    fi

    # Check if target has BTRFS root filesystem
    if ! ssh "$server" "df -T / | grep -q btrfs"; then
        echo -e "${RED}✗${NC} Target server root filesystem must be BTRFS"
        exit 1
    fi

    # Check available disk space (rough estimate)
    local remote_space=$(ssh "$server" "df -BG / | tail -1 | awk '{print \$4}' | sed 's/G//'")
    if [ "$remote_space" -lt 50 ]; then
        echo -e "${YELLOW}⚠${NC}  Target server has low disk space: ${remote_space}GB available"
        echo "  Consider freeing up space before deployment"
    fi

    echo -e "${GREEN}✓${NC} Target server validated"
}

# Create deployment snapshots
create_snapshots() {
    echo ""
    echo -e "${CYAN}=== Phase 1: Creating Deployment Snapshots ===${NC}"

    if ! "$SCRIPT_DIR/create-deployment-snapshots.sh"; then
        echo -e "${RED}✗${NC} Failed to create deployment snapshots"
        exit 1
    fi

    echo -e "${GREEN}✓${NC} Deployment snapshots created"
}

# Send snapshots to target
send_snapshots() {
    echo ""
    echo -e "${CYAN}=== Phase 2: Sending Snapshots to Target ===${NC}"

    for component in "${COMPONENTS[@]}"; do
        echo ""
        echo -e "${BLUE}Sending $component snapshot...${NC}"

        if ! "$SCRIPT_DIR/send-deployment-snapshot.sh" "$component" "$TARGET_SERVER"; then
            echo -e "${RED}✗${NC} Failed to send $component snapshot"
            exit 1
        fi

        echo -e "${GREEN}✓${NC} $component snapshot sent"
    done

    echo -e "${GREEN}✓${NC} All snapshots sent to target"
}

# Apply snapshots on target
apply_snapshots() {
    echo ""
    echo -e "${CYAN}=== Phase 3: Applying Snapshots on Target ===${NC}"

    # Copy apply script to target
    echo "Copying deployment scripts to target server..."
    scp "$SCRIPT_DIR/apply-deployment-snapshot.sh" "$TARGET_SERVER:/tmp/"

    # Apply each component
    for component in "${COMPONENTS[@]}"; do
        echo ""
        echo -e "${BLUE}Applying $component on target server...${NC}"

        if ! ssh "$TARGET_SERVER" "bash /tmp/apply-deployment-snapshot.sh '$component'"; then
            echo -e "${RED}✗${NC} Failed to apply $component on target"
            exit 1
        fi

        echo -e "${GREEN}✓${NC} $component applied on target"
    done

    echo -e "${GREEN}✓${NC} All components applied on target"
}

# Post-deployment verification
verify_deployment() {
    echo ""
    echo -e "${CYAN}=== Phase 4: Post-Deployment Verification ===${NC}"

    echo "Running verification on target server..."

    # Run verification script on target
    ssh "$TARGET_SERVER" << 'EOF'
#!/bin/bash
set -e

echo "=== Target Server Verification ==="

# Check system health
echo "System Information:"
uname -a
echo ""

# Check BTRFS filesystems
echo "BTRFS Filesystems:"
btrfs filesystem show || echo "No BTRFS filesystems found"
echo ""

# Check deployed components
echo "Deployed Components:"

# Proxmox
if [ -d "/var/lib/pve" ]; then
    echo "✓ Proxmox: $(du -sh /var/lib/pve | cut -f1)"
else
    echo "✗ Proxmox: Not found"
fi

# Blockchain core
if [ -d "/var/lib/op-dbus/blockchain" ]; then
    echo "✓ Blockchain Core: $(du -sh /var/lib/op-dbus/blockchain | cut -f1)"
    echo "  Subvolumes: $(ls /var/lib/op-dbus/blockchain/ | wc -l)"
else
    echo "✗ Blockchain Core: Not found"
fi

# Privacy network
if [ -d "/var/lib/op-dbus/@cache" ]; then
    echo "✓ Privacy Network: $(du -sh /var/lib/op-dbus/@cache | cut -f1)"
else
    echo "✗ Privacy Network: Not found"
fi

# op-dbus config
if [ -d "/etc/op-dbus" ]; then
    echo "✓ op-dbus Config: $(du -sh /etc/op-dbus | cut -f1)"
else
    echo "✗ op-dbus Config: Not found"
fi

echo ""
echo "Service Status:"
systemctl is-active op-dbus 2>/dev/null && echo "✓ op-dbus: Running" || echo "⚠ op-dbus: Not running"

echo ""
echo "=== Verification Complete ==="
EOF

    echo -e "${GREEN}✓${NC} Deployment verification complete"
}

# Show deployment summary
show_summary() {
    echo ""
    echo -e "${GREEN}🎉 System Deployment Complete!${NC}"
    echo ""
    echo -e "${CYAN}Deployment Summary:${NC}"
    echo "  Target Server: $TARGET_SERVER"
    echo "  Components: ${COMPONENTS[*]}"
    echo "  Timestamp: $(date)"
    echo ""
    echo -e "${CYAN}Next Steps on Target Server:${NC}"

    # Component-specific next steps
    if [[ " ${COMPONENTS[*]} " =~ " proxmox " ]]; then
        echo "  • Proxmox: Access web UI at https://$TARGET_SERVER:8006"
        echo "  • Proxmox: Run 'pvecm updatecerts' to update certificates"
    fi

    if [[ " ${COMPONENTS[*]} " =~ " blockchain-core " ]]; then
        echo "  • Blockchain: Check op-dbus status with 'systemctl status op-dbus'"
        echo "  • Blockchain: View logs with 'journalctl -u op-dbus -f'"
    fi

    if [[ " ${COMPONENTS[*]} " =~ " privacy-network " ]]; then
        echo "  • Privacy: Verify cache with 'op-dbus cache stats'"
        echo "  • Privacy: Check OpenFlow with 'ovs-ofctl dump-flows vmbr0'"
    fi

    echo ""
    echo "  • General: Reboot target server for full activation"
    echo "  • General: Run 'op-dbus verify' to check system health"
    echo ""
}

# Main execution
echo -e "${CYAN}Starting complete system deployment...${NC}"

# Validate target server
validate_target_server "$TARGET_SERVER"

# Phase 1: Create snapshots
create_snapshots

# Phase 2: Send snapshots
send_snapshots

# Phase 3: Apply snapshots
apply_snapshots

# Phase 4: Verify deployment
verify_deployment

# Show summary
show_summary

echo ""
echo -e "${GREEN}✓${NC} Complete system deployment finished successfully!"
echo ""
echo "Target server $TARGET_SERVER is now ready with all deployed components."