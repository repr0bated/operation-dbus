#!/bin/bash
# Start chat server with Ollama Cloud

cd /git/operation-dbus

# Stop any existing server
pkill -f "node /git/operation-dbus/chat-server.js" 2>/dev/null
sleep 2

# Export environment variables
export OLLAMA_USE_CLOUD=true
export OLLAMA_DEFAULT_MODEL="mistral-small3.2"
export NODE_ENV=production

# Start server
nohup node chat-server.js > /tmp/chat-server.log 2>&1 &

echo "✅ Chat server starting with cloud configuration..."
echo "   Model: mistral-small3.2"
echo "   Mode: Cloud API (api.ollama.com)"
echo ""
sleep 3

# Show logs
tail -20 /tmp/chat-server.log
