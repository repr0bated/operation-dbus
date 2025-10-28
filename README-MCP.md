# MCP Branch - D-Bus System Automation

> **Note:** You're on the `mcp` branch - this branch focuses on MCP (Model Context Protocol) components and can be easily submitted to MCP registries.

## 🚀 Quick Start

This branch contains a complete MCP server implementation for Linux system automation through D-Bus.

### Installation

```bash
# Build all MCP components
cargo build --release --features mcp

# Or build specific binaries
cargo build --release --bin mcp-chat        # Chat interface
cargo build --release --bin dbus-mcp        # Main MCP server
cargo build --release --bin dbus-mcp-web    # Web interface
```

### Running

```bash
# Start the chat interface (recommended for beginners)
./target/release/mcp-chat
# Open http://localhost:8080/chat.html

# Or start the main MCP server
./target/release/dbus-mcp

# Or start the web interface
./target/release/dbus-mcp-web
```

## ✨ Features

### 🎯 Chat Interface (NEW!)
- **Natural language commands**: Just type what you want
- **Real-time WebSocket communication**
- **Smart suggestions and auto-completion**
- **Tool templates with guided forms**
- **Dark/Light theme**
- **Mobile responsive**

Example commands:
```
"run systemd status nginx"
"start agent executor"
"list all tools"
"help me with network commands"
```

### 🔧 100+ Auto-Discovered Tools
- **Systemd**: Service management (start, stop, restart, status)
- **Network**: Interface control, routing, firewall
- **File Operations**: Read, write, delete with safety checks
- **Process Management**: List, monitor, control processes
- **D-Bus Introspection**: Auto-discover services

### 🤖 Multi-Agent System
- **Executor Agent**: Command execution with allowlisting
- **File Agent**: Secure file operations
- **Network Agent**: Network management
- **Systemd Agent**: Service control
- **Monitor Agent**: System monitoring

### 🌐 Web Interfaces
- **Main Web UI**: Visual tool execution
- **Chat Interface**: Conversational interaction
- **Real-time status monitoring**
- **WebSocket-based updates**

### 🔒 Security
- Input validation and sanitization
- Command allowlisting
- Path traversal prevention
- Encrypted state storage (AES-256-GCM)
- Audit logging

## 📦 What's in This Branch

### Core Components

```
src/mcp/                      # MCP implementation
├── main.rs                   # Main MCP server
├── orchestrator.rs           # Agent orchestrator
├── bridge.rs                 # D-Bus bridge
├── chat_server.rs           # Chat backend ⭐ NEW
├── chat_main.rs             # Chat application ⭐ NEW
├── tool_registry.rs         # Dynamic tool system
├── agent_registry.rs        # Dynamic agent system
├── agents/                   # Agent implementations
│   ├── executor.rs          # Command executor
│   ├── file.rs              # File operations
│   ├── network.rs           # Network management
│   ├── systemd.rs           # Service control
│   └── monitor.rs           # System monitoring
└── web/                      # Web interfaces
    ├── chat.html            # Chat UI ⭐ NEW
    ├── chat.js              # Chat client ⭐ NEW
    ├── chat-styles.css      # Chat themes ⭐ NEW
    ├── index.html           # Main UI
    ├── app.js               # Main app
    └── styles.css           # Main styles

src/plugin_system/           # Generic plugin architecture
src/event_bus/              # Event-driven system
```

### Documentation

- `MCP-CHAT-INTERFACE.md` - Chat interface guide ⭐
- `MCP-INTEGRATION.md` - Integration guide
- `MCP-WEB-IMPROVEMENTS.md` - Web UI documentation
- `SECURITY-FIXES.md` - Security enhancements
- `COUPLING-FIXES.md` - Architecture improvements
- `docs/MCP-COMPLETE-GUIDE.md` - Comprehensive guide
- `docs/MCP-API-REFERENCE.md` - API documentation
- `docs/MCP-DEVELOPER-GUIDE.md` - Developer guide

### Configuration

- `package.json` - NPM package metadata
- `mcp.json` - MCP client configuration
- `claude_desktop_config.json` - Claude Desktop config
- `mcp-configs/` - Runtime configurations

## 🎯 Use Cases

### For AI Assistants (Claude, GPT, etc.)
```json
{
  "mcpServers": {
    "dbus": {
      "command": "dbus-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### For System Administrators
```bash
# Use chat interface for natural commands
./target/release/mcp-chat

# Or script with the CLI
./target/release/dbus-mcp query systemd status nginx
```

### For Developers
```rust
use op_dbus::mcp::{tool_registry::*, agent_registry::*};

// Register custom tools
registry.register_tool(Box::new(MyCustomTool)).await?;

// Spawn agents dynamically
registry.spawn_agent("my-agent", config).await?;
```

## 📚 Documentation

### Getting Started
1. [Complete Guide](docs/MCP-COMPLETE-GUIDE.md) - Everything you need
2. [Chat Interface Guide](MCP-CHAT-INTERFACE.md) - Using the chat UI
3. [Integration Guide](MCP-INTEGRATION.md) - Integrating with your tools

### Reference
- [API Reference](docs/MCP-API-REFERENCE.md) - Complete API docs
- [Developer Guide](docs/MCP-DEVELOPER-GUIDE.md) - Extending MCP
- [Security Documentation](SECURITY-FIXES.md) - Security features

### Quick References
- [Architecture](COUPLING-FIXES.md) - System design
- [Web UI Guide](MCP-WEB-IMPROVEMENTS.md) - Web interfaces

## 🧪 Testing

```bash
# Test the chat interface
./test-mcp-chat.sh

