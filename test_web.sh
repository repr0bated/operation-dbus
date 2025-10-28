#!/bin/bash
# Test script for Web Interface

set -e

echo "=== D-Bus MCP Web Interface Test ==="
echo ""

GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Start orchestrator
echo -e "${BLUE}Starting orchestrator...${NC}"
./target/debug/dbus-orchestrator &
ORCH_PID=$!
sleep 2

# Start web server
echo -e "${BLUE}Starting web server...${NC}"
./target/debug/dbus-mcp-web &
WEB_PID=$!
sleep 2

echo ""
echo -e "${GREEN}✓ Services started${NC}"
echo ""
echo "Orchestrator PID: $ORCH_PID"
echo "Web Server PID: $WEB_PID"
echo ""
echo -e "${BLUE}Web interface available at: http://127.0.0.1:8080${NC}"
echo ""
echo "Press Ctrl+C to stop..."

# Trap Ctrl+C to clean up
trap "kill $ORCH_PID $WEB_PID 2>/dev/null; exit" INT

# Wait for Ctrl+C
wait
