# MCP (Model Context Protocol) Integration

## Overview

The MCP functionality has been integrated into the main operation-dbus codebase as an optional feature. This provides a Model Context Protocol server with D-Bus orchestration for Linux system automation.

## Features

- **Automatic Discovery** - Finds all D-Bus services and exposes them as MCP tools
- **Universal Bridge** - Any D-Bus service becomes MCP-accessible instantly
- **100+ Tools** - Auto-discovered from systemd, NetworkManager, and custom services
- **Smart Organization** - Services grouped by category (system, network, automation)
- **Zero Configuration** - New D-Bus services work without code changes

## Building with MCP Support

### Enable MCP Feature

```bash
# Build with MCP support
cargo build --release --features mcp

# Or build with all features
cargo build --release --all-features
```

### MCP Binaries

When built with the `mcp` feature, the following binaries are available:

- `dbus-mcp` - Main MCP server
- `dbus-orchestrator` - D-Bus orchestrator service
- `dbus-mcp-web` - Web interface for MCP
- `dbus-mcp-bridge` - Bridge between MCP and D-Bus
- `dbus-mcp-discovery` - Service discovery tool
- `dbus-mcp-discovery-enhanced` - Enhanced discovery with categorization
- `introspection-parser` - Parse D-Bus introspection data
- `dbus-agent-executor` - Command execution agent
- `dbus-agent-systemd` - Systemd management agent  
- `dbus-agent-file` - File operations agent
- `dbus-agent-monitor` - System monitoring agent
- `dbus-agent-network` - Network management agent

## Installation

### Quick Setup

```bash
# Install MCP configurations
./install-mcp-configs.sh

# This will:
# - Build all MCP components
# - Discover available D-Bus services
# - Generate MCP configurations
# - Configure your MCP client (Claude/Cursor)
```

### Manual Installation

```bash
# Build with MCP support
cargo build --release --features mcp

# Run discovery
./target/release/dbus-mcp-discovery-enhanced

# Install bridge system-wide (optional)
sudo cp ./target/release/dbus-mcp-bridge /usr/local/bin/

# Configure your MCP client
cp mcp-configs/cursor/mcp.json ~/.config/Claude/
# or for VSCode
cp mcp-configs/vscode/mcp.json ~/.vscode/
```

## Configuration

### Discovery Configuration

The discovery configuration is located at `mcp-configs/discovery_config.toml`:

```toml
# Service categories for organization
[categories]
system = ["systemd", "logind", "polkit"]
network = ["NetworkManager", "resolved"]
desktop = ["gnome", "kde", "xfce"]
```

### Agent Configuration

Agent configurations are in `mcp-configs/agents/`:
- `dbus-executor.json` - Executor agent config
- `dbus-orchestrator.json` - Orchestrator config

### MCP Client Configuration

For Cursor/Claude: `mcp-configs/cursor/mcp.json`
For VSCode: `mcp-configs/vscode/mcp.json`

## Usage

### Start MCP Server

```bash
# Basic MCP server
./target/release/dbus-mcp

# With orchestrator
./target/release/dbus-orchestrator &
./target/release/dbus-mcp
```

### Web Interface

```bash
# Start web interface on port 8080
./target/release/dbus-mcp-web
```

### Discovery

```bash
# Discover all D-Bus services
./target/release/dbus-mcp-discovery

# Enhanced discovery with categories
./target/release/dbus-mcp-discovery-enhanced
```

## Testing

```bash
# Test MCP discovery
./test_discovery.sh

# Test web interface
./test_web.sh

# Test system integration
./test_system.sh
```

## Architecture

The MCP integration follows a modular architecture:

```
src/mcp/
├── agents/          # Specialized D-Bus agents
│   ├── executor.rs  # Command execution
│   ├── file.rs      # File operations
│   ├── monitor.rs   # System monitoring
│   ├── network.rs   # Network management
│   └── systemd.rs   # Systemd control
├── bridge.rs        # MCP-D-Bus bridge
├── discovery.rs     # Service discovery
├── introspection_parser.rs  # D-Bus introspection
├── json_introspection.rs    # JSON conversion
├── main.rs          # Main MCP server
├── orchestrator.rs  # Agent orchestration
└── web_main.rs      # Web interface
```

## Integration with op-dbus

The MCP functionality is fully integrated with the main op-dbus system:

- Available as optional `mcp` feature
- Shares D-Bus dependencies and infrastructure
- Can leverage op-dbus state management
- Compatible with existing plugins

## Development

### Adding New Agents

1. Create agent in `src/mcp/agents/`
2. Add binary entry to `Cargo.toml`
3. Register with orchestrator
4. Update discovery configuration

### Extending Discovery

The discovery system automatically finds new D-Bus services. To add custom categorization:

1. Edit `mcp-configs/discovery_config.toml`
2. Add service patterns to categories
3. Re-run discovery

## Troubleshooting

### Common Issues

1. **MCP server fails to start**
   - Check D-Bus session is available: `echo $DBUS_SESSION_BUS_ADDRESS`
   - Ensure built with `mcp` feature: `cargo build --features mcp`

2. **No services discovered**
   - Run with debug logging: `RUST_LOG=debug ./target/release/dbus-mcp-discovery`
   - Check D-Bus services: `busctl list`

3. **Bridge connection issues**
   - Verify orchestrator is running
   - Check systemd service status if using systemd integration

## License

Same as operation-dbus: MIT License