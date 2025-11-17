#!/bin/bash
# Start chat server with Ollama Cloud

cd /git/operation-dbus

# Check if API key is set
if [ -z "$OLLAMA_API_KEY" ]; then
    echo "❌ Error: OLLAMA_API_KEY environment variable is not set"
    echo ""
    echo "To get an API key:"
    echo "  1. Go to https://ollama.com/settings/keys"
    echo "  2. Create a new API key"
    echo "  3. Run: export OLLAMA_API_KEY=your-key-here"
    echo ""
    echo "Then run this script again"
    exit 1
fi

# Stop any existing server
pkill -f "node /git/operation-dbus/chat-server.js" 2>/dev/null
sleep 2

# Set environment variables
export OLLAMA_USE_CLOUD=true
export OLLAMA_DEFAULT_MODEL="mistral-small3.2"
export NODE_ENV=production

# Update the chat-server.js to use the API key from environment
sed -i "s/const OLLAMA_API_KEY = '.*';/const OLLAMA_API_KEY = process.env.OLLAMA_API_KEY || '';/" chat-server.js

# Start server
nohup node chat-server.js > /tmp/chat-server.log 2>&1 &

echo "✅ Chat server starting with Ollama Cloud..."
echo "   Model: mistral-small3.2"
echo "   Endpoint: https://ollama.com"
echo "   API Key: ${OLLAMA_API_KEY:0:20}..."
echo ""
sleep 3

# Show logs
tail -20 /tmp/chat-server.log
