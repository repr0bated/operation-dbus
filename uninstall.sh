#!/bin/bash
# op-dbus uninstallation script

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== op-dbus Uninstallation ===${NC}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}Error: Please run as root (sudo ./uninstall.sh)${NC}"
    exit 1
fi

# Stop and disable service
if systemctl is-active --quiet op-dbus; then
    echo "Stopping op-dbus service..."
    systemctl stop op-dbus
    echo -e "${GREEN}✓${NC} Service stopped"
fi

if systemctl is-enabled --quiet op-dbus 2>/dev/null; then
    echo "Disabling op-dbus service..."
    systemctl disable op-dbus
    echo -e "${GREEN}✓${NC} Service disabled"
fi

# Remove systemd service
if [ -f "/etc/systemd/system/op-dbus.service" ]; then
    echo "Removing systemd service..."
    rm -f /etc/systemd/system/op-dbus.service
    systemctl daemon-reload
    echo -e "${GREEN}✓${NC} Service removed"
fi

# Remove binary
if [ -f "/usr/local/bin/op-dbus" ]; then
    echo "Removing binary..."
    rm -f /usr/local/bin/op-dbus
    echo -e "${GREEN}✓${NC} Binary removed"
fi

# Ask about config directory
echo ""
echo -e "${YELLOW}Configuration directory: /etc/op-dbus${NC}"
read -p "Remove configuration directory? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    rm -rf /etc/op-dbus
    echo -e "${GREEN}✓${NC} Configuration removed"
else
    echo -e "${YELLOW}⚠${NC}  Configuration preserved at /etc/op-dbus"
fi

echo ""
echo -e "${GREEN}=== Uninstallation Complete ===${NC}"
