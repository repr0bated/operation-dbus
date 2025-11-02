#!/bin/bash
# Enable MCP Server - Build and configure the MCP server

set -e

echo "╔══════════════════════════════════════════════╗"
echo "║    Enabling MCP Server                      ║"
echo "╚══════════════════════════════════════════════╝"
echo ""

# Build MCP server if needed
if [ ! -f "./target/release/dbus-mcp" ]; then
    echo "📦 Building MCP server (release mode)..."
    cargo build --release --features mcp --bin dbus-mcp
    echo "✅ Build complete"
else
    echo "✅ MCP server binary already exists"
fi

# Verify binary exists
if [ ! -f "./target/release/dbus-mcp" ]; then
    echo "❌ Error: Failed to build dbus-mcp binary"
    exit 1
fi

echo ""
echo "✅ MCP server is ready!"
echo ""
echo "📍 Binary location: ./target/release/dbus-mcp"
echo ""
echo "📝 Configuration files updated:"
echo "   • mcp-configs/cursor/mcp.json"
echo "   • mcp-configs/vscode/mcp.json"
echo ""
echo "🔧 To use in Cursor:"
echo "   1. Copy config: cp mcp-configs/cursor/mcp.json ~/.cursor/mcp.json"
echo "   2. Restart Cursor"
echo ""
echo "🔧 To use in VS Code:"
echo "   1. Open Command Palette (Ctrl+Shift+P)"
echo "   2. Run: MCP: Open User Configuration"
echo "   3. Copy contents from mcp-configs/vscode/mcp.json"
echo "   4. Restart VS Code"
echo ""
echo "🚀 To test the server manually:"
echo "   ./target/release/dbus-mcp"
echo ""

