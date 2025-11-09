#!/bin/bash
# Test MCP Server Connectivity over Netmaker Mesh
# Tests both Proxmox and VPS MCP servers are accessible

set -euo pipefail

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== GhostBridge MCP Connectivity Test ===${NC}"
echo ""

# Configuration
PROXMOX_MESH_IP="${PROXMOX_MESH_IP:-}"
VPS_MESH_IP="${VPS_MESH_IP:-}"
MCP_PORT="${MCP_PORT:-9573}"

# Check if mesh IPs are provided
if [ -z "$PROXMOX_MESH_IP" ]; then
    echo -e "${YELLOW}Enter Proxmox mesh IP:${NC}"
    read PROXMOX_MESH_IP
fi

if [ -z "$VPS_MESH_IP" ]; then
    echo -e "${YELLOW}Enter VPS mesh IP:${NC}"
    read VPS_MESH_IP
fi

echo ""
echo "Testing connectivity to:"
echo "  Proxmox: $PROXMOX_MESH_IP:$MCP_PORT"
echo "  VPS:     $VPS_MESH_IP:$MCP_PORT"
echo ""

# Function to test MCP server
test_mcp_server() {
    local NAME=$1
    local IP=$2
    local PORT=$3

    echo -e "${YELLOW}Testing $NAME MCP server...${NC}"

    # Test 1: Health check
    echo -n "  Health check: "
    if curl -s -m 5 "http://$IP:$PORT/health" &>/dev/null; then
        echo -e "${GREEN}✓ OK${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        return 1
    fi

    # Test 2: MCP tools/list
    echo -n "  MCP tools/list: "
    RESPONSE=$(curl -s -m 10 -X POST "http://$IP:$PORT/jsonrpc" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}')

    if echo "$RESPONSE" | jq -e '.result.tools' &>/dev/null; then
        TOOL_COUNT=$(echo "$RESPONSE" | jq '.result.tools | length')
        echo -e "${GREEN}✓ OK ($TOOL_COUNT tools)${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo "Response: $RESPONSE"
        return 1
    fi

    # Test 3: State query
    echo -n "  State query: "
    RESPONSE=$(curl -s -m 10 -X POST "http://$IP:$PORT/jsonrpc" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"state.query","arguments":{}}}')

    if echo "$RESPONSE" | jq -e '.result' &>/dev/null; then
        echo -e "${GREEN}✓ OK${NC}"
    else
        echo -e "${RED}✗ FAILED${NC}"
        echo "Response: $RESPONSE"
        return 1
    fi

    # Test 4: OVSDB introspection
    echo -n "  OVSDB introspect: "
    RESPONSE=$(curl -s -m 10 -X POST "http://$IP:$PORT/jsonrpc" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"introspect.ovsdb","arguments":{}}}')

    if echo "$RESPONSE" | jq -e '.result' &>/dev/null; then
        echo -e "${GREEN}✓ OK${NC}"
    else
        echo -e "${YELLOW}⚠ Not available (may not be installed yet)${NC}"
    fi

    echo ""
}

# Test Proxmox MCP
echo -e "${BLUE}[1/2] Testing Proxmox MCP Server${NC}"
if test_mcp_server "Proxmox" "$PROXMOX_MESH_IP" "$MCP_PORT"; then
    PROXMOX_OK=1
else
    PROXMOX_OK=0
fi

# Test VPS MCP
echo -e "${BLUE}[2/2] Testing VPS MCP Server${NC}"
if test_mcp_server "VPS" "$VPS_MESH_IP" "$MCP_PORT"; then
    VPS_OK=1
else
    VPS_OK=0
fi

# Summary
echo -e "${BLUE}=== Test Summary ===${NC}"
echo ""

if [ $PROXMOX_OK -eq 1 ] && [ $VPS_OK -eq 1 ]; then
    echo -e "${GREEN}✓ Both MCP servers accessible!${NC}"
    echo ""
    echo "Ready for remote deployment:"
    echo "  1. Deploy VPS (privacy-vps profile)"
    echo "  2. Deploy Proxmox/Workstation (privacy-client profile)"
    echo "  3. Test full GhostBridge chain"
    echo ""
    echo "Next: Run ./deploy-ghostbridge-remote.sh"
    exit 0
elif [ $PROXMOX_OK -eq 1 ] || [ $VPS_OK -eq 1 ]; then
    echo -e "${YELLOW}⚠ Partial connectivity${NC}"
    [ $PROXMOX_OK -eq 1 ] && echo "  Proxmox: OK"
    [ $PROXMOX_OK -eq 0 ] && echo "  Proxmox: FAILED"
    [ $VPS_OK -eq 1 ] && echo "  VPS: OK"
    [ $VPS_OK -eq 0 ] && echo "  VPS: FAILED"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Check MCP server is running: systemctl status op-dbus"
    echo "  2. Check firewall: ufw allow $MCP_PORT/tcp"
    echo "  3. Check Netmaker mesh: netclient list"
    echo "  4. Test ping: ping <mesh-ip>"
    exit 1
else
    echo -e "${RED}✗ No MCP servers accessible${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  1. Start MCP servers:"
    echo "     op-dbus serve --bind 0.0.0.0 --port $MCP_PORT"
    echo "  2. Check Netmaker mesh is active:"
    echo "     netclient list"
    echo "  3. Test network connectivity:"
    echo "     ping $PROXMOX_MESH_IP"
    echo "     ping $VPS_MESH_IP"
    exit 1
fi
