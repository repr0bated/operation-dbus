#!/bin/bash
# Test script for D-Bus MCP Server system
# Fish shell compatible version

set -e

echo "=== D-Bus MCP Server Test ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Build the project
echo -e "${BLUE}[1/5] Building project...${NC}"
cargo build 2>&1 | tail -3
echo -e "${GREEN}✓ Build complete${NC}"
echo ""

# Start orchestrator in background
echo -e "${BLUE}[2/5] Starting orchestrator...${NC}"
./target/debug/dbus-orchestrator &
ORCH_PID=$!
echo "Orchestrator PID: $ORCH_PID"
sleep 2

# Check if orchestrator is running
if ps -p $ORCH_PID > /dev/null; then
    echo -e "${GREEN}✓ Orchestrator running${NC}"
else
    echo -e "${RED}✗ Orchestrator failed to start${NC}"
    exit 1
fi
echo ""

# Test MCP server with initialize
echo -e "${BLUE}[3/5] Testing MCP initialize...${NC}"
# Create JSON request for fish compatibility
INIT_REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'
INIT_RESPONSE=$(echo "$INIT_REQUEST" | ./target/debug/dbus-mcp)
if echo "$INIT_RESPONSE" | grep -q "protocolVersion"; then
    echo -e "${GREEN}✓ MCP initialize successful${NC}"
    echo "Response: $INIT_RESPONSE" | head -c 100
    echo "..."
else
    echo -e "${RED}✗ MCP initialize failed${NC}"
    echo "Response: $INIT_RESPONSE"
fi
echo ""

# Test tool execution - direct command
echo -e "${BLUE}[4/5] Testing direct command execution...${NC}"
# Create JSON request for fish compatibility
CMD_REQUEST='{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"execute_command","arguments":{"command":"echo Hello from MCP!"}}}'
CMD_RESPONSE=$(echo "$CMD_REQUEST" | ./target/debug/dbus-mcp)
if echo "$CMD_RESPONSE" | grep -q "Hello from MCP"; then
    echo -e "${GREEN}✓ Direct command execution successful${NC}"
    echo "$CMD_RESPONSE" | grep -o '"text":"[^"]*"' | head -1
else
    echo -e "${RED}✗ Command execution failed${NC}"
    echo "Response: $CMD_RESPONSE"
fi
echo ""

# Test background task execution via agent
echo -e "${BLUE}[5/5] Testing background task execution via agent...${NC}"
# Create JSON request for fish compatibility
BG_REQUEST='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"execute_command","arguments":{"command":"echo Background task!","background":true}}}'
BG_RESPONSE=$(echo "$BG_REQUEST" | ./target/debug/dbus-mcp)
if echo "$BG_RESPONSE" | grep -q "Background task started"; then
    echo -e "${GREEN}✓ Background task execution successful${NC}"
    echo "$BG_RESPONSE" | grep -o '"text":"[^"]*"' | head -1
else
    echo -e "${RED}✗ Background task execution failed${NC}"
    echo "Response: $BG_RESPONSE"
fi
echo ""

# Check D-Bus services
echo -e "${BLUE}Checking D-Bus services...${NC}"
busctl --user list | grep dbusmcp || echo "D-Bus services not found"
echo ""

# Clean up
echo -e "${BLUE}Cleaning up...${NC}"
kill $ORCH_PID 2>/dev/null || true
sleep 1
pkill -f "dbus-agent-executor" 2>/dev/null || true
echo -e "${GREEN}✓ Test complete${NC}"
