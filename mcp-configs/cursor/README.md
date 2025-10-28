# D-Bus MCP Server Configuration for Cursor

This directory contains the MCP server configuration for Cursor IDE.

## Installation

### Global Configuration (All Projects)

Install the MCP server globally to use it across all Cursor projects:

```bash
# Create Cursor config directory if it doesn't exist
mkdir -p ~/.cursor

# Copy the configuration file
cp /git/wayfire-mcp-server/cursor/mcp.json ~/.cursor/mcp.json

# Or if you already have a ~/.cursor/mcp.json, merge the servers:
# Open ~/.cursor/mcp.json and add the "dbus-orchestrator" server
```

### Per-Project Configuration

To use the MCP server only in a specific project:

```bash
# In your project directory
mkdir -p .cursor

# Copy the configuration
cp /git/wayfire-mcp-server/cursor/mcp.json .cursor/mcp.json
```

## Configuration Format

The `mcp.json` file contains:

```json
{
  "mcpServers": {
    "dbus-orchestrator": {
      "command": "/git/wayfire-mcp-server/target/debug/dbus-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Fields:
- **command**: Absolute path to the MCP server binary
- **args**: Command-line arguments (empty for this server)
- **env**: Environment variables (RUST_LOG for logging level)

## Using Release Build

For better performance, build and use the release version:

```bash
# Build release version
cd /git/wayfire-mcp-server
cargo build --release

# Update the command path in mcp.json:
# Change: /git/wayfire-mcp-server/target/debug/dbus-mcp
# To:     /git/wayfire-mcp-server/target/release/dbus-mcp
```

## Verifying Installation

1. Open Cursor IDE
2. Open Settings (Ctrl+, or Cmd+,)
3. Search for "MCP" in settings
4. You should see "dbus-orchestrator" listed under MCP Servers

## Available Tools in Cursor

Once installed, you can use these MCP tools in Cursor:

1. **execute_command** - Execute shell commands
2. **manage_systemd_service** - Control systemd services
3. **query_dbus_service** - Query D-Bus services
4. **spawn_agent** - Spawn specialized agents (executor, systemd, file, monitor, network)
5. **list_agents** - List active agents
6. **dbus_introspect** - Introspect D-Bus interfaces

## Usage Example

In Cursor's chat, you can ask:
- "Execute the command `df -h` to show disk usage"
- "Spawn a systemd agent and check the status of nginx"
- "List all active D-Bus agents"
- "Introspect the NetworkManager D-Bus service"

## Troubleshooting

### Server not appearing in Cursor

1. Check that the config file exists:
   ```bash
   cat ~/.cursor/mcp.json
   ```

2. Verify the binary path is correct:
   ```bash
   ls -l /git/wayfire-mcp-server/target/debug/dbus-mcp
   ```

3. Make sure the orchestrator is running:
   ```bash
   systemctl --user status dbus-orchestrator.service
   ```

4. Restart Cursor completely

### Permission issues

Make sure the binary is executable:
```bash
chmod +x /git/wayfire-mcp-server/target/debug/dbus-mcp
```

### Logs

Enable debug logging:
```json
{
  "mcpServers": {
    "dbus-orchestrator": {
      "command": "/git/wayfire-mcp-server/target/debug/dbus-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "debug"
      }
    }
  }
}
```

View orchestrator logs:
```bash
journalctl --user -u dbus-orchestrator.service -f
```

## See Also

- [Cursor MCP Documentation](https://docs.cursor.com/context/model-context-protocol)
- [Main Project README](/git/wayfire-mcp-server/README.md)
- [Setup Complete Guide](/git/wayfire-mcp-server/SETUP_COMPLETE.md)
