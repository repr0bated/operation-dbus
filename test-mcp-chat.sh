#!/bin/bash

# Test script for MCP Chat Interface

set -e

echo "🚀 MCP Chat Interface Test Script"
echo "================================="

# Build the chat binary
echo "Building MCP Chat..."
cargo build --features mcp --bin mcp-chat --release

# Check if build succeeded
if [ ! -f "target/release/mcp-chat" ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"

# Start the chat server in background
echo ""
echo "Starting MCP Chat Server..."
target/release/mcp-chat &
SERVER_PID=$!

# Give server time to start
sleep 2

# Check if server is running
if ! ps -p $SERVER_PID > /dev/null; then
    echo "❌ Server failed to start!"
    exit 1
fi

echo "✅ Server started (PID: $SERVER_PID)"
echo ""
echo "📋 Chat Interface is available at:"
echo "   http://localhost:8080/chat.html"
echo ""
echo "🔧 Features:"
echo "   • Natural language command interface"
echo "   • Real-time WebSocket communication"
echo "   • Tool execution (systemd, file, network, process)"
echo "   • Agent management"
echo "   • Command suggestions and auto-completion"
echo "   • Dark/Light theme"
echo ""
echo "Press Ctrl+C to stop the server..."

# Wait for user interrupt
trap "kill $SERVER_PID 2>/dev/null; echo ''; echo 'Server stopped.'; exit 0" INT
wait $SERVER_PID