# AI Model Configuration

This system has been made model-agnostic and supports any LLM model compatible with the Ollama API.

## Environment Variables

### Required

- `OLLAMA_API_KEY` - Your Ollama API key for cloud access
  - Get from: https://ollama.com
  - Example: `export OLLAMA_API_KEY=your-api-key-here`

### Optional

- `OLLAMA_DEFAULT_MODEL` - The default model to use for chat interactions
  - Default: `llama2` (if not specified)
  - Example models:
    - `deepseek-v3.1:671b-cloud` - DeepSeek Cloud model
    - `llama2` - Llama 2
    - `mistral` - Mistral
    - `codellama` - Code Llama
    - `gemma` - Google's Gemma
    - Any other Ollama-compatible model

## Usage Examples

### Using DeepSeek (Original Configuration)
```bash
export OLLAMA_API_KEY=your-key
export OLLAMA_DEFAULT_MODEL=deepseek-v3.1:671b-cloud
cargo run --bin mcp_chat
```

### Using Llama 2
```bash
export OLLAMA_API_KEY=your-key
export OLLAMA_DEFAULT_MODEL=llama2
cargo run --bin mcp_chat
```

### Using Mistral
```bash
export OLLAMA_API_KEY=your-key
export OLLAMA_DEFAULT_MODEL=mistral
cargo run --bin chat_simple --release
```

### Using Local Ollama (No API Key Required)
```bash
# Make sure Ollama is running locally: ollama serve
export OLLAMA_DEFAULT_MODEL=llama2
cargo run --bin chat_simple
```

## Programmatic Configuration

You can also set the default model programmatically in your code:

```rust
use op_dbus::mcp::ollama::OllamaClient;

// Create client with specific model
let client = OllamaClient::cloud(api_key)
    .with_default_model("mistral".to_string());

// Or use the default from environment
let client = OllamaClient::cloud(api_key);
let model = client.default_model(); // Returns OLLAMA_DEFAULT_MODEL or "llama2"
```

## Systemd Service Configuration

Update your systemd service file to set the model:

```ini
[Service]
Environment="OLLAMA_API_KEY=your-key"
Environment="OLLAMA_DEFAULT_MODEL=llama2"
ExecStart=/path/to/your/binary
```

## Migration from DeepSeek-specific Code

All DeepSeek-specific methods have been replaced with generic equivalents:

| Old Method | New Method |
|------------|------------|
| `deepseek_cloud(key)` | `cloud(key)` |
| `deepseek_chat(msg)` | `simple_chat(model, msg)` |
| `deepseek_chat_with_tools(...)` | `chat_with_tools(model, ...)` |

The hardcoded model `"deepseek-v3.1:671b-cloud"` has been replaced with `client.default_model()` throughout the codebase.

## Benefits of Model-Agnostic Design

1. **Flexibility** - Switch between models without code changes
2. **Cost Optimization** - Use cheaper models for simple tasks
3. **Testing** - Test with local models before deploying to cloud
4. **Vendor Independence** - Not locked into a single model provider
5. **Future-Proof** - Easy to adopt new models as they become available

## Troubleshooting

If the model is not working:

1. Check your API key is valid
2. Verify the model name is correct (run `ollama list` locally or check Ollama documentation)
3. Ensure you have network connectivity to the Ollama API
4. Check logs for specific error messages

For local Ollama:
```bash
# Check if Ollama is running
curl http://localhost:11434/api/version

# List available models
ollama list

# Pull a new model
ollama pull llama2
```
