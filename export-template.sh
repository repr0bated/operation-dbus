#!/bin/bash
# Export an existing LXC container as a netmaker-ready template
# Usage: ./export-template.sh <container-id>

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Export LXC Container as Template ==="
echo ""

# Get container ID from argument
CT_ID="${1:-9999}"
OUTPUT_TEMPLATE="debian-13-netmaker_custom.tar.zst"
STORAGE="local-btrfs"

echo "Configuration:"
echo "  Container ID: $CT_ID"
echo "  Output template: $OUTPUT_TEMPLATE"
echo "  Storage: $STORAGE"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo)${NC}"
    exit 1
fi

# Check if container exists
if ! pct status $CT_ID >/dev/null 2>&1; then
    echo -e "${RED}✗${NC} Container $CT_ID not found"
    echo ""
    echo "Available containers:"
    pct list
    exit 1
fi

echo -e "${GREEN}✓${NC} Container $CT_ID found"

# Check if container is running
if pct status $CT_ID | grep -q "running"; then
    echo -e "${YELLOW}⚠${NC}  Container is running, stopping it..."
    pct stop $CT_ID
    sleep 3
fi

echo -e "${GREEN}✓${NC} Container stopped"

# Clean up netclient state in container before templating
echo "Cleaning netclient state (to preserve first-boot behavior)..."
pct start $CT_ID
sleep 3

pct exec $CT_ID -- rm -rf /etc/netclient 2>/dev/null || true
pct exec $CT_ID -- rm -rf /root/.netclient 2>/dev/null || true
pct exec $CT_ID -- rm -f /var/log/netclient.log 2>/dev/null || true
pct exec $CT_ID -- apt-get clean 2>/dev/null || true
pct exec $CT_ID -- rm -rf /var/lib/apt/lists/* 2>/dev/null || true
pct exec $CT_ID -- rm -rf /tmp/* 2>/dev/null || true
pct exec $CT_ID -- bash -c 'for log in /var/log/*.log; do > "$log"; done' 2>/dev/null || true
pct exec $CT_ID -- history -c 2>/dev/null || true

echo -e "${GREEN}✓${NC} Container cleaned"

# Stop container
pct stop $CT_ID
sleep 3

# Create template archive
echo "Creating template archive..."
OUTPUT_PATH="/var/lib/pve/$STORAGE/template/cache/$OUTPUT_TEMPLATE"

# Create template directory if needed
mkdir -p "/var/lib/pve/$STORAGE/template/cache"

# Export container as template
# Method 1: Use vzdump to create proper template
echo "Exporting container $CT_ID as template..."

if vzdump $CT_ID --mode stop --compress zstd --storage $STORAGE --dumpdir /tmp; then
    # Find the dump file
    DUMP_FILE=$(ls -t /tmp/vzdump-lxc-${CT_ID}-*.tar.zst 2>/dev/null | head -1)
    
    if [ -f "$DUMP_FILE" ]; then
        # Move and rename to template location
        mv "$DUMP_FILE" "$OUTPUT_PATH"
        echo -e "${GREEN}✓${NC} Template created: $OUTPUT_PATH"
    else
        echo -e "${RED}✗${NC} Dump file not found"
        exit 1
    fi
else
    echo -e "${RED}✗${NC} vzdump failed"
    exit 1
fi

# Cleanup dump files
rm -f /tmp/vzdump-lxc-${CT_ID}-*.{log,tmp} 2>/dev/null || true

# Verify template
if [ -f "$OUTPUT_PATH" ]; then
    TEMPLATE_SIZE=$(du -h "$OUTPUT_PATH" | cut -f1)
    echo -e "${GREEN}✓${NC} Template size: $TEMPLATE_SIZE"
    
    echo ""
    echo "=== Template Export Complete ==="
    echo ""
    echo "Template: $OUTPUT_TEMPLATE"
    echo "Location: $OUTPUT_PATH"
    echo "Storage: $STORAGE"
    echo ""
    echo "Usage in state.json:"
    echo '  "template": "local-btrfs:vztmpl/debian-13-netmaker_custom.tar.zst"'
    echo ""
    echo "Containers created from this template will:"
    echo "  ✓ Have netclient pre-installed"
    echo "  ✓ Join netmaker network on first boot (via hook)"
    echo "  ✓ Get unique identity (no baked-in config)"
else
    echo -e "${RED}✗${NC} Template creation failed"
    exit 1
fi

