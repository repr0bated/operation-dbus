#!/bin/bash
# GhostBridge Master Deployment Script
# Orchestrates complete deployment via MCP over Netmaker mesh

set -euo pipefail

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Banner
clear
echo -e "${CYAN}"
cat << "EOF"
   _____ _               _   ____       _     _
  / ____| |             | | |  _ \     (_)   | |
 | |  __| |__   ___  ___| |_| |_) |_ __ _  __| | __ _  ___
 | | |_ | '_ \ / _ \/ __| __|  _ <| '__| |/ _` |/ _` |/ _ \
 | |__| | | | | (_) \__ \ |_| |_) | |  | | (_| | (_| |  __/
  \_____|_| |_|\___/|___/\__|____/|_|  |_|\__,_|\__, |\___|
                                                  __/ |
  Privacy Router - Full Remote Deployment       |___/
EOF
echo -e "${NC}"

echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  GhostBridge Automated Deployment${NC}"
echo -e "${BLUE}  Branch: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
echo ""

# Menu
echo -e "${YELLOW}Select deployment stage:${NC}"
echo ""
echo "  ${GREEN}1${NC}. Setup Netmaker Mesh (run on EACH server)"
echo "  ${GREEN}2${NC}. Test MCP Connectivity (after mesh setup)"
echo "  ${GREEN}3${NC}. Deploy GhostBridge Containers (remote via MCP)"
echo "  ${GREEN}4${NC}. Configure XRay Server/Client (remote via MCP)"
echo "  ${GREEN}5${NC}. Full Automated Deployment (all steps)"
echo "  ${GREEN}6${NC}. View Deployment Status"
echo "  ${GREEN}7${NC}. Manual MCP Commands (advanced)"
echo ""
echo "  ${RED}0${NC}. Exit"
echo ""
read -p "$(echo -e ${CYAN}Choose option [0-7]: ${NC})" OPTION

case $OPTION in
    1)
        echo ""
        echo -e "${BLUE}═══ Stage 1: Netmaker Mesh Setup ═══${NC}"
        echo ""
        echo "This will:"
        echo "  - Install netclient if needed"
        echo "  - Join this server to Netmaker mesh"
        echo "  - Configure mesh networking"
        echo ""
        echo -e "${YELLOW}NOTE: Run this on BOTH Proxmox and VPS servers${NC}"
        echo ""
        read -p "Continue? [y/N]: " CONFIRM
        if [[ "$CONFIRM" == "y" || "$CONFIRM" == "Y" ]]; then
            if [ -f "./setup-netmaker-mesh.sh" ]; then
                sudo ./setup-netmaker-mesh.sh
            else
                echo -e "${RED}Error: setup-netmaker-mesh.sh not found${NC}"
                exit 1
            fi
        fi
        ;;

    2)
        echo ""
        echo -e "${BLUE}═══ Stage 2: MCP Connectivity Test ═══${NC}"
        echo ""
        echo "This will:"
        echo "  - Test MCP server on Proxmox"
        echo "  - Test MCP server on VPS"
        echo "  - Verify JSON-RPC endpoints"
        echo ""
        read -p "Proxmox mesh IP: " PROXMOX_MESH_IP
        read -p "VPS mesh IP: " VPS_MESH_IP

        export PROXMOX_MESH_IP
        export VPS_MESH_IP

        if [ -f "./test-mcp-connectivity.sh" ]; then
            ./test-mcp-connectivity.sh
        else
            echo -e "${RED}Error: test-mcp-connectivity.sh not found${NC}"
            exit 1
        fi
        ;;

    3)
        echo ""
        echo -e "${BLUE}═══ Stage 3: Deploy Containers ═══${NC}"
        echo ""
        echo "This will deploy:"
        echo "  ${CYAN}VPS${NC}: 1 container (xray-server)"
        echo "  ${CYAN}Proxmox${NC}: 3 containers (wireguard, warp, xray-client)"
        echo ""
        read -p "Proxmox mesh IP: " PROXMOX_MESH_IP
        read -p "VPS mesh IP: " VPS_MESH_IP

        export PROXMOX_MESH_IP
        export VPS_MESH_IP

        if [ -f "./deploy-ghostbridge-remote.sh" ]; then
            ./deploy-ghostbridge-remote.sh
        else
            echo -e "${RED}Error: deploy-ghostbridge-remote.sh not found${NC}"
            exit 1
        fi
        ;;

    4)
        echo ""
        echo -e "${BLUE}═══ Stage 4: Configure XRay ═══${NC}"
        echo ""
        echo "This will:"
        echo "  - Install XRay in containers"
        echo "  - Generate UUID"
        echo "  - Configure server/client connection"
        echo ""
        read -p "Proxmox mesh IP: " PROXMOX_MESH_IP
        read -p "VPS mesh IP: " VPS_MESH_IP
        read -p "VPS public IP: " VPS_PUBLIC_IP

        export PROXMOX_MESH_IP
        export VPS_MESH_IP
        export VPS_PUBLIC_IP

        if [ -f "./configure-xray-remote.sh" ]; then
            ./configure-xray-remote.sh
        else
            echo -e "${RED}Error: configure-xray-remote.sh not found${NC}"
            exit 1
        fi
        ;;

    5)
        echo ""
        echo -e "${BLUE}═══ Full Automated Deployment ═══${NC}"
        echo ""
        echo -e "${YELLOW}Prerequisites:${NC}"
        echo "  1. Both servers already joined to Netmaker mesh"
        echo "  2. MCP servers running on both servers"
        echo "  3. You know the mesh IPs and VPS public IP"
        echo ""
        read -p "All prerequisites met? [y/N]: " CONFIRM
        if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
            echo "Please complete prerequisites first."
            exit 0
        fi

        echo ""
        read -p "Proxmox mesh IP: " PROXMOX_MESH_IP
        read -p "VPS mesh IP: " VPS_MESH_IP
        read -p "VPS public IP: " VPS_PUBLIC_IP

        export PROXMOX_MESH_IP
        export VPS_MESH_IP
        export VPS_PUBLIC_IP

        echo ""
        echo -e "${CYAN}Starting full deployment...${NC}"
        echo ""

        # Step 1: Test connectivity
        echo -e "${BLUE}[1/3] Testing MCP connectivity...${NC}"
        if [ -f "./test-mcp-connectivity.sh" ]; then
            ./test-mcp-connectivity.sh || {
                echo -e "${RED}MCP connectivity test failed!${NC}"
                exit 1
            }
        else
            echo -e "${RED}Error: test-mcp-connectivity.sh not found${NC}"
            exit 1
        fi

        sleep 2

        # Step 2: Deploy containers
        echo ""
        echo -e "${BLUE}[2/3] Deploying containers...${NC}"
        if [ -f "./deploy-ghostbridge-remote.sh" ]; then
            ./deploy-ghostbridge-remote.sh || {
                echo -e "${RED}Container deployment failed!${NC}"
                exit 1
            }
        else
            echo -e "${RED}Error: deploy-ghostbridge-remote.sh not found${NC}"
            exit 1
        fi

        sleep 2

        # Step 3: Configure XRay
        echo ""
        echo -e "${BLUE}[3/3] Configuring XRay...${NC}"
        if [ -f "./configure-xray-remote.sh" ]; then
            ./configure-xray-remote.sh || {
                echo -e "${RED}XRay configuration failed!${NC}"
                exit 1
            }
        else
            echo -e "${RED}Error: configure-xray-remote.sh not found${NC}"
            exit 1
        fi

        echo ""
        echo -e "${GREEN}═══════════════════════════════════════════════════${NC}"
        echo -e "${GREEN}  ✓ Full GhostBridge Deployment Complete!${NC}"
        echo -e "${GREEN}═══════════════════════════════════════════════════${NC}"
        echo ""
        echo "Next steps:"
        echo "  1. Configure WireGuard in container 100"
        echo "  2. Configure Warp tunnel in container 101"
        echo "  3. Test full privacy chain"
        ;;

    6)
        echo ""
        if [ -f "./DEPLOYMENT-STATUS.md" ]; then
            cat ./DEPLOYMENT-STATUS.md | less
        else
            echo -e "${RED}Error: DEPLOYMENT-STATUS.md not found${NC}"
        fi
        ;;

    7)
        echo ""
        echo -e "${BLUE}═══ Manual MCP Commands ═══${NC}"
        echo ""
        read -p "MCP server IP: " MCP_IP
        read -p "MCP server port [9573]: " MCP_PORT
        MCP_PORT=${MCP_PORT:-9573}

        echo ""
        echo -e "${YELLOW}Available commands:${NC}"
        echo "  1. Query state"
        echo "  2. List containers"
        echo "  3. List OpenFlow flows"
        echo "  4. Execute command in container"
        echo "  5. Custom JSON-RPC call"
        echo ""
        read -p "Choose [1-5]: " CMD_OPTION

        case $CMD_OPTION in
            1)
                echo ""
                echo "Querying state..."
                curl -s -X POST "http://$MCP_IP:$MCP_PORT/jsonrpc" \
                    -H "Content-Type: application/json" \
                    -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"state.query","arguments":{}}}' | jq '.'
                ;;
            2)
                echo ""
                echo "Listing containers..."
                curl -s -X POST "http://$MCP_IP:$MCP_PORT/jsonrpc" \
                    -H "Content-Type: application/json" \
                    -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"container.list","arguments":{}}}' | jq '.'
                ;;
            3)
                echo ""
                read -p "Bridge name [ovsbr0]: " BRIDGE
                BRIDGE=${BRIDGE:-ovsbr0}
                echo "Listing OpenFlow flows on $BRIDGE..."
                curl -s -X POST "http://$MCP_IP:$MCP_PORT/jsonrpc" \
                    -H "Content-Type: application/json" \
                    -d "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"openflow.flows.list\",\"arguments\":{\"bridge\":\"$BRIDGE\"}}}" | jq '.'
                ;;
            4)
                echo ""
                read -p "Container ID: " CONTAINER_ID
                read -p "Command: " COMMAND
                echo "Executing command in container $CONTAINER_ID..."
                curl -s -X POST "http://$MCP_IP:$MCP_PORT/jsonrpc" \
                    -H "Content-Type: application/json" \
                    -d "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"container.exec\",\"arguments\":{\"container_id\":\"$CONTAINER_ID\",\"command\":\"$COMMAND\"}}}" | jq '.'
                ;;
            5)
                echo ""
                read -p "Tool name: " TOOL_NAME
                read -p "Arguments (JSON): " ARGS
                echo "Calling $TOOL_NAME..."
                curl -s -X POST "http://$MCP_IP:$MCP_PORT/jsonrpc" \
                    -H "Content-Type: application/json" \
                    -d "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"$TOOL_NAME\",\"arguments\":$ARGS}}" | jq '.'
                ;;
        esac
        ;;

    0)
        echo ""
        echo "Exiting."
        exit 0
        ;;

    *)
        echo ""
        echo -e "${RED}Invalid option${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}Done!${NC}"