# Run all tests
cargo test --features mcp

# Test specific component
cargo test --features mcp --bin dbus-mcp
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────┐
│          AI Assistant / User                 │
└─────────────┬───────────────────────────────┘
              │
    ┌─────────┴─────────┐
    │                   │
    ▼                   ▼
┌─────────┐      ┌─────────────┐
│Chat UI  │      │  MCP Client │
│(Browser)│      │  (Claude)   │
└────┬────┘      └──────┬──────┘
     │ WebSocket        │ JSON-RPC
     ▼                  ▼
┌────────────────────────────────┐
│       MCP Server               │
│  ┌──────────────────────────┐  │
│  │   Tool Registry          │  │
│  │   Agent Registry         │  │
│  │   Event Bus              │  │
│  └──────────────────────────┘  │
└────────────┬───────────────────┘
             │
    ┌────────┴────────┐
    │                 │
    ▼                 ▼
┌─────────┐      ┌─────────┐
│ Agents  │      │  D-Bus  │
│ (Rust)  │◄────►│ System  │
└─────────┘      └─────────┘
                      │
              ┌───────┴───────┐
              │               │
              ▼               ▼
        ┌─────────┐    ┌──────────┐
        │Systemd  │    │NetworkMgr│
        └─────────┘    └──────────┘
```

## 🔧 Development

### Adding Custom Tools

```rust
struct MyTool;

#[async_trait::async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Does something" }
    
    async fn execute(&self, params: Value) -> Result<ToolResult> {
        // Implementation
    }
}

// Register
registry.register_tool(Box::new(MyTool)).await?;
```

### Creating Agents

```rust
struct MyAgent;

#[zbus::interface(name = "org.dbusmcp.Agent.MyAgent")]
impl MyAgent {
    async fn do_something(&self, param: String) -> zbus::fdo::Result<String> {
        // Implementation
    }
}
```

## 🚢 Submitting to MCP Registry

This branch is ready to be submitted to MCP registries:

1. **GitHub URL**: Use this branch URL
   ```
   https://github.com/repr0bated/operation-dbus/tree/mcp
   ```

2. **Package Name**: `mcp-dbus` or `dbus-mcp`

3. **Description**: "MCP server for Linux system automation via D-Bus"

4. **Keywords**: `mcp`, `dbus`, `linux`, `systemd`, `automation`

## 🤝 Contributing

This branch contains the MCP implementation. For contributing:

1. Fork the repository
2. Work on the `mcp` branch
3. Submit PR to `mcp` branch
4. Follow guidelines in `CONTRIBUTING.md`

## 📊 Statistics

- **Tools**: 100+ auto-discovered
- **Agents**: 5 built-in, extensible
- **Binaries**: 11 executables
- **Lines of Code**: ~15,000
- **Documentation**: ~100 KB

## 🆘 Troubleshooting

### Connection Issues
```bash
# Check D-Bus
systemctl status dbus

# Check permissions
groups  # Should include 'users' or appropriate groups

# Enable verbose logging
RUST_LOG=debug ./target/release/dbus-mcp
```

### Build Issues
```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build --release --features mcp
```

### Runtime Issues
```bash
# Check logs
journalctl -u dbus-orchestrator -f

# Test D-Bus connection
dbus-send --system --print-reply \
  --dest=org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.DBus.Introspectable.Introspect
```

## 📝 License

MIT License - See LICENSE file for details

## 🔗 Links

- **Main Repository**: https://github.com/repr0bated/operation-dbus
- **MCP Branch**: https://github.com/repr0bated/operation-dbus/tree/mcp
- **Issues**: https://github.com/repr0bated/operation-dbus/issues
- **MCP Specification**: https://modelcontextprotocol.io

## 🎉 Highlights

### What Makes This Special

1. **Native Linux Integration**: Direct D-Bus communication, no shell scripts
2. **Auto-Discovery**: Automatically discovers available D-Bus services
3. **Secure by Default**: Input validation, allowlisting, encryption
4. **Real-time Updates**: WebSocket-based live monitoring
5. **Natural Language**: Chat interface with NLP
6. **Extensible**: Plugin system for custom tools and agents
7. **Well Documented**: Comprehensive guides and references
8. **Production Ready**: Error handling, logging, security

### Perfect For

- 🤖 AI assistant integration (Claude, GPT, etc.)
- 🖥️ System administrators
- 🔧 DevOps automation
- 📊 System monitoring
- 🔒 Secure remote management
- 🎓 Learning D-Bus and system programming

---

**Ready to start?** Try the chat interface:

```bash
cargo run --release --bin mcp-chat
# Open http://localhost:8080/chat.html
```

Have fun automating your Linux system! 🚀