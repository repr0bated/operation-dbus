# Ollama Cloud Setup Guide

## Fixed 404 Error

The 404 error was caused by:
1. Wrong API endpoint (`api.ollama.com` instead of `ollama.com`)
2. Hardcoded API key instead of using environment variable
3. Model name not being read from environment properly

## Solution

### 1. Get Your Ollama Cloud API Key

Visit: https://ollama.com/settings/keys

Create a new API key and save it.

### 2. Set Environment Variable

```bash
export OLLAMA_API_KEY=your-ollama-cloud-api-key-here
```

### 3. Start the Server

```bash
cd /git/operation-dbus
./start-chat-ollama-cloud.sh
```

This will start the server with:
- **Model**: mistral-small3.2 (cloud version)
- **Endpoint**: https://ollama.com
- **Port**: 8080

### 4. Access the Chat UI

Open in your browser:
- Local: http://localhost:8080
- Network: http://80.209.240.244:8080

## Configuration Files

The following files have been updated to be model-agnostic:

### Rust Code
- `src/mcp/ollama.rs` - Added `OLLAMA_DEFAULT_MODEL` support
- `src/bin/chat_simple.rs` - Uses environment variable for model
- `src/bin/mcp_chat.rs` - Uses environment variable for model
- `src/mcp/chat_server.rs` - Generic AI instead of DeepSeek branding

### JavaScript
- `chat-server.js` - Now supports:
  - `OLLAMA_USE_CLOUD=true` - Use cloud API
  - `OLLAMA_DEFAULT_MODEL` - Set model name
  - `OLLAMA_API_KEY` - Your API key

## Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `OLLAMA_API_KEY` | Your Ollama Cloud API key (required for cloud) | `sk-...` |
| `OLLAMA_USE_CLOUD` | Set to `true` for cloud, omit for local | `true` |
| `OLLAMA_DEFAULT_MODEL` | Model name to use | `mistral-small3.2` |

## Local vs Cloud

### Local Ollama (Free, requires local installation)
```bash
# Don't set OLLAMA_USE_CLOUD
export OLLAMA_DEFAULT_MODEL=deepseek-coder:latest
./start-chat-local.sh
```

### Ollama Cloud (Paid, no local installation needed)
```bash
export OLLAMA_API_KEY=your-key
export OLLAMA_USE_CLOUD=true
export OLLAMA_DEFAULT_MODEL=mistral-small3.2
./start-chat-ollama-cloud.sh
```

## Available Cloud Models

Visit https://ollama.com to see available cloud models, including:
- mistral-small3.2
- llama3.3
- And many others

## Testing

Test the chat endpoint:
```bash
curl -X POST http://localhost:8080/api/chat \
  -H "Content-Type: application/json" \
  -d '{"message":"Hello!"}'
```

You should get a JSON response with the AI's message.

## Troubleshooting

### Still Getting 404?
1. Check API key is valid: `echo $OLLAMA_API_KEY`
2. Verify cloud mode is on: `echo $OLLAMA_USE_CLOUD` (should be "true")
3. Check logs: `tail -f /tmp/chat-server.log`

### Model Not Found?
1. Verify model name exists at https://ollama.com
2. Check capitalization (case-sensitive)
3. Try a different model like `llama3.3`

### Authentication Error?
1. Regenerate API key at https://ollama.com/settings/keys
2. Make sure you're using the full key (not truncated)
3. Export it again: `export OLLAMA_API_KEY=new-key`

## Success!

Once working, you'll see:
- ✅ Server startup with correct model name
- ✅ Chat responses from the AI
- ✅ No 404 or authentication errors

The system is now fully model-agnostic and works with any Ollama-compatible API (local or cloud)!
