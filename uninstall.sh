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

# Ask about blockchain data and BTRFS subvolumes
BLOCKCHAIN_DIR="/var/lib/op-dbus/blockchain"
if [ -d "$BLOCKCHAIN_DIR" ]; then
    echo ""
    echo -e "${YELLOW}Blockchain data: $BLOCKCHAIN_DIR${NC}"

    # Check if it's a BTRFS subvolume
    IS_SUBVOLUME=false
    if df -T /var/lib 2>/dev/null | grep -q btrfs; then
        if sudo btrfs subvolume show "$BLOCKCHAIN_DIR" >/dev/null 2>&1; then
            IS_SUBVOLUME=true
            echo -e "${YELLOW}⚠${NC}  This is a BTRFS subvolume"
        fi
    fi

    read -p "Remove blockchain data? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [ "$IS_SUBVOLUME" = true ]; then
            echo "Deleting BTRFS subvolume..."
            sudo btrfs subvolume delete "$BLOCKCHAIN_DIR"
            echo -e "${GREEN}✓${NC} Blockchain subvolume deleted"
        else
            rm -rf "$BLOCKCHAIN_DIR"
            echo -e "${GREEN}✓${NC} Blockchain data removed"
        fi

        # Remove parent directory if empty
        if [ -d "/var/lib/op-dbus" ] && [ -z "$(ls -A /var/lib/op-dbus)" ]; then
            rmdir /var/lib/op-dbus
            echo -e "${GREEN}✓${NC} Removed empty /var/lib/op-dbus"
        fi
    else
        echo -e "${YELLOW}⚠${NC}  Blockchain data preserved at $BLOCKCHAIN_DIR"
    fi
fi

# List any remaining op-dbus subvolumes
if df -T /var/lib 2>/dev/null | grep -q btrfs; then
    REMAINING=$(sudo btrfs subvolume list / 2>/dev/null | grep "op-dbus" || true)
    if [ -n "$REMAINING" ]; then
        echo ""
        echo -e "${YELLOW}Remaining op-dbus subvolumes:${NC}"
        echo "$REMAINING"
        echo -e "${YELLOW}⚠${NC}  Clean these up manually if needed:"
        echo "    sudo btrfs subvolume delete /path/to/subvolume"
    fi
fi

echo ""
echo -e "${GREEN}=== Uninstallation Complete ===${NC}"
