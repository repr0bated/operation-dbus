#!/bin/bash
# Batch installer for multiple plugins
# Usage: ./install_all_plugins.sh [directory]
# Default directory: /home/jeremy

PLUGIN_DIR="${1:-/home/jeremy}"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}=== Batch Plugin Installer ===${NC}"
echo "Scanning: $PLUGIN_DIR"
echo ""

# Find all plugin packs
PLUGINS=($(find "$PLUGIN_DIR" -maxdepth 1 -name "*_pack_*.zip" -type f 2>/dev/null | sort))

if [ ${#PLUGINS[@]} -eq 0 ]; then
    echo "No plugin packs found in $PLUGIN_DIR"
    exit 0
fi

echo "Found ${#PLUGINS[@]} plugin pack(s):"
for plugin in "${PLUGINS[@]}"; do
    echo "  - $(basename "$plugin")"
done
echo ""

# Track results
SUCCESS=0
FAILED=0
SKIPPED=0

for plugin_zip in "${PLUGINS[@]}"; do
    PLUGIN_NAME=$(basename "$plugin_zip" .zip)

    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Processing: $PLUGIN_NAME${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    # Extract plugin name from filename (e.g., firewall_pack_20251031_1430.zip -> firewall)
    ACTUAL_NAME=$(echo "$PLUGIN_NAME" | sed 's/_pack_[0-9_]*//')

    # Check if already installed
    if [ -f "src/state/plugins/${ACTUAL_NAME}.rs" ]; then
        echo -e "⚠ Plugin '${ACTUAL_NAME}' already installed - skipping"
        SKIPPED=$((SKIPPED + 1))
        echo ""
        continue
    fi

    # Install
    if ./install_plugin.sh "$plugin_zip"; then
        SUCCESS=$((SUCCESS + 1))
    else
        FAILED=$((FAILED + 1))
        echo -e "${RED}✗ Failed to install $PLUGIN_NAME${NC}"
    fi

    echo ""
done

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}Installation Summary${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Successful: $SUCCESS${NC}"
[ $FAILED -gt 0 ] && echo -e "${RED}✗ Failed: $FAILED${NC}"
[ $SKIPPED -gt 0 ] && echo "⚠ Skipped: $SKIPPED"
echo ""

# List all installed plugins
echo "Installed plugins:"
ls -1 src/state/plugins/*.rs | grep -v mod.rs | while read f; do
    PNAME=$(basename "$f" .rs)
    echo "  - $PNAME"
done

echo ""
echo "Test all: for p in \$(ls src/state/plugins/*.rs | grep -v mod.rs | xargs -n1 basename | sed 's/.rs//'); do echo \"=== \$p ===\"
; ./target/release/op-dbus query --plugin \$p 2>/dev/null | head -5; done"
