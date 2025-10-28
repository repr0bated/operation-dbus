# D-Bus MCP Server - Agent Configurations

This directory contains MCP server configurations for the D-Bus MCP agents.

## Available Agents

### 1. **dbus-orchestrator.json** (Main Server)
The primary MCP server that provides tools to:
- Spawn D-Bus agents dynamically
- Execute commands (foreground or background)
- Manage systemd services
- Query D-Bus services
- Introspect D-Bus interfaces

**Usage:**
```bash
cp dbus-orchestrator.json ~/.config/Claude/mcp-servers/
```

### 2. **dbus-executor.json** (Direct Executor)
Direct access to a single executor agent for running shell commands via D-Bus.

**Usage:**
```bash
cp dbus-executor.json ~/.config/Claude/mcp-servers/
```

## Planned Agent Types

The orchestrator supports these agent types (not all built yet):
- ✅ **executor** - Execute shell commands
- ⏳ **systemd** - Manage systemd services
- ⏳ **file** - File operations
- ⏳ **monitor** - System monitoring
- ⏳ **network** - Network operations

## Installation

### Install Main Orchestrator (Recommended)
```bash
# Copy to Claude Code MCP servers directory
cp agents/dbus-orchestrator.json ~/.config/Claude/mcp-servers/

# Restart Claude Code to load the new server
```

### Build Requirements

Make sure the binaries are built first:
```bash
cd /git/wayfire-mcp-server
cargo build --release

# The binaries will be in:
# - target/release/dbus-mcp (main MCP server)
# - target/release/dbus-agent-executor (executor agent)
# - target/release/dbus-orchestrator (D-Bus orchestrator)
```

### Update Config Paths

If using release builds, update the JSON configs to use `target/release/` instead of `target/debug/`.

## Architecture

```
┌─────────────────────────────────────────┐
│         Claude Code / Claude Desktop     │
└────────────────┬────────────────────────┘
                 │ stdio (MCP protocol)
                 │
┌────────────────▼────────────────────────┐
│      wayfire-mcp-server                  │
│  (Main MCP Server + D-Bus Proxy)         │
└────────────────┬────────────────────────┘
                 │ D-Bus session bus
                 │
┌────────────────▼────────────────────────┐
│      wayfire-orchestrator                │
│  (D-Bus service: org.dbusmcp.Orchestrator)│
└────────────────┬────────────────────────┘
                 │ Spawns agents
                 │
     ┌───────────┼───────────┐
     ▼           ▼           ▼
┌─────────┐ ┌─────────┐ ┌─────────┐
│executor │ │systemd  │ │  file   │
│ agent   │ │ agent   │ │ agent   │
└─────────┘ └─────────┘ └─────────┘
```

## Testing

Test the orchestrator is working:
```bash
# The orchestrator should be running via systemd
systemctl --user status dbus-orchestrator.service

# Test the MCP server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | cargo run --bin dbus-mcp
```

## See Also

- `/git/wayfire-mcp-server/README.md` - Main project documentation
- `/git/wayfire-mcp-server/ARCHITECTURE.md` - System architecture
- `/git/wayfire-mcp-server/src/agents/` - Agent source code
