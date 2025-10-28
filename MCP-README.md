# MCP D-Bus Server

A powerful MCP (Model Context Protocol) server that provides comprehensive Linux system automation through D-Bus interfaces. This server enables AI assistants to interact with systemd services, file systems, network configurations, and execute commands securely.

## Features

### 🚀 Core Capabilities
- **100+ Auto-discovered Tools** - Automatically discovers and exposes D-Bus services as MCP tools
- **Zero Configuration** - Works out of the box with sensible defaults
- **Secure by Default** - Sandboxed operations with strict validation
- **Real-time Monitoring** - WebSocket support for live updates
- **Multi-Agent System** - Orchestrated agents for different operations

### 🛠️ Available Tools

#### System Management
- `systemd_status` - Get service status
- `systemd_start` - Start services
- `systemd_stop` - Stop services
- `systemd_restart` - Restart services
- `systemd_logs` - View service logs

#### File Operations
- `file_read` - Read file contents (with path validation)
- `file_write` - Write to files (sandboxed)
- `file_list` - List directory contents
- `file_delete` - Delete files/directories
- `file_exists` - Check file existence

#### Network Management
- `network_interfaces` - List network interfaces
- `network_connections` - Manage NetworkManager connections
- `network_connect` - Activate connections
- `network_status` - Get network status

#### Process Management
- `process_list` - List running processes
- `process_kill` - Terminate processes
- `process_info` - Get process information

#### Command Execution
- `exec_command` - Execute whitelisted commands safely

### 🔒 Security Features
- **Command Whitelisting** - Only safe commands allowed
- **Path Traversal Protection** - Prevents directory escape
- **Input Validation** - All inputs sanitized
- **Resource Limits** - CPU and memory constraints
- **Audit Logging** - All operations logged

## Installation

### Quick Start

```bash
# Clone the repository
git clone https://github.com/yourusername/operation-dbus.git
cd operation-dbus

# Build with MCP support
cargo build --release --features mcp

# Install MCP configurations
./install-mcp-configs.sh
```

### NPM Installation (when published)

```bash
npm install -g @operation-dbus/mcp-server
```

### Manual Installation

1. **Build the server:**
```bash
cargo build --release --features mcp
```

2. **Install binaries:**
```bash
sudo cp target/release/dbus-mcp /usr/local/bin/
sudo cp target/release/dbus-orchestrator /usr/local/bin/
```

3. **Configure your MCP client:**

For Claude Desktop, add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "dbus": {
      "command": "/usr/local/bin/dbus-mcp",
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

For Cursor/VSCode, add to settings:

```json
{
  "mcp.servers": [
    {
      "name": "dbus",
      "command": "/usr/local/bin/dbus-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  ]
}
```

## Usage

### Starting the Server

```bash
# Start MCP server
dbus-mcp

# With orchestrator for agent management
dbus-orchestrator &
dbus-mcp

# With debug logging
RUST_LOG=debug dbus-mcp
```

### Web Interface

```bash
# Start web interface
dbus-mcp-web

# Access at http://localhost:8080
```

### Discovery

```bash
# Discover available D-Bus services
dbus-mcp-discovery-enhanced
```

### Example Tool Usage

Once configured with your MCP client, you can use natural language:

- "Check the status of the nginx service"
- "List all files in /var/log"
- "Show network interfaces"
- "Restart the postgresql service"

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `MCP_TIMEOUT` | Request timeout (seconds) | `30` |
| `FILE_AGENT_BASE_DIR` | Base directory for file operations | `/tmp/file-agent` |
| `ALLOWED_DIRECTORIES` | Comma-separated allowed paths | `/home,/tmp,/var/log` |

### Configuration Files

#### `mcp-configs/discovery_config.toml`
```toml
[discovery]
auto_discover = true
interval = 60

[categories]
system = ["systemd", "logind", "polkit"]
network = ["NetworkManager", "resolved"]
```

#### `mcp-configs/agents/executor.json`
```json
{
  "allowed_commands": ["ls", "ps", "df", "top"],
  "timeout_seconds": 30,
  "max_output_size": 1048576
}
```

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   MCP Client    │────▶│   MCP Server     │────▶│   D-Bus System  │
│  (Claude, etc)  │◀────│   (dbus-mcp)     │◀────│    Services     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                │
                        ┌───────┴────────┐
                        │  Orchestrator  │
                        └───────┬────────┘
                                │
                ┌───────────────┼───────────────┐
                ▼               ▼               ▼
         ┌──────────┐    ┌──────────┐   ┌──────────┐
         │  Agent   │    │  Agent   │   │  Agent   │
         │(Executor)│    │  (File)  │   │(Network) │
         └──────────┘    └──────────┘   └──────────┘
```

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/yourusername/operation-dbus.git
cd operation-dbus

# Build all features
cargo build --release --all-features

# Run tests
cargo test --features mcp

# Run with logging
RUST_LOG=debug cargo run --bin dbus-mcp --features mcp
```

### Adding New Tools

1. Define tool in `src/mcp/main.rs`
2. Implement handler function
3. Add to tool list
4. Update schema

### Creating Custom Agents

See [MCP-DEVELOPER-GUIDE.md](docs/MCP-DEVELOPER-GUIDE.md) for detailed instructions.

## API Reference

### JSON-RPC Protocol

The server implements the MCP specification with JSON-RPC 2.0:

```json
// List available tools
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "id": 1
}

// Execute a tool
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "systemd_status",
    "arguments": {
      "service": "nginx"
    }
  },
  "id": 2
}
```

## Security Considerations

### Default Security Features
- ✅ Command whitelisting (only safe commands)
- ✅ Path validation (no directory traversal)
- ✅ Input sanitization (prevent injection)
- ✅ Resource limits (CPU/memory constraints)
- ✅ Audit logging (track all operations)

### Best Practices
1. Run with minimal privileges
2. Use systemd service with sandboxing
3. Configure allowed directories
4. Regular security updates
5. Monitor audit logs

## Troubleshooting

### Common Issues

**MCP server won't start:**
```bash
# Check D-Bus is running
systemctl status dbus

# Verify session bus
echo $DBUS_SESSION_BUS_ADDRESS
```

**No tools available:**
```bash
# Run discovery
dbus-mcp-discovery-enhanced

# Check with debug logging
RUST_LOG=debug dbus-mcp
```

**Permission denied errors:**
```bash
# Some operations need elevated privileges
sudo dbus-mcp

# Or configure sudoers for specific commands
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Reporting Issues

Please report security vulnerabilities privately to security@example.com

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built on the [MCP specification](https://modelcontextprotocol.io)
- Uses [zbus](https://github.com/dbus2/zbus) for D-Bus communication
- Powered by [Rust](https://www.rust-lang.org/) and [Tokio](https://tokio.rs/)

## Links

- [Documentation](https://github.com/yourusername/operation-dbus/docs)
- [API Reference](https://github.com/yourusername/operation-dbus/docs/MCP-API-REFERENCE.md)
- [Web Interface Guide](https://github.com/yourusername/operation-dbus/docs/MCP-WEB-IMPROVEMENTS.md)
- [Security Guide](https://github.com/yourusername/operation-dbus/SECURITY-FIXES.md)

---

**Version:** 1.0.0  
**Status:** Production Ready  
**Compatibility:** MCP Protocol 2024-11-05