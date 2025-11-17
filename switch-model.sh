#!/bin/bash
# AI Model Switcher - Supports Gemini and Ollama
# Usage: ./switch-model.sh [gemini|ollama] [list|select <model-id>]

# Function to restart chat server
restart_chat_server() {
    echo "  🛑 Stopping existing chat server..."

    # Kill existing chat server processes
    pkill -f "node.*chat-server.js" 2>/dev/null || true
    sleep 2

    echo "  🚀 Starting new chat server..."

    # Start chat server in background
    cd /git/operation-dbus
    export AI_PROVIDER="${AI_PROVIDER:-ollama}"
    export OLLAMA_USE_CLOUD="${OLLAMA_USE_CLOUD:-true}"
    export OLLAMA_DEFAULT_MODEL="${OLLAMA_DEFAULT_MODEL}"
    export OLLAMA_API_KEY="${OLLAMA_API_KEY}"
    export GEMINI_MODEL="${GEMINI_MODEL}"

    nohup node chat-server.js > /tmp/chat-server.log 2>&1 &
    sleep 3

    # Check if server started successfully
    if curl -s http://localhost:8080/api/health >/dev/null 2>&1; then
        echo "  ✅ Chat server restarted successfully on http://localhost:8080"
    else
        echo "  ⚠️  Chat server may have issues - check /tmp/chat-server.log"
    fi
}

# Default provider if not specified
PROVIDER="${1:-gemini}"

# Shift arguments if provider was specified
if [[ "$1" == "gemini" || "$1" == "ollama" ]]; then
    shift
fi

COMMAND="$1"
MODEL_ID="$2"

# Provider-specific configuration
case "$PROVIDER" in
    gemini)
        API_BASE="http://localhost:8080"
        API_MODELS="$API_BASE/api/models"
        API_SELECT="$API_BASE/api/models/select"
        PROVIDER_NAME="Gemini"
        export AI_PROVIDER="gemini"
        ;;
    ollama)
        PROVIDER_NAME="Ollama"
        export AI_PROVIDER="ollama"
        ;;
    *)
        echo "❌ Error: Unknown provider '$PROVIDER'"
        echo "Supported providers: gemini, ollama"
        exit 1
        ;;
esac

