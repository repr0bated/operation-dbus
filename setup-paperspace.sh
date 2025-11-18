#!/bin/bash
# Paperspace Gradient Setup Script
# Automates Paperspace account setup and CLI configuration

set -e

echo "🚀 Paperspace Gradient Setup"
echo "============================"
echo ""

# Check if CLI is installed
if ! command -v gradient >/dev/null 2>&1; then
    echo "📦 Installing Paperspace Gradient CLI..."
    pip install paperspace
    echo "✅ CLI installed"
else
    echo "✅ Paperspace CLI already installed"
fi

# Check for API key
if [ -z "$PAPERSPACE_API_KEY" ]; then
    echo "🔑 Paperspace API Key Setup"
    echo "1. Go to https://gradient.paperspace.com"
    echo "2. Sign up/Login with your account"
    echo "3. Go to Settings > API Keys"
    echo "4. Create a new API key"
    echo "5. Copy the key and add it to your .env file:"
    echo ""
    echo "   PAPERSPACE_API_KEY=your_api_key_here"
    echo ""
    echo "Then run this script again."
    exit 1
fi

# Configure CLI
echo "🔧 Configuring Paperspace CLI..."
gradient apiKey set "$PAPERSPACE_API_KEY"
echo "✅ CLI configured"

# Test connection
echo "🧪 Testing connection..."
if gradient jobs list >/dev/null 2>&1; then
    echo "✅ Connection successful"
else
    echo "❌ Connection failed. Check your API key."
    exit 1
fi

# Show account info
echo "📊 Account Information:"
gradient account

echo ""
echo "🎉 Paperspace setup complete!"
echo ""
echo "🚀 You can now run GPU jobs:"
echo "   ./gpu-workflow.sh inference paperspace microsoft/phi-2"
echo "   ./gpu-workflow.sh train paperspace meta-llama/Llama-2-7b-chat-hf"
echo ""
echo "📚 Useful commands:"
echo "   gradient jobs list          # List your jobs"
echo "   gradient jobs logs <id>     # View job logs"
echo "   gradient machines list      # Available GPU types"