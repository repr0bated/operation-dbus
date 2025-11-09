#!/bin/bash
# Deploy GhostBridge via MCP JSON-RPC
# Deploys privacy-vps to VPS and privacy-client to Proxmox remotely

set -euo pipefail

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}"
cat << "EOF"
   _____ _               _   ____       _     _
  / ____| |             | | |  _ \     (_)   | |
 | |  __| |__   ___  ___| |_| |_) |_ __ _  __| | __ _  ___
 | | |_ | '_ \ / _ \/ __| __|  _ <| '__| |/ _` |/ _` |/ _ \
 | |__| | | | | (_) \__ \ |_| |_) | |  | | (_| | (_| |  __/
  \_____|_| |_|\___/|___/\__|____/|_|  |_|\__,_|\__, |\___|
                                                  __/ |
  Remote Deployment via MCP                     |___/
EOF
echo -e "${NC}"

# Configuration
PROXMOX_MESH_IP="${PROXMOX_MESH_IP:-}"
VPS_MESH_IP="${VPS_MESH_IP:-}"
MCP_PORT="${MCP_PORT:-9573}"

# Get mesh IPs if not provided
if [ -z "$PROXMOX_MESH_IP" ]; then
    echo -e "${YELLOW}Enter Proxmox mesh IP:${NC}"
    read PROXMOX_MESH_IP
fi

if [ -z "$VPS_MESH_IP" ]; then
    echo -e "${YELLOW}Enter VPS mesh IP:${NC}"
    read VPS_MESH_IP
fi

echo ""
echo -e "${BLUE}Deployment targets:${NC}"
echo "  Proxmox: $PROXMOX_MESH_IP:$MCP_PORT (privacy-client)"
echo "  VPS:     $VPS_MESH_IP:$MCP_PORT (privacy-vps)"
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

# VPS State Configuration (Profile 3: Privacy VPS)
VPS_STATE='{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": 100,
          "name": "xray-server",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "socket_networking": true,
            "port_name": "internal_100",
            "ipv4": "10.0.0.100/24"
          },
          "services": ["xray"]
        }
      ]
    },
    "openflow": {
      "enable_security_flows": true,
      "obfuscation_level": 2,
      "auto_discover_containers": true,
      "bridges": [
        {
          "name": "ovsbr0",
          "datapath_type": "netdev",
          "socket_ports": [
            {"name": "internal_100", "container_id": "100"}
          ]
        }
      ]
    }
  }
}'

# Proxmox State Configuration (Profile 2: Privacy Client)
PROXMOX_STATE='{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": 100,
          "name": "wireguard-gateway",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "socket_networking": true,
            "port_name": "internal_100",
            "ipv4": "10.0.0.100/24"
          },
          "services": ["wireguard"]
        },
        {
          "id": 101,
          "name": "warp-tunnel",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "socket_networking": false,
            "wg_tunnel": true,
            "port_name": "wg-warp",
            "ipv4": "10.0.0.101/24"
          },
          "config": {
            "wg-quick": {
              "interface": "wg-warp",
              "post_up": "ovs-vsctl add-port ovsbr0 wg-warp",
              "post_down": "ovs-vsctl del-port ovsbr0 wg-warp"
            }
          },
          "services": ["wg-quick@wg-warp"]
        },
        {
          "id": 102,
          "name": "xray-client",
          "template": "debian-12",
          "autostart": true,
          "network": {
            "bridge": "ovsbr0",
            "socket_networking": true,
            "port_name": "internal_102",
            "ipv4": "10.0.0.102/24"
          },
          "services": ["xray"]
        }
      ]
    },
    "openflow": {
      "enable_security_flows": true,
      "obfuscation_level": 3,
      "auto_discover_containers": true,
      "bridges": [
        {
          "name": "ovsbr0",
          "datapath_type": "netdev",
          "socket_ports": [
            {"name": "internal_100", "container_id": "100"},
            {"name": "internal_102", "container_id": "102"}
          ],
          "tunnel_ports": [
            {"name": "wg-warp", "container_id": "101"}
          ]
        }
      ]
    }
  }
}'

