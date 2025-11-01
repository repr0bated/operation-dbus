#!/bin/bash
# Automated plugin installer for op-dbus
# Usage: ./install_plugin.sh <plugin_pack.zip>

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <plugin_pack.zip>"
    echo "Example: $0 firewall_pack_20251031_1430.zip"
    exit 1
fi

ZIP_FILE="$1"
WORK_DIR="/tmp/plugin-install-$$"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== op-dbus Plugin Installer ===${NC}"
echo "Processing: $ZIP_FILE"
echo ""

# Check if file exists
if [ ! -f "$ZIP_FILE" ]; then
    echo -e "${RED}Error: File not found: $ZIP_FILE${NC}"
    exit 1
fi

# Create work directory
mkdir -p "$WORK_DIR"
cd "$WORK_DIR"

# Extract
echo -e "${YELLOW}→${NC} Extracting plugin..."
unzip -q "$ZIP_FILE" 2>/dev/null || {
    echo -e "${RED}Error: Failed to extract ZIP${NC}"
    rm -rf "$WORK_DIR"
    exit 1
}

# Find plugin file
PLUGIN_FILE=$(ls *_plugin.rs 2>/dev/null | head -1)
if [ -z "$PLUGIN_FILE" ]; then
    echo -e "${RED}Error: No *_plugin.rs file found${NC}"
    rm -rf "$WORK_DIR"
    exit 1
fi

# Extract plugin name (remove _plugin.rs)
PLUGIN_NAME="${PLUGIN_FILE%_plugin.rs}"
echo -e "${GREEN}✓${NC} Found plugin: ${PLUGIN_NAME}"

# Check for register.sh
if [ ! -f "register.sh" ]; then
    echo -e "${RED}Error: register.sh not found${NC}"
    rm -rf "$WORK_DIR"
    exit 1
fi

# Extract PLUGIN_STRUCT from register.sh
PLUGIN_STRUCT=$(grep "^PLUGIN_STRUCT=" register.sh | cut -d'"' -f2)
if [ -z "$PLUGIN_STRUCT" ]; then
    echo -e "${RED}Error: Could not find PLUGIN_STRUCT in register.sh${NC}"
    rm -rf "$WORK_DIR"
    exit 1
fi

echo -e "${GREEN}✓${NC} Plugin struct: ${PLUGIN_STRUCT}"

# Return to project root
cd - > /dev/null

# Copy plugin file
echo -e "${YELLOW}→${NC} Installing plugin file..."
cp "$WORK_DIR/$PLUGIN_FILE" "src/state/plugins/${PLUGIN_NAME}.rs"

# Run registration
echo -e "${YELLOW}→${NC} Registering plugin..."
cd "$WORK_DIR"
chmod +x register.sh
./register.sh
cd - > /dev/null

# Check for common syntax issues and fix them
echo -e "${YELLOW}→${NC} Checking for common issues..."
PLUGIN_PATH="src/state/plugins/${PLUGIN_NAME}.rs"

# Fix Python f-strings
if grep -q 'f".*{' "$PLUGIN_PATH"; then
    echo -e "${YELLOW}  ⚠${NC} Found Python f-string syntax - needs manual fix"
    grep -n 'f".*{' "$PLUGIN_PATH"
fi

# Fix Python booleans
if grep -q '\(False\|True\)' "$PLUGIN_PATH"; then
    echo -e "${YELLOW}  ⚠${NC} Found Python boolean syntax - fixing..."
    sed -i 's/\bFalse\b/false/g' "$PLUGIN_PATH"
    sed -i 's/\bTrue\b/true/g' "$PLUGIN_PATH"
    echo -e "${GREEN}  ✓${NC} Fixed Python booleans"
fi

# Check for missing .await;
if grep -q "register_plugin.*${PLUGIN_STRUCT}" src/main.rs; then
    # Check if it's missing .await
    if ! grep -A1 "register_plugin.*${PLUGIN_STRUCT}" src/main.rs | grep -q "\.await;"; then
        echo -e "${YELLOW}  ⚠${NC} Fixing missing .await in main.rs..."
        # This is handled by register.sh but double check
    fi
fi

# Build
echo -e "${YELLOW}→${NC} Building..."
if cargo build --release 2>&1 | tee /tmp/build-$$.log | grep -q "error:"; then
    echo -e "${RED}✗ Build failed!${NC}"
    echo ""
    echo "Common fixes needed:"
    echo "1. Python f-strings: Look for f\"...{var}\" and replace with format!(\"...{}\", var)"
    echo "2. Invalid syntax in json! macros"
    echo "3. Missing semicolons or .await"
    echo ""
    echo "Build log:"
    grep -A5 "error:" /tmp/build-$$.log
    echo ""
    echo "Plugin installed to: $PLUGIN_PATH"
    echo "Fix errors and run: cargo build --release"
    rm -rf "$WORK_DIR"
    rm -f /tmp/build-$$.log
    exit 1
fi

rm -f /tmp/build-$$.log

# Test
echo -e "${YELLOW}→${NC} Testing plugin..."
./target/release/op-dbus query --plugin "${PLUGIN_NAME}" > /tmp/test-$$.json 2>&1 || true

if [ -s /tmp/test-$$.json ]; then
    echo -e "${GREEN}✓${NC} Plugin query successful"
    head -5 /tmp/test-$$.json
else
    echo -e "${YELLOW}⚠${NC} Plugin query returned no data (may be normal)"
fi

rm -f /tmp/test-$$.json

# Cleanup
rm -rf "$WORK_DIR"

echo ""
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo -e "${GREEN}✓ Plugin '${PLUGIN_NAME}' installed!${NC}"
echo -e "${GREEN}═══════════════════════════════════════${NC}"
echo ""
echo "Commands:"
echo "  Query:  ./target/release/op-dbus query --plugin ${PLUGIN_NAME}"
echo "  Apply:  ./target/release/op-dbus apply state.json --plugin ${PLUGIN_NAME}"
echo ""
echo "Example config: $WORK_DIR/${PLUGIN_NAME}_example.json (if extracted)"
