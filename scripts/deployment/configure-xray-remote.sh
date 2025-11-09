#!/bin/bash
# Configure XRay Server and Client via MCP
# Installs and configures XRay in containers remotely

set -euo pipefail

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}=== GhostBridge XRay Configuration ===${NC}"
echo ""

# Configuration
PROXMOX_MESH_IP="${PROXMOX_MESH_IP:-}"
VPS_MESH_IP="${VPS_MESH_IP:-}"
MCP_PORT="${MCP_PORT:-9573}"
VPS_PUBLIC_IP="${VPS_PUBLIC_IP:-}"

# Get mesh IPs if not provided
if [ -z "$PROXMOX_MESH_IP" ]; then
    echo -e "${YELLOW}Enter Proxmox mesh IP:${NC}"
    read PROXMOX_MESH_IP
fi

if [ -z "$VPS_MESH_IP" ]; then
    echo -e "${YELLOW}Enter VPS mesh IP:${NC}"
    read VPS_MESH_IP
fi

if [ -z "$VPS_PUBLIC_IP" ]; then
    echo -e "${YELLOW}Enter VPS public IP:${NC}"
    read VPS_PUBLIC_IP
fi

echo ""

# Function to call MCP tool
mcp_call() {
    local IP=$1
    local PORT=$2
    local TOOL=$3
    local ARGS=$4

    curl -s -X POST "http://$IP:$PORT/jsonrpc" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":$(date +%s),\"method\":\"tools/call\",\"params\":{\"name\":\"$TOOL\",\"arguments\":$ARGS}}"
}

# Part 1: Configure VPS XRay Server
echo -e "${BLUE}=== [1/2] Configuring VPS XRay Server ===${NC}"
echo ""

echo -e "${YELLOW}Step 1: Install XRay in container 100${NC}"
INSTALL_CMD='bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install'
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"100\",\"command\":\"$INSTALL_CMD\"}")
echo "XRay installation started..."
echo ""

echo -e "${YELLOW}Step 2: Generate XRay UUID${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"100\",\"command\":\"xray uuid\"}")
XRAY_UUID=$(echo "$RESPONSE" | jq -r '.result.stdout // empty' | tr -d '\n')

if [ -z "$XRAY_UUID" ]; then
    echo -e "${RED}Failed to generate UUID, creating one locally${NC}"
    XRAY_UUID=$(uuidgen)
fi

echo "XRay UUID: $XRAY_UUID"
echo "$XRAY_UUID" > /tmp/xray-uuid.txt
echo -e "${GREEN}✓ UUID saved to /tmp/xray-uuid.txt${NC}"
echo ""

echo -e "${YELLOW}Step 3: Create XRay server config${NC}"
XRAY_SERVER_CONFIG=$(cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": 443,
      "protocol": "vmess",
      "settings": {
        "clients": [
          {
            "id": "$XRAY_UUID",
            "alterId": 0
          }
        ]
      },
      "streamSettings": {
        "network": "tcp",
        "security": "none"
      }
    }
  ],
  "outbounds": [
    {
      "protocol": "freedom",
      "settings": {}
    }
  ]
}
EOF
)

# Write config via MCP
ESCAPED_CONFIG=$(echo "$XRAY_SERVER_CONFIG" | jq -c . | jq -R .)
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "file.write" "{\"path\": \"/var/lib/lxc/xray-server/rootfs/usr/local/etc/xray/config.json\", \"content\": $ESCAPED_CONFIG}")
echo -e "${GREEN}✓ XRay server config written${NC}"
echo ""

echo -e "${YELLOW}Step 4: Start XRay service${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"100\",\"command\":\"systemctl enable --now xray\"}")
echo -e "${GREEN}✓ XRay server started${NC}"
echo ""

echo -e "${YELLOW}Step 5: Verify XRay listening on port 443${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"100\",\"command\":\"netstat -tlnp | grep 443\"}")
echo "$RESPONSE" | jq -r '.result.stdout // "Not listening"'
echo ""

echo -e "${GREEN}✓ VPS XRay server configured${NC}"
echo ""
sleep 2

# Part 2: Configure Proxmox XRay Client
echo -e "${BLUE}=== [2/2] Configuring Proxmox XRay Client ===${NC}"
echo ""

echo -e "${YELLOW}Step 1: Install XRay in container 102${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"102\",\"command\":\"$INSTALL_CMD\"}")
echo "XRay installation started..."
echo ""

echo -e "${YELLOW}Step 2: Create XRay client config${NC}"
XRAY_CLIENT_CONFIG=$(cat <<EOF
{
  "log": {
    "loglevel": "warning"
  },
  "inbounds": [
    {
      "port": 1080,
      "protocol": "socks",
      "settings": {
        "auth": "noauth",
        "udp": true
      }
    }
  ],
  "outbounds": [
    {
      "protocol": "vmess",
      "settings": {
        "vnext": [
          {
            "address": "$VPS_PUBLIC_IP",
            "port": 443,
            "users": [
              {
                "id": "$XRAY_UUID",
                "alterId": 0
              }
            ]
          }
        ]
      },
      "streamSettings": {
        "network": "tcp",
        "security": "none"
      }
    }
  ]
}
EOF
)

ESCAPED_CONFIG=$(echo "$XRAY_CLIENT_CONFIG" | jq -c . | jq -R .)
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "file.write" "{\"path\": \"/var/lib/lxc/xray-client/rootfs/usr/local/etc/xray/config.json\", \"content\": $ESCAPED_CONFIG}")
echo -e "${GREEN}✓ XRay client config written${NC}"
echo ""

echo -e "${YELLOW}Step 3: Start XRay service${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"102\",\"command\":\"systemctl enable --now xray\"}")
echo -e "${GREEN}✓ XRay client started${NC}"
echo ""

echo -e "${YELLOW}Step 4: Verify XRay client running${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "container.exec" "{\"container_id\":\"102\",\"command\":\"systemctl status xray\"}")
echo "$RESPONSE" | jq -r '.result.stdout // "Status unknown"'
echo ""

echo -e "${GREEN}✓ Proxmox XRay client configured${NC}"
echo ""

# Summary
echo -e "${CYAN}=== Configuration Complete ===${NC}"
echo ""
echo -e "${GREEN}✓ VPS XRay Server:${NC}"
echo "  - Container: 100 (xray-server)"
echo "  - Port: 443"
echo "  - UUID: $XRAY_UUID"
echo "  - Public IP: $VPS_PUBLIC_IP"
echo ""
echo -e "${GREEN}✓ Proxmox XRay Client:${NC}"
echo "  - Container: 102 (xray-client)"
echo "  - SOCKS proxy: localhost:1080"
echo "  - Connected to: $VPS_PUBLIC_IP:443"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "  1. Configure WireGuard in container 100"
echo "  2. Configure Warp tunnel in container 101"
echo "  3. Set up traffic routing through the chain"
echo "  4. Test full privacy chain"
echo ""
echo "XRay UUID saved to: /tmp/xray-uuid.txt"
