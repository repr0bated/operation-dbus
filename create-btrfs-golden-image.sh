#!/bin/bash
# create-btrfs-golden-image.sh - Create BTRFS subvolume golden images for instant container provisioning
#
# This script creates reusable BTRFS subvolume templates that can be instantly
# snapshotted to create new containers (milliseconds vs 30+ seconds for tar extraction)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
GOLDEN_IMAGE_NAME="${1:-debian-minimal}"
TEMP_CT_ID="${2:-9998}"
BASE_TEMPLATE="${3:-local-btrfs:vztmpl/debian-13-standard_13.1-2_amd64.tar.zst}"

STORAGE="local-btrfs"
STORAGE_PATH="/var/lib/pve/$STORAGE"
GOLDEN_IMAGE_DIR="$STORAGE_PATH/templates/subvol"
GOLDEN_IMAGE_PATH="$GOLDEN_IMAGE_DIR/$GOLDEN_IMAGE_NAME"

echo -e "${BLUE}=== BTRFS Golden Image Creator ===${NC}"
echo ""
echo "Configuration:"
echo "  Golden image name: $GOLDEN_IMAGE_NAME"
echo "  Temp container ID: $TEMP_CT_ID"
echo "  Base template: $BASE_TEMPLATE"
echo "  Output path: $GOLDEN_IMAGE_PATH"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}✗${NC} This script must be run as root"
    exit 1
fi

# Check if pct is available
if ! command -v pct >/dev/null 2>&1; then
    export PATH="/usr/sbin:$PATH"
    if ! command -v pct >/dev/null 2>&1; then
        echo -e "${RED}✗${NC} pct command not found. This must run on a Proxmox host."
        exit 1
    fi
fi

# Check if BTRFS filesystem
if ! btrfs filesystem show "$STORAGE_PATH" &>/dev/null; then
    echo -e "${RED}✗${NC} $STORAGE_PATH is not a BTRFS filesystem"
    echo "This script requires BTRFS for copy-on-write snapshots"
    exit 1
fi

echo -e "${GREEN}✓${NC} BTRFS filesystem detected"

# Create golden image directory if it doesn't exist
mkdir -p "$GOLDEN_IMAGE_DIR"
echo -e "${GREEN}✓${NC} Golden image directory ready: $GOLDEN_IMAGE_DIR"

# Check if golden image already exists
if [ -d "$GOLDEN_IMAGE_PATH" ]; then
    echo -e "${YELLOW}⚠${NC}  Golden image '$GOLDEN_IMAGE_NAME' already exists"
    read -p "Delete and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "Deleting existing golden image..."
        btrfs subvolume delete "$GOLDEN_IMAGE_PATH"
        echo -e "${GREEN}✓${NC} Deleted old golden image"
    else
        echo "Aborting"
        exit 0
    fi
fi

# Check if temp container exists
if pct status "$TEMP_CT_ID" &>/dev/null; then
    echo -e "${YELLOW}⚠${NC}  Temp container $TEMP_CT_ID already exists"
    read -p "Delete and recreate? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        pct stop "$TEMP_CT_ID" 2>/dev/null || true
        pct destroy "$TEMP_CT_ID"
        echo -e "${GREEN}✓${NC} Deleted temp container"
    else
        echo "Aborting"
        exit 0
    fi
fi

# Create temporary container
echo ""
echo "Creating temporary container $TEMP_CT_ID from template..."
pct create "$TEMP_CT_ID" "$BASE_TEMPLATE" \
    --hostname "golden-$GOLDEN_IMAGE_NAME" \
    --memory 512 \
    --swap 512 \
    --cores 2 \
    --rootfs "$STORAGE:8" \
    --net0 "name=eth0,bridge=vmbr0,firewall=1" \
    --unprivileged 1 \
    --features "nesting=1"

echo -e "${GREEN}✓${NC} Temporary container created"

# Start container
echo "Starting container..."
pct start "$TEMP_CT_ID"
sleep 5

