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
    # Try adding /usr/sbin to PATH
    export PATH="/usr/sbin:$PATH"
    if ! command -v pct >/dev/null 2>&1; then
        echo -e "${RED}✗${NC} pct command not found. This must run on a Proxmox host."
        exit 1
    fi
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

# Check if netclient binary exists locally
NETCLIENT_BINARY=""
for path in ./netclient /tmp/netclient /usr/local/bin/netclient /sbin/netclient; do
    if [ -f "$path" ]; then
        NETCLIENT_BINARY="$path"
        echo -e "${GREEN}✓${NC} Found netclient at: $path"
        break
    fi
done

if [ -z "$NETCLIENT_BINARY" ]; then
    echo -e "${YELLOW}⚠${NC}  No local netclient found, will try to download..."
    
    # Wait for network
    echo "Waiting for network (timeout 30s)..."
    pct exec $TEMP_CT_ID -- bash -c 'for i in {1..30}; do ping -c1 8.8.8.8 &>/dev/null && exit 0; sleep 1; done; exit 1' || {
        echo -e "${RED}✗${NC} Container network not available"
        echo -e "${RED}✗${NC} Cannot create template without netclient or network"
        pct stop $TEMP_CT_ID
        pct destroy $TEMP_CT_ID
        exit 1
    }
    
    echo -e "${GREEN}✓${NC} Container network ready"
    
    # Update system
    echo "Updating system packages..."
    pct exec $TEMP_CT_ID -- apt-get update
    pct exec $TEMP_CT_ID -- apt-get upgrade -y
    
    # Install dependencies
    echo "Installing dependencies..."
    pct exec $TEMP_CT_ID -- apt-get install -y curl ca-certificates
    
    # Install netclient (direct binary method)
    echo "Downloading netclient..."
    pct exec $TEMP_CT_ID -- wget -O /tmp/netclient https://fileserver.netmaker.io/releases/download/v1.1.0/netclient-linux-amd64
    pct exec $TEMP_CT_ID -- chmod +x /tmp/netclient
    pct exec $TEMP_CT_ID -- /tmp/netclient install
else
    echo "Installing netclient from host..."
    pct push $TEMP_CT_ID "$NETCLIENT_BINARY" /tmp/netclient
    pct exec $TEMP_CT_ID -- chmod +x /tmp/netclient
    pct exec $TEMP_CT_ID -- /tmp/netclient install
fi

# Verify netclient installation
if pct exec $TEMP_CT_ID -- which netclient >/dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} netclient installed successfully"
    NETCLIENT_VERSION=$(pct exec $TEMP_CT_ID -- netclient --version 2>&1 | head -1)
    echo "  Version: $NETCLIENT_VERSION"
else
    echo -e "${RED}✗${NC} netclient installation failed"
    pct stop $TEMP_CT_ID
    pct destroy $TEMP_CT_ID
    exit 1
fi

echo -e "${GREEN}✓${NC} netclient installation complete"

# Install first-boot script to join netmaker
echo "Installing first-boot netmaker join script..."

# Check if netmaker token exists on host
NETMAKER_TOKEN=""
if [ -f "/etc/op-dbus/netmaker.env" ]; then
    source /etc/op-dbus/netmaker.env
    if [ -n "$NETMAKER_TOKEN" ]; then
        echo -e "${GREEN}✓${NC} Found Netmaker token on host, injecting into template..."
        pct exec $TEMP_CT_ID -- bash -c "echo 'NETMAKER_TOKEN=$NETMAKER_TOKEN' > /etc/netmaker.env"
        pct exec $TEMP_CT_ID -- chmod 600 /etc/netmaker.env
        echo -e "${GREEN}✓${NC} Token stored in template at /etc/netmaker.env"
    else
        echo -e "${YELLOW}⚠${NC}  NETMAKER_TOKEN not set in /etc/op-dbus/netmaker.env"
        echo -e "${YELLOW}⚠${NC}  Template will be created without token"
    fi
