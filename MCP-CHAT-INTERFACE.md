# MCP Chat Interface

## Overview

The MCP Chat Interface provides a modern, conversational UI for interacting with the MCP (Model Context Protocol) system. It enables natural language communication with D-Bus services, tools, and agents through an intuitive web-based chat interface.

## Features

### 🎯 Core Capabilities

- **Natural Language Processing**: Convert conversational input into MCP commands
- **Real-time Communication**: WebSocket-based bidirectional messaging
- **Tool Execution**: Run system tools through simple chat commands
- **Agent Management**: Start, stop, and monitor MCP agents
- **Smart Suggestions**: Auto-completion and command suggestions
- **Conversation History**: Persistent chat history with context
- **Visual Feedback**: Rich formatting with status indicators and tool badges

### 🎨 User Interface

- **Modern Design**: Clean, responsive interface with Material Design principles
- **Dark/Light Theme**: Toggle between themes for comfortable viewing
- **Quick Commands**: One-click access to common operations
- **Tool Templates**: Pre-configured forms for complex tool parameters
- **Real-time Status**: Live connection and system status indicators
- **Mobile Responsive**: Works on desktop, tablet, and mobile devices

## Architecture

```
┌─────────────────────────────────────────────┐
│             Web Browser                      │
│  ┌─────────────────────────────────────┐    │
│  │     Chat UI (HTML/CSS/JS)           │    │
│  │  - Message Display                   │    │
│  │  - Input Handling                    │    │
│  │  - WebSocket Client                  │    │
│  └─────────────┬───────────────────────┘    │
└────────────────┼────────────────────────────┘
                 │ WebSocket
┌────────────────┼────────────────────────────┐
│                ▼                            │
│  ┌─────────────────────────────────────┐   │
│  │      Chat Server (Rust/Axum)        │   │
│  │  - WebSocket Handler                 │   │
│  │  - NLP Command Parser               │   │
│  │  - Session Management               │   │
│  └─────────────┬───────────────────────┘   │
│                │                            │
│  ┌─────────────┴───────────────────────┐   │
│  │                                      │   │
│  ▼                                      ▼   │
│ ┌──────────────┐          ┌──────────────┐ │
│ │Tool Registry │          │Agent Registry│ │
│ └──────────────┘          └──────────────┘ │
│        MCP Server                           │
└─────────────────────────────────────────────┘
```

## Installation

### Prerequisites

- Rust 1.70+ with Cargo
- D-Bus system (Linux)
- Modern web browser with WebSocket support

### Building

```bash
# Build with MCP features
cargo build --features mcp --bin mcp-chat --release

# Or use the build script
./build.sh --features mcp
```

### Running

```bash
# Start the chat server
./target/release/mcp-chat

# Or use the test script
./test-mcp-chat.sh
```

The chat interface will be available at: `http://localhost:8080/chat.html`

## Usage

### Basic Commands

The chat interface understands various natural language patterns:

#### Tool Execution
```
run systemd status service=nginx
run file read path=/etc/hosts
run network list
run process list
```

#### Agent Management
```
start agent executor
stop agent monitor
list agents
agents
```

#### System Queries
```
status
list tools
tools
help
help tools
help agents
```

### Command Patterns

The NLP processor recognizes several command patterns:

1. **Direct Commands**: `run <tool> <params>`
2. **Agent Control**: `start/stop agent <name>`
3. **Query Commands**: `status`, `list`, `help`
4. **Contextual Understanding**: Keywords trigger appropriate tools

### Tool Templates

Click on tool cards in the sidebar to open pre-configured forms:

- **Systemd**: Service management (start, stop, status, etc.)
- **File**: File operations (read, write, delete)
- **Network**: Interface management (list, up, down, configure)
- **Process**: Process control (list, kill, info)

### Keyboard Shortcuts

- `Enter`: Send message
- `Shift+Enter`: New line in message
- `Tab`: Auto-complete suggestion
- `↑/↓`: Navigate history or suggestions
- `Escape`: Close suggestions

## API Reference

### WebSocket Protocol