echo -e "${GREEN}✓${NC} Container started"

# Wait for network (with timeout)
echo "Waiting for network (timeout 30s)..."
if ! pct exec "$TEMP_CT_ID" -- bash -c 'for i in {1..30}; do ping -c1 8.8.8.8 &>/dev/null && exit 0; sleep 1; done; exit 1'; then
    echo -e "${YELLOW}⚠${NC}  Network not available, skipping package installation"
    NETWORK_AVAILABLE=false
else
    echo -e "${GREEN}✓${NC} Network ready"
    NETWORK_AVAILABLE=true
fi

# Configure container based on image name
if [ "$NETWORK_AVAILABLE" = true ]; then
    echo ""
    echo "Configuring golden image..."

    case "$GOLDEN_IMAGE_NAME" in
        *minimal* | debian-minimal)
            echo "Installing minimal package set..."
            pct exec "$TEMP_CT_ID" -- apt-get update
            pct exec "$TEMP_CT_ID" -- apt-get install -y curl wget ca-certificates gnupg sudo
            echo -e "${GREEN}✓${NC} Minimal packages installed"
            ;;

        *privacy* | privacy-router)
            echo "Installing privacy router packages (WireGuard, networking tools)..."
            pct exec "$TEMP_CT_ID" -- apt-get update
            pct exec "$TEMP_CT_ID" -- apt-get install -y \
                curl wget ca-certificates gnupg sudo \
                wireguard-tools iptables iproute2 iputils-ping \
                dnsutils tcpdump net-tools
            echo -e "${GREEN}✓${NC} Privacy router packages installed"
            ;;

        *netmaker*)
            echo "Installing Netmaker-ready image (netclient pre-installed)..."
            pct exec "$TEMP_CT_ID" -- apt-get update
            pct exec "$TEMP_CT_ID" -- apt-get install -y \
                curl wget ca-certificates gnupg sudo \
                wireguard-tools iptables

            # Download netclient
            NETCLIENT_VERSION="v0.25.0"
            pct exec "$TEMP_CT_ID" -- bash -c "
                curl -fsSL https://github.com/gravitl/netclient/releases/download/${NETCLIENT_VERSION}/netclient \
                    -o /usr/local/bin/netclient
                chmod +x /usr/local/bin/netclient
            "

            if pct exec "$TEMP_CT_ID" -- which netclient &>/dev/null; then
                NETCLIENT_VER=$(pct exec "$TEMP_CT_ID" -- netclient --version 2>&1 | head -1)
                echo -e "${GREEN}✓${NC} Netclient installed: $NETCLIENT_VER"
            fi

            # Inject Netmaker token if available on host
            if [ -f "/etc/op-dbus/netmaker.env" ]; then
                source /etc/op-dbus/netmaker.env
                if [ -n "$NETMAKER_TOKEN" ]; then
                    echo "Injecting Netmaker token into golden image..."
                    pct exec "$TEMP_CT_ID" -- mkdir -p /etc/netmaker
                    pct exec "$TEMP_CT_ID" -- bash -c "echo '$NETMAKER_TOKEN' > /etc/netmaker/enrollment-token"
                    pct exec "$TEMP_CT_ID" -- chmod 600 /etc/netmaker/enrollment-token
                    echo -e "${GREEN}✓${NC} Netmaker token injected into golden image"
                else
                    echo -e "${YELLOW}⚠${NC}  NETMAKER_TOKEN not set in /etc/op-dbus/netmaker.env"
                fi
            else
                echo -e "${YELLOW}⚠${NC}  /etc/op-dbus/netmaker.env not found - token not injected"
                echo "  Containers will need token provided separately"
            fi
            ;;

        *)
            echo "Custom golden image: $GOLDEN_IMAGE_NAME"
            echo "Performing basic system update..."
            pct exec "$TEMP_CT_ID" -- apt-get update
            pct exec "$TEMP_CT_ID" -- apt-get install -y curl wget ca-certificates sudo
            echo -e "${GREEN}✓${NC} Basic packages installed"
            ;;
    esac

    # Clean up package cache
    echo "Cleaning up..."
    pct exec "$TEMP_CT_ID" -- apt-get clean
    pct exec "$TEMP_CT_ID" -- rm -rf /var/lib/apt/lists/*
    pct exec "$TEMP_CT_ID" -- rm -rf /tmp/*
    pct exec "$TEMP_CT_ID" -- rm -f /var/log/*.log
    echo -e "${GREEN}✓${NC} Cleanup complete"
fi

# Stop container
echo ""
echo "Stopping container..."
pct stop "$TEMP_CT_ID"
sleep 3

echo -e "${GREEN}✓${NC} Container stopped"

# Create BTRFS snapshot of the rootfs
TEMP_ROOTFS_PATH="$STORAGE_PATH/images/$TEMP_CT_ID/rootfs"

if [ ! -d "$TEMP_ROOTFS_PATH" ]; then
    echo -e "${RED}✗${NC} Container rootfs not found at: $TEMP_ROOTFS_PATH"
    echo "Available paths:"
    find "$STORAGE_PATH" -maxdepth 3 -type d -name "$TEMP_CT_ID" 2>/dev/null || true
    exit 1
fi

# Check if rootfs is a BTRFS subvolume
if ! btrfs subvolume show "$TEMP_ROOTFS_PATH" &>/dev/null; then
    echo -e "${RED}✗${NC} Container rootfs is not a BTRFS subvolume"
    echo "Path: $TEMP_ROOTFS_PATH"
    echo "This script requires Proxmox to use BTRFS storage"
    exit 1
fi

echo ""
echo "Creating BTRFS snapshot of container rootfs..."
echo "  Source: $TEMP_ROOTFS_PATH"
echo "  Destination: $GOLDEN_IMAGE_PATH"

# Create read-write snapshot for golden image
btrfs subvolume snapshot "$TEMP_ROOTFS_PATH" "$GOLDEN_IMAGE_PATH"

echo -e "${GREEN}✓${NC} BTRFS snapshot created"

# Get snapshot size
SNAPSHOT_SIZE=$(du -sh "$GOLDEN_IMAGE_PATH" | cut -f1)
echo "  Golden image size: $SNAPSHOT_SIZE"

# Optionally make read-only for safety
read -p "Make golden image read-only? (recommended) (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    btrfs property set -ts "$GOLDEN_IMAGE_PATH" ro true
    echo -e "${GREEN}✓${NC} Golden image set to read-only"
    READONLY_NOTE="(read-only)"
else
    READONLY_NOTE="(read-write)"
fi

# Clean up temporary container
echo ""
echo "Cleaning up temporary container..."
pct destroy "$TEMP_CT_ID"
echo -e "${GREEN}✓${NC} Temporary container removed"

# Show available golden images
echo ""
echo -e "${BLUE}=== Golden Image Creation Complete ===${NC}"
echo ""
echo "Golden image: $GOLDEN_IMAGE_NAME $READONLY_NOTE"
echo "Location: $GOLDEN_IMAGE_PATH"
echo "Size: $SNAPSHOT_SIZE"
echo ""
echo "Available golden images:"
ls -lh "$GOLDEN_IMAGE_DIR" 2>/dev/null || echo "  (none yet)"
echo ""
echo "Next steps:"
echo "1. Use this golden image in your op-dbus state file:"
echo ""
cat <<EOF
{
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": "100",
          "veth": "vi100",
          "bridge": "mesh",
          "properties": {
            "golden_image": "$GOLDEN_IMAGE_NAME",
            "hostname": "my-container",
            "memory": 1024,
            "cores": 2
          }
        }
      ]
    }
  }
}
EOF
echo ""
echo "2. Rebuild op-dbus with BTRFS snapshot support"
echo "3. Apply state - containers will be created instantly!"
echo ""
echo -e "${GREEN}✓${NC} Container creation will now take milliseconds instead of 30+ seconds"
echo ""
