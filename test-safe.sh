#!/bin/bash
# Safe testing script - read-only operations first

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== op-dbus Safe Testing ===${NC}"
echo ""

# Check binary exists
if [ ! -f "/usr/local/bin/op-dbus" ]; then
    echo "Binary not installed. Run ./install.sh first"
    exit 1
fi

# Test 1: Query all plugins
echo -e "${YELLOW}Test 1: Query all plugins${NC}"
op-dbus query
echo ""

# Test 2: Query net plugin specifically
echo -e "${YELLOW}Test 2: Query network plugin${NC}"
op-dbus query --plugin net
echo ""

# Test 3: Query systemd plugin
echo -e "${YELLOW}Test 3: Query systemd plugin${NC}"
op-dbus query --plugin systemd
echo ""

# Test 4: Show diff with state file
if [ -f "/etc/op-dbus/state.json" ]; then
    echo -e "${YELLOW}Test 4: Show diff (what would change)${NC}"
    op-dbus diff /etc/op-dbus/state.json
    echo ""
fi

echo -e "${GREEN}=== All Read-Only Tests Complete ===${NC}"
echo ""
echo "Next: Review diff output before running apply"
echo "When ready: op-dbus apply /etc/op-dbus/state.json"