#### Client → Server
```javascript
// Text message (plain string)
"run systemd status service=nginx"
```

#### Server → Client
```javascript
{
  "type": "user|assistant|system|error",
  "content": "Message content",
  "timestamp": 1234567890,
  "tools_used": ["systemd"]  // Optional
}
```

### REST Endpoints

#### Get Suggestions
```http
POST /chat/api/suggestions
Content-Type: application/json

{
  "partial": "run sys"
}

Response: ["run systemd status service=", "run systemd start service="]
```

#### Get History
```http
POST /chat/api/history
Content-Type: application/json

{
  "id": "conversation-id"
}

Response: [/* Array of ChatMessage objects */]
```

## Configuration

### Server Configuration

The chat server can be configured through environment variables:

```bash
# Server port (default: 8080)
export MCP_CHAT_PORT=8080

# Max message history (default: 100)
export MCP_CHAT_HISTORY_SIZE=100

# WebSocket timeout (default: 60s)
export MCP_CHAT_WS_TIMEOUT=60
```

### Client Configuration

Customize the UI through localStorage:

```javascript
// Set theme preference
localStorage.setItem('theme', 'dark');  // or 'light'

// Set auto-complete preferences
localStorage.setItem('enableSuggestions', 'true');
```

## Development

### Adding Custom Tools

Extend the chat server with new tools:

```rust
struct CustomTool;

#[async_trait::async_trait]
impl Tool for CustomTool {
    fn name(&self) -> &str { "custom" }
    fn description(&self) -> &str { "Custom tool" }
    
    async fn execute(&self, params: Value) -> Result<ToolResult> {
        // Implementation
    }
}

// Register in main.rs
registry.register_tool(Box::new(CustomTool)).await?;
```

### Extending NLP

Add new command patterns in `chat_server.rs`:

```rust
impl NaturalLanguageProcessor {
    pub fn parse_command(input: &str) -> ParsedCommand {
        // Add custom patterns
        if input.contains("custom keyword") {
            CommandIntent::ExecuteTool {
                tool_name: "custom".to_string()
            }
        }
    }
}
```

### Custom UI Themes

Add CSS variables for theming:

```css
:root {
    --bg-primary: #0f1419;
    --accent-primary: #58a6ff;
    /* Add custom variables */
}
```

## Security Considerations

- **Input Validation**: All user input is sanitized before processing
- **Command Allowlisting**: Tools validate parameters against allowed values
- **Session Isolation**: Each WebSocket connection has isolated context
- **Rate Limiting**: Consider implementing rate limits for production
- **Authentication**: Add authentication layer for production deployments

## Troubleshooting

### Connection Issues

```bash
# Check if server is running
ps aux | grep mcp-chat

# Check port availability
netstat -tulpn | grep 8080

# View server logs
RUST_LOG=debug ./target/release/mcp-chat
```

### WebSocket Errors

1. Check browser console for errors
2. Verify WebSocket support: `typeof WebSocket !== 'undefined'`
3. Check CORS settings if accessing from different domain

### Command Not Recognized

1. Check available tools: Type `list tools`
2. Verify command syntax: Type `help`
3. Use tool templates for complex commands

## Performance Optimization

- **Message Batching**: Groups multiple updates in single WebSocket frame
- **Lazy Loading**: Loads UI components on demand
- **Connection Pooling**: Reuses D-Bus connections
- **Async Processing**: Non-blocking tool execution

## Future Enhancements

- [ ] Voice input/output support
- [ ] Multi-language support
- [ ] Collaborative sessions
- [ ] Command macros and shortcuts
- [ ] Plugin system for UI extensions
- [ ] Mobile native apps
- [ ] End-to-end encryption
- [ ] Advanced NLP with LLM integration

## License

MIT License - See LICENSE file for details

## Contributing

Contributions welcome! Please read CONTRIBUTING.md for guidelines.

## Support

For issues and questions:
- GitHub Issues: [Report bugs](https://github.com/your-org/operation-dbus/issues)
- Documentation: [MCP-COMPLETE-GUIDE.md](./docs/MCP-COMPLETE-GUIDE.md)