case "$COMMAND" in
    list)
        echo "📋 Available $PROVIDER_NAME Models:"
        echo "─────────────────────────────────────────────────"

        case "$PROVIDER" in
            gemini)
                # Get models from API
                API_MODELS_LIST=$(curl -s "$API_MODELS" | jq -r '
                    (.availableModels[] | "  [\(.id)]  \(.name)")
                ' 2>/dev/null || echo "")

                # Add high-rate-limit models that might not be in the API
                echo "Current: $(curl -s "$API_MODELS" | jq -r '.currentModel // "Unknown"')"
                echo "Available Models:"
                echo "$API_MODELS_LIST"

                # Add additional high-rate-limit models
                echo "  [gemini-2.0-flash-lite]  Gemini 2.0 Flash Lite (0/10, 0/250K, 0/500)"
                echo "  [gemini-2.0-flash]  Gemini 2.0 Flash (0/2K, 0/4M, Unlimited)"
                echo "  [gemini-2.5-flash-lite]  Gemini 2.5 Flash Lite (0/4K, 0/4M, Unlimited)"
                echo "  [gemini-2.0-flash-preview-image-generation]  Gemini 2.0 Flash Preview Image Generation (0/1K, 0/1M, 0/10K)"
                echo "  [gemini-2.5-flash-preview-image]  Gemini 2.5 Flash Preview Image (0/500, 0/500K, 0/2K)"
                echo "  [gemini-2.5-flash]  Gemini 2.5 Flash (0/1K, 0/1M, 0/10K)"
                echo "  [gemini-2.5-flash-tts]  Gemini 2.5 Flash TTS (0/10, 0/10K, 0/100)"
                echo "  [gemini-2.0-flash-live]  Gemini 2.0 Flash Live (Unlimited, 0/4M, Unlimited)"
                echo "  [gemini-2.5-flash-live]  Gemini 2.5 Flash Live (Unlimited, 0/1M, Unlimited)"
                echo "  [gemini-2.5-flash-native-audio-dialog]  Gemini 2.5 Flash Native Audio Dialog (Unlimited, 0/1M, Unlimited)"
                echo "  [gemini-robotics-er-1.5-preview]  Gemini Robotics ER 1.5 Preview (0/1K, 0/2M, 0/10K)"
                echo "  [computer-use-preview]  Computer Use Preview"
                ;;
            ollama)
                # Determine host based on whether we have API key
                if [ -n "$OLLAMA_API_KEY" ] || [ "$OLLAMA_USE_CLOUD" = "true" ]; then
                    API_HOST="${OLLAMA_HOST:-https://ollama.com}"
                else
                    API_HOST="${OLLAMA_HOST:-http://localhost:11434}"
                fi

                # Include API key for cloud models
                if [ -n "$OLLAMA_API_KEY" ]; then
                    AUTH_HEADERS="-H \"Authorization: Bearer $OLLAMA_API_KEY\" -H \"X-API-Key: $OLLAMA_API_KEY\""
                else
                    AUTH_HEADERS=""
                fi

                # Get running model
                CURRENT=$(eval "curl -s $AUTH_HEADERS \"$API_HOST/api/tags\"" | jq -r '.models[] | select(.running == true) | .name' | head -1)
                if [ -n "$CURRENT" ]; then
                    echo "Current: $CURRENT"
                else
                    echo "Current: None"
                fi
                echo "Available Models:"
                eval "curl -s $AUTH_HEADERS \"$API_HOST/api/tags\"" | jq -r '.models[] | "  [\(.name)]  \(.name)"'
                ;;
        esac
        ;;
    select)
        if [ -z "$MODEL_ID" ]; then
            echo "❌ Error: Please specify a model ID"
            echo "Usage: $0 $PROVIDER select <model-id>"
            echo "Run '$0 $PROVIDER list' to see available models"
            exit 1
        fi

        echo "🔄 Switching to $PROVIDER_NAME model: $MODEL_ID"

        case "$PROVIDER" in
            gemini)
                # Check if this is one of the high-rate-limit models that might not be in the API
                HIGH_RATE_MODELS="gemini-2.0-flash-lite gemini-2.0-flash gemini-2.5-flash-lite gemini-2.0-flash-preview-image-generation gemini-2.5-flash-preview-image gemini-2.5-flash gemini-2.5-flash-tts gemini-2.0-flash-live gemini-2.5-flash-live gemini-2.5-flash-native-audio-dialog gemini-robotics-er-1.5-preview computer-use-preview"

                if echo "$HIGH_RATE_MODELS" | grep -q "$MODEL_ID"; then
                    # Direct model switch for high-rate-limit models
                    echo "✅ Switching to high-rate-limit model: $MODEL_ID"

                    # Update environment variables for chat server
                    export GEMINI_MODEL="$MODEL_ID"
                    echo "💡 Environment updated: GEMINI_MODEL=$MODEL_ID"

                    # Restart the chat server
                    echo "🔄 Restarting chat server..."
                    restart_chat_server
                else
                    # Use API for other models
                    RESULT=$(curl -s -X POST "$API_SELECT" \
                        -H "Content-Type: application/json" \
                        -d "{\"modelId\":\"$MODEL_ID\"}")

                    if echo "$RESULT" | jq -e '.success' > /dev/null; then
                        MODEL_NAME=$(echo "$RESULT" | jq -r '.modelName')
                        echo "✅ Successfully switched to: $MODEL_NAME"

                        # Update environment variables for chat server
                        export GEMINI_MODEL="$MODEL_ID"
                        echo "💡 Environment updated: GEMINI_MODEL=$MODEL_ID"

                        # Restart the chat server
                        echo "🔄 Restarting chat server..."
                        restart_chat_server
                    else
                        ERROR=$(echo "$RESULT" | jq -r '.error')
                        echo "❌ Error: $ERROR"
                        exit 1
                    fi
                fi
                ;;
            ollama)
                # Determine host based on whether we have API key
                if [ -n "$OLLAMA_API_KEY" ] || [ "$OLLAMA_USE_CLOUD" = "true" ]; then
                    API_HOST="${OLLAMA_HOST:-https://ollama.com}"
                else
                    API_HOST="${OLLAMA_HOST:-http://localhost:11434}"
                fi

                # Test the model with a simple chat request
                # Include API key for cloud models
                if [ -n "$OLLAMA_API_KEY" ]; then
                    AUTH_HEADERS="-H \"Authorization: Bearer $OLLAMA_API_KEY\" -H \"X-API-Key: $OLLAMA_API_KEY\""
                else
                    AUTH_HEADERS=""
                fi

                RESULT=$(eval "curl -s -X POST \"$API_HOST/api/chat\" \
                    -H \"Content-Type: application/json\" \
                    $AUTH_HEADERS \
                    -d '{\"model\":\"$MODEL_ID\",\"messages\":[{\"role\":\"user\",\"content\":\"test\"}],\"stream\":false}'")

                if echo "$RESULT" | jq -e '.message' > /dev/null; then
                    echo "✅ Successfully switched to: $MODEL_ID"

                    # Update environment variables for chat server
                    export OLLAMA_DEFAULT_MODEL="$MODEL_ID"
                    echo "💡 Environment updated: OLLAMA_DEFAULT_MODEL=$MODEL_ID"

                    # Restart the chat server
                    echo "🔄 Restarting chat server..."
                    restart_chat_server
                else
                    ERROR=$(echo "$RESULT" | jq -r '.error // "Unknown error"')
                    echo "❌ Error: $ERROR"
                    exit 1
                fi
                ;;
        esac
        ;;
    *)
        echo "AI Model Switcher - $PROVIDER_NAME"
        echo ""
        echo "Usage:"
        echo "  $0 [provider] list                    - List all available models"
        echo "  $0 [provider] select <model-id>       - Switch to a specific model"
        echo ""
        echo "Providers:"
        echo "  gemini    - Google Gemini models (default)"
        echo "  ollama    - Ollama local models"
        echo ""
        echo "Examples:"
        echo "  $0 list                           # List Gemini models"
        echo "  $0 gemini list                    # Same as above"
        echo "  $0 ollama list                    # List Ollama models"
        echo "  $0 select gemini-1.5-pro          # Switch to Gemini model"
        echo "  $0 gemini select gemini-2.0-flash-exp  # Same as above"
        echo "  $0 ollama select llama3.2:3b      # Switch to Ollama model"
        echo "  $0 ollama select qwen3-vl:235b    # Switch to specific Ollama model"
        ;;
esac
