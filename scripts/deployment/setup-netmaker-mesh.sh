#!/bin/bash
# Automated Netmaker Mesh Setup for GhostBridge
# Enrolls both Proxmox and VPS servers into Netmaker mesh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== GhostBridge Netmaker Mesh Setup ===${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}ERROR: Please run as root${NC}"
    exit 1
fi

# Step 1: Install netclient if not already installed
echo -e "${YELLOW}[1/5] Checking netclient installation...${NC}"
if ! command -v netclient &> /dev/null; then
    echo "Installing netclient..."
    curl -sfL https://raw.githubusercontent.com/gravitl/netmaker/master/scripts/netclient-install.sh | sh
    echo -e "${GREEN}✓ netclient installed${NC}"
else
    echo -e "${GREEN}✓ netclient already installed${NC}"
    netclient --version
fi

echo ""

# Step 2: Check if already enrolled
echo -e "${YELLOW}[2/5] Checking current mesh status...${NC}"
if netclient list &> /dev/null; then
    echo -e "${GREEN}Already enrolled in mesh:${NC}"
    netclient list
    echo ""
    read -p "Do you want to re-enroll? (y/N): " REENROLL
    if [[ "$REENROLL" != "y" && "$REENROLL" != "Y" ]]; then
        echo "Skipping enrollment"
        exit 0
    fi
    echo "Leaving current mesh..."
    netclient leave
fi

echo ""

# Step 3: Get enrollment token
echo -e "${YELLOW}[3/5] Netmaker Enrollment${NC}"
echo "You need an enrollment token from your Netmaker server."
echo ""
echo "Options:"
echo "  1. SaaS: Get token from https://app.netmaker.io"
echo "  2. Self-hosted: Get token from Netmaker admin dashboard"
echo ""
read -p "Enter enrollment token: " ENROLLMENT_TOKEN

if [ -z "$ENROLLMENT_TOKEN" ]; then
    echo -e "${RED}ERROR: Enrollment token required${NC}"
    exit 1
fi

echo ""

# Step 4: Join mesh
echo -e "${YELLOW}[4/5] Joining Netmaker mesh...${NC}"
if netclient join -t "$ENROLLMENT_TOKEN"; then
    echo -e "${GREEN}✓ Successfully joined mesh${NC}"
else
    echo -e "${RED}✗ Failed to join mesh${NC}"
    exit 1
fi

echo ""

# Step 5: Verify connectivity
echo -e "${YELLOW}[5/5] Verifying mesh connectivity...${NC}"
sleep 3  # Wait for mesh to initialize

# Get mesh network info
MESH_INFO=$(netclient list)
echo "$MESH_INFO"

# Extract mesh IP
MESH_IP=$(echo "$MESH_INFO" | grep -oP 'Address: \K[0-9.]+' | head -1)

if [ -z "$MESH_IP" ]; then
    echo -e "${RED}✗ Could not determine mesh IP${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}=== Mesh Setup Complete ===${NC}"
echo ""
echo "Mesh IP: $MESH_IP"
echo ""
echo "Next steps:"
echo "  1. Run this script on the OTHER server (Proxmox or VPS)"
echo "  2. Note the mesh IP from both servers"
echo "  3. Test connectivity: ping <other-server-mesh-ip>"
echo "  4. Start MCP servers on both machines:"
echo "     op-dbus serve --bind 0.0.0.0 --port 9573"
echo "  5. Test MCP connectivity: curl http://<mesh-ip>:9573/health"
echo ""
echo "Save this mesh IP for deployment:"
echo "$MESH_IP" > /tmp/netmaker-mesh-ip.txt
echo -e "${GREEN}✓ Mesh IP saved to /tmp/netmaker-mesh-ip.txt${NC}"
