#!/bin/bash
# Create LXC template with netclient pre-installed for op-dbus containers
# This template will be used by the LXC plugin for automatic container creation

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=== Creating Netmaker-Ready LXC Template ==="

# Check if running on Proxmox
if ! command -v pct >/dev/null 2>&1; then
    echo -e "${RED}✗${NC} pct command not found. This must run on a Proxmox host."
    exit 1
fi

# Configuration
TEMP_CT_ID=9999
BASE_TEMPLATE="debian-13-standard_13.1-2_amd64.tar.zst"
BASE_TEMPLATE_URL="http://download.proxmox.com/images/system/debian-13-standard_13.1-2_amd64.tar.zst"
OUTPUT_TEMPLATE="debian-13-netmaker_custom.tar.zst"
STORAGE="local-btrfs"

echo "Configuration:"
echo "  Base template: $BASE_TEMPLATE"
echo "  Output template: $OUTPUT_TEMPLATE"
echo "  Temp container ID: $TEMP_CT_ID"
echo ""

# Check if base template exists
TEMPLATE_PATH="/var/lib/pve/$STORAGE/template/cache/$BASE_TEMPLATE"
if [ ! -f "$TEMPLATE_PATH" ]; then
    echo -e "${YELLOW}⚠${NC}  Base template not found, downloading from Proxmox CDN..."
    echo "Downloading: $BASE_TEMPLATE_URL"
    
    # Create template directory if needed
    mkdir -p "/var/lib/pve/$STORAGE/template/cache"
    
    # Download template directly
    if curl -fL "$BASE_TEMPLATE_URL" -o "$TEMPLATE_PATH"; then
        echo -e "${GREEN}✓${NC} Downloaded base template"
    else
        echo -e "${RED}✗${NC} Failed to download template"
        echo "Trying with pveam as fallback..."
        pveam update
        pveam download $STORAGE $BASE_TEMPLATE || {
            echo -e "${RED}✗${NC} Failed to download template via pveam"
            exit 1
        }
    fi
else
    echo -e "${GREEN}✓${NC} Base template already available"
fi

echo -e "${GREEN}✓${NC} Base template ready: $TEMPLATE_PATH"

# Check if temp container already exists
if pct status $TEMP_CT_ID >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC}  Container $TEMP_CT_ID already exists, destroying..."
    pct stop $TEMP_CT_ID 2>/dev/null || true
    pct destroy $TEMP_CT_ID
fi

# Create temporary container
echo "Creating temporary container $TEMP_CT_ID..."
pct create $TEMP_CT_ID $STORAGE:vztmpl/$BASE_TEMPLATE \
    --hostname netmaker-template \
    --memory 512 \
    --swap 512 \
    --rootfs $STORAGE:8 \
    --net0 name=eth0,bridge=vmbr0,firewall=1,ip=dhcp \
    --unprivileged 1 \
    --features nesting=1

echo -e "${GREEN}✓${NC} Temporary container created"

# Start container
echo "Starting container..."
pct start $TEMP_CT_ID

# Wait for container to be ready
echo "Waiting for container to boot..."
sleep 5

# Wait for network
pct exec $TEMP_CT_ID -- bash -c 'until ping -c1 8.8.8.8 &>/dev/null; do sleep 1; done'
echo -e "${GREEN}✓${NC} Container network ready"

# Update system
echo "Updating system packages..."
pct exec $TEMP_CT_ID -- apt-get update
pct exec $TEMP_CT_ID -- apt-get upgrade -y

# Install dependencies
echo "Installing dependencies..."
pct exec $TEMP_CT_ID -- apt-get install -y \
    curl \
    gnupg \
    ca-certificates \
    wireguard \
    jq \
    systemd

# Install netclient
echo "Installing netclient..."
pct exec $TEMP_CT_ID -- bash -c 'curl -sL https://apt.netmaker.org/gpg.key | apt-key add -'
pct exec $TEMP_CT_ID -- bash -c 'curl -sL https://apt.netmaker.org/debian.deb.txt | tee /etc/apt/sources.list.d/netmaker.list'
pct exec $TEMP_CT_ID -- apt-get update
pct exec $TEMP_CT_ID -- apt-get install -y netclient

# Verify netclient installation
if pct exec $TEMP_CT_ID -- which netclient >/dev/null; then
    echo -e "${GREEN}✓${NC} netclient installed successfully"
    NETCLIENT_VERSION=$(pct exec $TEMP_CT_ID -- netclient --version 2>&1 | head -1)
    echo "  Version: $NETCLIENT_VERSION"
else
    echo -e "${RED}✗${NC} netclient installation failed"
    pct stop $TEMP_CT_ID
    pct destroy $TEMP_CT_ID
    exit 1
fi

# Clean up
echo "Cleaning up container..."
pct exec $TEMP_CT_ID -- apt-get clean
pct exec $TEMP_CT_ID -- rm -rf /var/lib/apt/lists/*
pct exec $TEMP_CT_ID -- rm -rf /tmp/*
pct exec $TEMP_CT_ID -- rm -f /var/log/*.log
pct exec $TEMP_CT_ID -- history -c

# CRITICAL: Remove any netclient state to ensure fresh join on first boot
echo "Removing netclient state (to ensure fresh join)..."
pct exec $TEMP_CT_ID -- rm -rf /etc/netclient 2>/dev/null || true
pct exec $TEMP_CT_ID -- rm -rf /root/.netclient 2>/dev/null || true
pct exec $TEMP_CT_ID -- rm -f /var/log/netclient.log 2>/dev/null || true
echo -e "${GREEN}✓${NC} netclient state cleared (containers will join fresh on first boot)"

# Stop container
echo "Stopping container..."
pct stop $TEMP_CT_ID

# Wait for container to fully stop
sleep 3

# Create template from container
echo "Creating template from container..."
ROOTFS_PATH="/var/lib/lxc/$TEMP_CT_ID/rootfs"
OUTPUT_TEMPLATE_PATH="/var/lib/pve/$STORAGE/template/cache/$OUTPUT_TEMPLATE"

# Create archive
echo "Creating template archive..."
cd /var/lib/lxc/$TEMP_CT_ID
tar czf "$OUTPUT_TEMPLATE_PATH" rootfs/

echo -e "${GREEN}✓${NC} Template created: $OUTPUT_TEMPLATE_PATH"

# Cleanup temporary container
echo "Cleaning up temporary container..."
pct destroy $TEMP_CT_ID

# Verify template
if [ -f "$OUTPUT_TEMPLATE_PATH" ]; then
    TEMPLATE_SIZE=$(du -h "$OUTPUT_TEMPLATE_PATH" | cut -f1)
    echo -e "${GREEN}✓${NC} Template size: $TEMPLATE_SIZE"

    # List templates
    echo ""
    echo "Available templates:"
    pveam list $STORAGE | grep -E "debian.*netmaker|debian.*standard"
else
    echo -e "${RED}✗${NC} Template creation failed"
    exit 1
fi

echo ""
echo "=== Template Creation Complete ==="
echo ""
echo "Template: $OUTPUT_TEMPLATE"
echo "Location: $OUTPUT_TEMPLATE_PATH"
echo ""
echo "Next steps:"
echo "1. Update src/state/plugins/lxc.rs to use: $OUTPUT_TEMPLATE"
echo "2. Rebuild op-dbus: cargo build --release"
echo "3. Reinstall: sudo ./install.sh"
echo ""
echo "Containers created from this template will have:"
echo "  ✓ netclient pre-installed"
echo "  ✓ wireguard support"
echo "  ✓ Ready to join netmaker networks"
