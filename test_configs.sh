#!/bin/bash
# Test script to verify all MCP configurations

set -e

echo "=== D-Bus MCP Server Configuration Test ==="
echo

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
PASS=0
FAIL=0

# Function to check file
check_file() {
    local file=$1
    local name=$2

    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $name exists"
        ((PASS++))
        return 0
    else
        echo -e "${RED}✗${NC} $name missing"
        ((FAIL++))
        return 1
    fi
}

# Function to validate JSON
validate_json() {
    local file=$1
    local name=$2

    if python3 -m json.tool "$file" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC} $name is valid JSON"
        ((PASS++))
        return 0
    else
        echo -e "${RED}✗${NC} $name has invalid JSON"
        ((FAIL++))
        return 1
    fi
}

# Function to check binary
check_binary() {
    local binary=$1
    local name=$2

    if [ -x "$binary" ]; then
        echo -e "${GREEN}✓${NC} $name is executable"
        ((PASS++))
        return 0
    else
        echo -e "${RED}✗${NC} $name is not executable"
        ((FAIL++))
        return 1
    fi
}

# Function to check service
check_service() {
    if systemctl --user is-active --quiet dbus-orchestrator.service; then
        echo -e "${GREEN}✓${NC} dbus-orchestrator.service is running"
        ((PASS++))
        return 0
    else
        echo -e "${YELLOW}⚠${NC} dbus-orchestrator.service is not running"
        echo "  Run: systemctl --user start dbus-orchestrator.service"
        return 1
    fi
}

echo "## Testing Configuration Files"
echo

# Test Claude Code config
check_file "/git/wayfire-mcp-server/agents/dbus-orchestrator.json" "Claude Code config"
validate_json "/git/wayfire-mcp-server/agents/dbus-orchestrator.json" "Claude Code config"

# Test Cursor config
check_file "/git/wayfire-mcp-server/cursor/mcp.json" "Cursor config"
validate_json "/git/wayfire-mcp-server/cursor/mcp.json" "Cursor config"

# Test VS Code config
check_file "/git/wayfire-mcp-server/vscode/mcp.json" "VS Code config"
validate_json "/git/wayfire-mcp-server/vscode/mcp.json" "VS Code config"

echo
echo "## Testing Binaries"
echo

# Test main MCP server binary
check_file "/git/wayfire-mcp-server/target/debug/dbus-mcp" "MCP server binary"
check_binary "/git/wayfire-mcp-server/target/debug/dbus-mcp" "MCP server binary"

# Test orchestrator binary
check_file "/git/wayfire-mcp-server/target/debug/dbus-orchestrator" "Orchestrator binary"
check_binary "/git/wayfire-mcp-server/target/debug/dbus-orchestrator" "Orchestrator binary"

# Test agent binaries
for agent in executor systemd file monitor network; do
    binary="/git/wayfire-mcp-server/target/debug/dbus-agent-$agent"
    check_file "$binary" "Agent: $agent"
    check_binary "$binary" "Agent: $agent"
done

echo
echo "## Testing Services"
echo

# Test orchestrator service
check_service

echo
echo "## Testing MCP Server"
echo

# Test MCP server initialization
if echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | timeout 5 /git/wayfire-mcp-server/target/debug/dbus-mcp 2>&1 | grep -q "dbus-orchestrated-mcp-server"; then
    echo -e "${GREEN}✓${NC} MCP server responds to initialize"
    ((PASS++))
else
    echo -e "${RED}✗${NC} MCP server failed to initialize"
    ((FAIL++))
fi

# Test tools list
if echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | timeout 5 /git/wayfire-mcp-server/target/debug/dbus-mcp 2>&1 | grep -q "execute_command"; then
    echo -e "${GREEN}✓${NC} MCP server lists tools"
    ((PASS++))
else
    echo -e "${RED}✗${NC} MCP server failed to list tools"
    ((FAIL++))
fi

echo
echo "=== Summary ==="
echo -e "${GREEN}Passed: $PASS${NC}"
if [ $FAIL -gt 0 ]; then
    echo -e "${RED}Failed: $FAIL${NC}"
fi

echo
if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    echo
    echo "Ready to install:"
    echo "  Claude Code: cp agents/dbus-orchestrator.json ~/.config/Claude/mcp-servers/"
    echo "  Cursor:      cp cursor/mcp.json ~/.cursor/mcp.json"
    echo "  VS Code:     Use MCP: Open User Configuration command"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    echo "See errors above"
    exit 1
fi