else
    echo -e "${YELLOW}⚠${NC}  /etc/op-dbus/netmaker.env not found"
    echo -e "${YELLOW}⚠${NC}  Template will be created without token"
fi

pct exec $TEMP_CT_ID -- tee /usr/local/bin/netmaker-first-boot.sh > /dev/null <<'FIRSTBOOT_EOF'
#!/bin/bash
# First boot script to join netmaker network
# Runs once on first boot, then disables itself

NETMAKER_TOKEN_FILE="/etc/netmaker.env"
MARKER_FILE="/var/lib/netmaker-joined"

# Exit if already joined
if [ -f "$MARKER_FILE" ]; then
    exit 0
fi

# Wait for network to be ready
sleep 5

# Read token from env file
if [ ! -f "$NETMAKER_TOKEN_FILE" ]; then
    echo "No netmaker token found at $NETMAKER_TOKEN_FILE"
    exit 0
fi

source "$NETMAKER_TOKEN_FILE"

if [ -z "$NETMAKER_TOKEN" ]; then
    echo "NETMAKER_TOKEN not set in $NETMAKER_TOKEN_FILE"
    exit 0
fi

# Join netmaker
echo "Joining netmaker network..."
if netclient join -t "$NETMAKER_TOKEN"; then
    echo "Successfully joined netmaker network"
    touch "$MARKER_FILE"
else
    echo "Failed to join netmaker network"
    exit 1
fi
FIRSTBOOT_EOF

pct exec $TEMP_CT_ID -- chmod +x /usr/local/bin/netmaker-first-boot.sh

# Create systemd service for first boot
pct exec $TEMP_CT_ID -- tee /etc/systemd/system/netmaker-first-boot.service > /dev/null <<'SERVICE_EOF'
[Unit]
Description=Netmaker First Boot Join
After=network-online.target
Wants=network-online.target
ConditionPathExists=!/var/lib/netmaker-joined

[Service]
Type=oneshot
ExecStart=/usr/local/bin/netmaker-first-boot.sh
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
SERVICE_EOF

# Enable the service
pct exec $TEMP_CT_ID -- systemctl enable netmaker-first-boot.service

echo -e "${GREEN}✓${NC} First-boot join script installed and enabled"
echo -e "${YELLOW}Note:${NC} Containers will auto-join netmaker on first boot when token is provided"

# Clean up
echo "Cleaning up container..."
pct exec $TEMP_CT_ID -- apt-get clean
pct exec $TEMP_CT_ID -- rm -rf /var/lib/apt/lists/*
pct exec $TEMP_CT_ID -- rm -rf /tmp/*
pct exec $TEMP_CT_ID -- rm -f /var/log/*.log
# history -c doesn't work with pct exec, skip it

# CRITICAL: Remove any netclient state to ensure fresh join on first boot
echo "Removing netclient state (to ensure fresh join)..."
pct exec $TEMP_CT_ID -- rm -rf /etc/netclient 2>/dev/null || true
pct exec $TEMP_CT_ID -- rm -rf /root/.netclient 2>/dev/null || true
pct exec $TEMP_CT_ID -- rm -f /var/log/netclient.log 2>/dev/null || true
# Keep /etc/netmaker.env - it contains the token for auto-join
echo -e "${GREEN}✓${NC} netclient state cleared (token preserved at /etc/netmaker.env)"

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
echo "Creating template archive (this may take a few minutes)..."
cd /var/lib/lxc/$TEMP_CT_ID
# Use zstd compression (tar with -I zstd) for .tar.zst format
if command -v zstd >/dev/null 2>&1; then
    tar -I zstd -cf "$OUTPUT_TEMPLATE_PATH" rootfs/
else
    echo -e "${YELLOW}⚠${NC}  zstd not found, using gzip"
    tar czf "${OUTPUT_TEMPLATE_PATH%.zst}.gz" rootfs/
    OUTPUT_TEMPLATE_PATH="${OUTPUT_TEMPLATE_PATH%.zst}.gz"
fi

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
