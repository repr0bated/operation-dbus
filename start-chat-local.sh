#!/bin/bash
# Start chat server with LOCAL Ollama

cd /git/operation-dbus

# Stop any existing server
pkill -f "node /git/operation-dbus/chat-server.js" 2>/dev/null
sleep 2

# DO NOT set OLLAMA_USE_CLOUD - this makes it use localhost:11434
export OLLAMA_DEFAULT_MODEL="deepseek-coder:latest"
export NODE_ENV=production

# Start server
nohup node chat-server.js > /tmp/chat-server.log 2>&1 &

echo "✅ Chat server starting with LOCAL Ollama..."
echo "   Model: deepseek-coder:latest"
echo "   Endpoint: http://localhost:11434"
echo ""
sleep 3

# Show logs
tail -20 /tmp/chat-server.log
