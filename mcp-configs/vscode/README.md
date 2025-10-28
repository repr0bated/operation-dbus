# D-Bus MCP Server Configuration for VS Code

This directory contains the MCP server configuration for Visual Studio Code.

## Prerequisites

**Important**: You need VS Code with Copilot/GitHub Copilot Chat extension that supports MCP servers. This is available in VS Code Insiders or recent stable versions with the feature flag enabled.

Check if MCP is available:
1. Open VS Code Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Search for "MCP" - if you see commands like "MCP: Add Server", you have MCP support

## Installation

### Method 1: User Configuration (Recommended - All Projects)

Install globally for all your VS Code projects:

1. Open VS Code Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Run: `MCP: Open User Configuration`
3. This opens your user-level `mcp.json`
4. Copy the contents from `/git/wayfire-mcp-server/vscode/mcp.json`
5. Save and restart VS Code

### Method 2: Workspace Configuration (Per-Project)

Install only for a specific project/workspace:

```bash
# In your project directory
mkdir -p .vscode

# Copy the configuration
cp /git/wayfire-mcp-server/vscode/mcp.json .vscode/mcp.json
```

### Method 3: Add Server via Command Palette

1. Open VS Code Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Run: `MCP: Add Server`
3. Choose "Global" (all projects) or "Workspace" (current project)
4. Enter server name: `dbus-orchestrator`
5. Enter command: `/git/wayfire-mcp-server/target/debug/dbus-mcp`
6. Leave args empty (press Enter)
7. Add environment variable: `RUST_LOG=info`

## Configuration Format

The `mcp.json` file for VS Code uses this format:

```json
{
  "servers": {
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

**Note**: VS Code uses `"servers"` (not `"mcpServers"` like Cursor)

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

## Auto-Start Setting (Experimental)

Enable automatic MCP server restart on configuration changes:

1. Open VS Code Settings (Ctrl+, / Cmd+,)
2. Search for: `chat.mcp.autostart`
3. Enable it

## Verifying Installation

1. Open VS Code Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Run: `MCP: List Servers`
3. You should see "dbus-orchestrator" listed

Or check the GitHub Copilot Chat panel:
1. Open Copilot Chat (Ctrl+Alt+I / Cmd+Option+I)
2. Look for MCP tools in the tool picker

## Available Tools in VS Code

Once installed, you can use these MCP tools in Copilot Chat:

1. **execute_command** - Execute shell commands
2. **manage_systemd_service** - Control systemd services
3. **query_dbus_service** - Query D-Bus services
4. **spawn_agent** - Spawn specialized agents (executor, systemd, file, monitor, network)
5. **list_agents** - List active agents
6. **dbus_introspect** - Introspect D-Bus interfaces

## Usage Example

In VS Code Copilot Chat, you can ask:
- "Use the execute_command tool to show disk usage with `df -h`"
- "Spawn a systemd agent and check nginx status"
- "List all active D-Bus agents"
- "Introspect the NetworkManager D-Bus service"

## Settings Sync

If you have VS Code Settings Sync enabled, your MCP server configurations will sync across all your devices automatically.

## Troubleshooting

### MCP Commands Not Available

Make sure you have:
- VS Code version with MCP support (Insiders or recent stable)
- GitHub Copilot or Copilot Chat extension installed
- MCP feature enabled (may require feature flag in some versions)

### Server Not Appearing

1. Check that the config file exists:
   ```bash
   # For user config
   code ~/.vscode/mcp.json

   # For workspace config
   cat .vscode/mcp.json
   ```

2. Verify the binary path is correct:
   ```bash
   ls -l /git/wayfire-mcp-server/target/debug/dbus-mcp
   ```

3. Make sure the orchestrator is running:
   ```bash
   systemctl --user status dbus-orchestrator.service
   ```

4. Restart VS Code completely:
   - Close all VS Code windows
   - Reopen VS Code
   - Run: `MCP: List Servers` in Command Palette

### Permission Issues

Make sure the binary is executable:
```bash
chmod +x /git/wayfire-mcp-server/target/debug/dbus-mcp
```

### Debugging

1. Enable debug logging in `mcp.json`:
   ```json
   {
     "servers": {
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

2. View orchestrator logs:
   ```bash
   journalctl --user -u dbus-orchestrator.service -f
   ```

3. Check VS Code output:
   - View → Output
   - Select "GitHub Copilot Chat" from the dropdown

## File Locations

### User Configuration
- Linux: `~/.config/Code/User/mcp.json`
- macOS: `~/Library/Application Support/Code/User/mcp.json`
- Windows: `%APPDATA%\Code\User\mcp.json`

### Workspace Configuration
- `.vscode/mcp.json` in your project root

## See Also

- [VS Code MCP Documentation](https://code.visualstudio.com/docs/copilot/chat/mcp-servers)
- [Main Project README](/git/wayfire-mcp-server/README.md)
- [Setup Complete Guide](/git/wayfire-mcp-server/SETUP_COMPLETE.md)
