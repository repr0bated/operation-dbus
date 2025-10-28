#!/bin/bash

# Smart MCP Configuration Installer
# Automatically configures MCP clients with discovered D-Bus services

set -e

echo "╔══════════════════════════════════════════════╗"
echo "║    MCP Configuration Installer              ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# Detect MCP client
detect_client() {
    if [ -d "$HOME/.config/Claude" ]; then
        echo "Claude Code"
        return 0
    elif [ -d "$HOME/.cursor" ]; then
        echo "Cursor"
        return 1
    else
        echo "Unknown"
        return 2
    fi
}

CLIENT=$(detect_client)
CLIENT_CODE=$?

case $CLIENT_CODE in
    0)
        CONFIG_DIR="$HOME/.config/Claude"
        CONFIG_FILE="$CONFIG_DIR/claude_desktop_config.json"
        echo "✅ Detected: Claude Code"
        ;;
    1)
        CONFIG_DIR="$HOME/.cursor"
        CONFIG_FILE="$CONFIG_DIR/mcp_config.json"
        echo "✅ Detected: Cursor"
        ;;
    *)
        echo "❌ No MCP client detected. Please specify the config directory:"
        read -p "Config directory path: " CONFIG_DIR
        CONFIG_FILE="$CONFIG_DIR/mcp_config.json"
        ;;
esac

echo ""
echo "📍 Configuration will be installed to:"
echo "   $CONFIG_FILE"
echo ""

# Build the project if needed
if [ ! -f "./target/release/dbus-mcp-discovery-enhanced" ]; then
    echo "📦 Building discovery service..."
    cargo build --release --bin dbus-mcp-discovery-enhanced
fi

if [ ! -f "./target/release/dbus-mcp-bridge" ]; then
    echo "📦 Building MCP bridge..."
    cargo build --release --bin dbus-mcp-bridge
fi

# Run discovery
echo "🔍 Discovering D-Bus services..."
./target/release/dbus-mcp-discovery-enhanced > /tmp/discovery.log 2>&1

# Check what was generated
GENERATED_DIR="$HOME/.config/Claude/mcp-servers"
if [ ! -d "$GENERATED_DIR" ]; then
    echo "❌ Discovery failed. Check /tmp/discovery.log for details."
    exit 1
fi

# Count discovered services
SERVICE_COUNT=$(ls -1 "$GENERATED_DIR"/*.json 2>/dev/null | grep -v "all-services\|category-" | wc -l)
echo "✅ Found $SERVICE_COUNT MCP servers"
echo ""

# Show available configurations
echo "📂 Available Configurations:"
echo "   1. All Services (all $SERVICE_COUNT servers)"
echo "   2. System Services Only"
echo "   3. Automation Services Only"
echo "   4. Custom Selection"
echo ""

read -p "Select configuration [1-4]: " CHOICE

case $CHOICE in
    1)
        SOURCE_CONFIG="$GENERATED_DIR/all-services.json"
        DESC="All Services"
        ;;
    2)
        SOURCE_CONFIG="$GENERATED_DIR/category-system.json"
        DESC="System Services"
        ;;
    3)
        SOURCE_CONFIG="$GENERATED_DIR/category-automation.json"
        DESC="Automation Services"
        ;;
    4)
        echo ""
        echo "Available servers:"
        for file in "$GENERATED_DIR"/*.json; do
            if [[ ! "$file" =~ (all-services|category-) ]]; then
                basename "$file" .json
            fi
        done
        echo ""
        echo "Enter server names (space-separated):"
        read -p "> " SELECTED_SERVERS
        
        # Build custom config
        echo "{" > /tmp/custom-mcp.json
        echo '  "mcpServers": {' >> /tmp/custom-mcp.json
        FIRST=true
        for server in $SELECTED_SERVERS; do
            if [ -f "$GENERATED_DIR/$server.json" ]; then
                if [ "$FIRST" = false ]; then
                    echo "," >> /tmp/custom-mcp.json
                fi
                echo -n "    \"$server\": " >> /tmp/custom-mcp.json
                cat "$GENERATED_DIR/$server.json" >> /tmp/custom-mcp.json
                FIRST=false
            fi
        done
        echo "" >> /tmp/custom-mcp.json
        echo "  }" >> /tmp/custom-mcp.json
        echo "}" >> /tmp/custom-mcp.json
        
        SOURCE_CONFIG="/tmp/custom-mcp.json"
        DESC="Custom Selection"
        ;;
    *)
        echo "Invalid choice"
        exit 1
        ;;
esac

# Backup existing config
if [ -f "$CONFIG_FILE" ]; then
    BACKUP="$CONFIG_FILE.backup.$(date +%Y%m%d_%H%M%S)"
    echo "📦 Backing up existing config to: $BACKUP"
    cp "$CONFIG_FILE" "$BACKUP"
fi

# Install configuration
echo "📝 Installing $DESC configuration..."
cp "$SOURCE_CONFIG" "$CONFIG_FILE"

# Install bridge binary if not already installed
if ! command -v dbus-mcp-bridge &> /dev/null; then
    echo "📦 Installing dbus-mcp-bridge to /usr/local/bin..."
    sudo cp ./target/release/dbus-mcp-bridge /usr/local/bin/
    sudo chmod +x /usr/local/bin/dbus-mcp-bridge
    echo "✅ Bridge installed"
fi

echo ""
echo "✨ Configuration Complete!"
echo ""
echo "📍 Next Steps:"
echo "1. Restart your MCP client ($CLIENT)"
echo "2. The following services will be available:"

if [ "$CHOICE" = "1" ] || [ "$CHOICE" = "2" ] || [ "$CHOICE" = "3" ]; then
    jq -r '.mcpServers | keys[]' "$SOURCE_CONFIG" | while read server; do
        echo "   • $server"
    done
else
    for server in $SELECTED_SERVERS; do
        echo "   • $server"
    done
fi

echo ""
echo "🎯 Each service appears as a separate MCP server with its own tools!"
echo ""
echo "💡 Tip: You can run this installer again to change your configuration."