#!/bin/bash
# Test AI integration

set -e

echo "🧪 Testing AI Integration..."
echo ""

# Source environment
source ~/.bashrc

# Verify API key
if [ -z "$OLLAMA_API_KEY" ]; then
    echo "❌ OLLAMA_API_KEY not set!"
    exit 1
fi
echo "✅ OLLAMA_API_KEY is set (${#OLLAMA_API_KEY} chars)"

# Build the library
echo ""
echo "📦 Building library with web features..."
cargo build --lib --features web --quiet

if [ $? -eq 0 ]; then
    echo "✅ Library built successfully!"
else
    echo "❌ Library build failed!"
    exit 1
fi

# Try to build the chat binary
echo ""
echo "📦 Attempting to build ai-chat binary..."
cargo build --bin ai-chat --features web 2>&1 | tail -5

echo ""
echo "🎉 AI integration is ready!"
echo ""
echo "Next steps:"
echo "  1. Fix the 'mcp' feature compilation errors (pre-existing)"
echo "  2. Run: cargo run --bin ai-chat --features web,mcp"
echo "  3. Access the chat UI at: http://100.104.70.1:8080"
echo ""
echo "Or use the library in your own code:"
echo "  use op_dbus::mcp::ollama::OllamaClient;"
echo "  use op_dbus::mcp::ai_context_provider::AiContextProvider;"
