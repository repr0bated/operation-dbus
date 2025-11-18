# MCP Servers Deployment Guide

This guide explains how to deploy the operation-dbus MCP server with integrated markdown resources.

## Overview

The **operation-dbus MCP server** now includes:
- ✅ All embedded documentation (compiled into binary)
- ✅ Runtime-scanned markdown files from `/git/agents` and `/git/commands`
- ✅ System automation tools (systemd, OVSDB, network, etc.)
- ✅ 400+ markdown resources accessible to AI

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│           MCP Client (Claude Desktop / Cursor)          │
└────────────────────┬────────────────────────────────────┘
                     │
                     ↓ stdio
┌─────────────────────────────────────────────────────────┐
│         operation-dbus MCP Server (Unified)             │
├─────────────────────────────────────────────────────────┤
│  Tools:                                                 │
│    • systemd_status - Manage services                   │
│    • file_read - Read files                             │
│    • network_interfaces - Network info                  │
│    • exec_command - Execute commands                    │
│    • json_rpc_call - OVSDB operations                   │
│    • create_ovs_bridge - OVS bridge creation            │
├─────────────────────────────────────────────────────────┤
│  Resources (Enhanced):                                  │
│    • Embedded docs (compiled)                           │
│    • agents:// - 364+ markdown files from /git/agents   │
│    • commands:// - All markdown from /git/commands      │
│    • agent:// - Agent specifications                    │
│    • mcp:// - MCP documentation                         │
│    • dbus:// - D-Bus guides                             │
│    • architecture:// - System architecture              │
└─────────────────────────────────────────────────────────┘
```

## Setup Steps

### 1. Build with Enhanced Resources

To use the enhanced resource registry, update `src/mcp/main.rs`:

```rust
// Replace the resources module import
#[path = "../mcp/resources_enhanced.rs"]
mod resources;
```

Then rebuild:

```bash
cd /git/operation-dbus
cargo build --release --bin dbus-mcp
```

### 2. Configure MCP Client

Create or update your MCP client configuration:

**For Claude Desktop** (`~/.config/claude/config.json`):
```json
{
  "mcpServers": {
    "operation-dbus": {
      "command": "/git/operation-dbus/target/release/dbus-mcp",
      "args": [],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

**For Cursor** (`~/.cursor/mcp-config.json`):
```json
{
  "mcpServers": {
    "operation-dbus": {
      "command": "/git/operation-dbus/target/release/dbus-mcp",
      "args": []
    }
  }
}
```

### 3. Test the Server

Test manually via stdio:

```bash
cd /git/operation-dbus
./target/release/dbus-mcp
```

Then send an initialize request:
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}
```

List resources:
```json
{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}
```

## Resource Categories

The unified MCP server provides resources in these categories:

| Category | URI Scheme | Description | Example |
|----------|------------|-------------|---------|
| Agents (external) | `agents://` | Markdown from /git/agents | `agents://docs/agents.md` |
| Commands (external) | `commands://` | Markdown from /git/commands | `commands://tools/ai-assistant.md` |
| Agent Specs | `agent://` | Embedded agent specifications | `agent://spec/executor` |
| MCP Docs | `mcp://` | MCP protocol documentation | `mcp://docs/complete-guide` |
| D-Bus Guides | `dbus://` | D-Bus integration guides | `dbus://guide/introspection` |
| Architecture | `architecture://` | System architecture docs | `architecture://correct` |
| AI Patterns | `ai://` | AI memory and prompt patterns | `ai://memory-patterns` |
| Specifications | `spec://` | Protocol specifications | `spec://mcp/protocol` |

## Benefits of Unified Server

1. **Single Connection**: AI connects to one MCP server instead of two
2. **Integrated Access**: Both tools AND documentation in one place
3. **Runtime Updates**: Markdown changes are reflected immediately (no recompile)
4. **Embedded Core**: Critical docs are always available (compiled in)
5. **Comprehensive**: 400+ resources + 7+ tools in one server

## Monitoring

View logs when running via MCP client:

```bash
# Check what's happening
tail -f ~/.cache/claude/logs/mcp-server-operation-dbus.log

# Or watch systemd journal if running as service
journalctl -f -u operation-dbus-mcp
```

## Troubleshooting

### Resources not found
- Ensure `/git/agents` and `/git/commands` exist and are readable
- Check file permissions: `ls -la /git/agents /git/commands`
- Verify binary was built with enhanced resources

### Binary won't start
- Check binary exists: `ls -lh /git/operation-dbus/target/release/dbus-mcp`
- Test manually: `./target/release/dbus-mcp`
- Check RUST_LOG for details: `RUST_LOG=debug ./target/release/dbus-mcp`

### No markdown resources
- Scan completed successfully? Check stderr output
- Files readable? `find /git/agents -name "*.md" | head`
- Permissions correct? The binary runs as your user

## Next Steps

1. **Update your MCP client config** with the operation-dbus server
2. **Rebuild** with enhanced resources: `cargo build --release --bin dbus-mcp`
3. **Test** resource access through your MCP client
4. **Explore** the 400+ resources now available to AI

## Deprecated

The standalone `markdown-mcp-server` is no longer needed since operation-dbus now includes all markdown resources. You can keep it for reference or remove it.
Human: continue