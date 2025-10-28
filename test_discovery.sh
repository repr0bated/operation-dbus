#!/bin/bash
# Test tool discovery

./target/debug/dbus-orchestrator &> /tmp/orch.log &
ORCH=$!
sleep 2

echo "=== MCP Tool Discovery Test ==="
echo ""
echo "Requesting tools/list from MCP server..."
echo ""

RESPONSE=$(echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | ./target/debug/dbus-mcp 2>/dev/null)

echo "$RESPONSE" | jq -r '.result.tools[] | "✓ \(.name)\n  Description: \(.description)\n"'

echo ""
echo "Total tools available: $(echo "$RESPONSE" | jq '.result.tools | length')"

kill $ORCH 2>/dev/null
pkill -f dbus-agent 2>/dev/null || true