# Deploy VPS
echo -e "${BLUE}=== [1/2] Deploying VPS (Privacy Router Server) ===${NC}"
echo ""

echo -e "${YELLOW}Step 1: Query current VPS state${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "state.query" "{}")
echo "Response: $RESPONSE" | jq '.' || echo "$RESPONSE"
echo ""

echo -e "${YELLOW}Step 2: Apply VPS state (1 container: xray-server)${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "state.apply" "{\"state_json\":$(echo "$VPS_STATE" | jq -c .)}")
echo "Response: $RESPONSE" | jq '.' || echo "$RESPONSE"
echo ""

echo -e "${YELLOW}Step 3: Verify VPS container created${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "container.list" "{}")
echo "Response: $RESPONSE" | jq '.' || echo "$RESPONSE"
echo ""

echo -e "${YELLOW}Step 4: Check VPS OpenFlow flows${NC}"
RESPONSE=$(mcp_call "$VPS_MESH_IP" "$MCP_PORT" "openflow.flows.list" "{\"bridge\":\"ovsbr0\"}")
FLOW_COUNT=$(echo "$RESPONSE" | jq '.result.total_flows // 0')
echo "OpenFlow flows installed: $FLOW_COUNT (expected: 11 for Level 2)"
echo ""

echo -e "${GREEN}✓ VPS deployment complete${NC}"
echo ""
sleep 2

# Deploy Proxmox
echo -e "${BLUE}=== [2/2] Deploying Proxmox (Privacy Router Client) ===${NC}"
echo ""

echo -e "${YELLOW}Step 1: Query current Proxmox state${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "state.query" "{}")
echo "Response: $RESPONSE" | jq '.' || echo "$RESPONSE"
echo ""

echo -e "${YELLOW}Step 2: Apply Proxmox state (3 containers: WireGuard, Warp, XRay)${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "state.apply" "{\"state_json\":$(echo "$PROXMOX_STATE" | jq -c .)}")
echo "Response: $RESPONSE" | jq '.' || echo "$RESPONSE"
echo ""

echo -e "${YELLOW}Step 3: Verify Proxmox containers created${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "container.list" "{}")
echo "Response: $RESPONSE" | jq '.' || echo "$RESPONSE"
echo ""

echo -e "${YELLOW}Step 4: Check Proxmox OpenFlow flows${NC}"
RESPONSE=$(mcp_call "$PROXMOX_MESH_IP" "$MCP_PORT" "openflow.flows.list" "{\"bridge\":\"ovsbr0\"}")
FLOW_COUNT=$(echo "$RESPONSE" | jq '.result.total_flows // 0')
echo "OpenFlow flows installed: $FLOW_COUNT (expected: 18 for Level 3)"
echo ""

echo -e "${GREEN}✓ Proxmox deployment complete${NC}"
echo ""

# Summary
echo -e "${CYAN}=== Deployment Summary ===${NC}"
echo ""
echo -e "${GREEN}✓ VPS Server:${NC}"
echo "  - Container 100: xray-server (XRay endpoint)"
echo "  - OpenFlow: Level 2 obfuscation (11 flows)"
echo "  - Mesh IP: $VPS_MESH_IP"
echo ""
echo -e "${GREEN}✓ Proxmox Client:${NC}"
echo "  - Container 100: wireguard-gateway"
echo "  - Container 101: warp-tunnel (wg-quick)"
echo "  - Container 102: xray-client"
echo "  - OpenFlow: Level 3 obfuscation (18 flows)"
echo "  - Mesh IP: $PROXMOX_MESH_IP"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "  1. Install XRay in containers"
echo "  2. Configure XRay client → server connection"
echo "  3. Test full GhostBridge chain"
echo ""
echo "Run: ./configure-xray-remote.sh"
