# MCP Servers Architecture

## Overview

There are **TWO MCP servers** that both serve different clients:

1. **`dbus-mcp`** (`src/mcp/main.rs`) - Rust MCP server
   - Serves: **Cursor, Claude Desktop, and other MCP clients**
   - Protocol: JSON-RPC 2.0 via stdin/stdout
   - Also serves: **Chatbot MCP** (the chatbot connects to this server)

2. **`mcp-chat`** (`src/mcp/chat_main.rs` + `chat_server.rs`) - Chat server
   - Serves: **Web browser** (WebSocket/HTTP)
   - Protocol: WebSocket + HTTP REST
   - Uses: **ToolRegistry** to execute tools (same as dbus-mcp)

## Architecture Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    External Clients                          │
│  (Cursor, Claude Desktop, other MCP clients)                 │
└───────────────┬─────────────────────────────────────────────┘
                │
                │ JSON-RPC (stdio)
                │
    ┌───────────▼──────────┐
    │   dbus-mcp Server    │  ← Rust MCP server
    │  (src/mcp/main.rs)   │
    │                      │
    │  • ToolRegistry      │
    │  • ResourceRegistry  │
    │  • Orchestrator Proxy│
    └───────────┬──────────┘
                │
                │ Also serves Chatbot MCP
                │
    ┌───────────▼──────────┐
    │   Chatbot (Ollama)   │
    │   Connects to        │
    │   dbus-mcp via MCP   │
    └──────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    Web Browser                               │
└───────────────┬─────────────────────────────────────────────┘
                │
                │ WebSocket/HTTP
                │
    ┌───────────▼──────────┐
    │   mcp-chat Server    │  ← Chat server
    │  (chat_main.rs)      │
    │                      │
    │  • ToolRegistry      │  (separate instance)
    │  • AgentRegistry     │
    │  • OllamaClient      │
    │  • Natural Language  │
    └───────────┬──────────┘
                │
                │ Executes tools via ToolRegistry
                │
    ┌───────────▼──────────┐
    │   System Operations  │
    │  (D-Bus, OVSDB, etc.)│
    └──────────────────────┘
```

## Orchestrator Location

**Orchestrator** (`src/mcp/orchestrator.rs`):
- **Binary**: `dbus-orchestrator` (runs as separate process)
- **D-Bus Service**: `org.dbusmcp.Orchestrator`
- **D-Bus Path**: `/org/dbusmcp/Orchestrator`
- **Purpose**: Manages agent lifecycle (spawn, stop, monitor)

**How MCP servers connect**:
- Both `dbus-mcp` and `mcp-chat` try to connect to orchestrator via D-Bus
- They use `zbus::Proxy` to call orchestrator methods
- Orchestrator runs independently as a D-Bus service

## Python Server (TO DELETE)

**File**: `gpu-automation.py`

**Status**: Needs to be deleted - keeps interfering

**Action**: Remove this file and any references to it.

## Agents and Tools Count

### Embedded Agents (Rust MCP)
**Location**: `src/mcp/agents/`

**Count**: 14 agent files
- executor.rs
- systemd.rs
- file.rs
- network.rs
- packagekit.rs
- monitor.rs
- rust_pro.rs
- python_pro.rs
- c_pro.rs
- cpp_pro.rs
- golang_pro.rs
- javascript_pro.rs
- php_pro.rs
- sql_pro.rs

**Note**: User mentioned "over 100 agents" - this might include:
- Multiple instances of same agent type
- Agents registered dynamically
- Agents from config files (`/etc/op-dbus/agents`)

### Tools (Rust MCP)
**Location**: `src/mcp/tools/`

**Count**: 3 tool files
- agents.rs (agent tools - NEW)
- dbus_granular.rs (granular D-Bus tools - NEW)
- introspection.rs (introspection tools)

**Registration Points**:
- `src/mcp/main.rs` - `register_default_tools()` - registers basic tools
- `src/mcp/introspection_tools.rs` - `register_introspection_tools()` - registers introspection tools
- `src/mcp/tools/agents.rs` - `register_agent_tools()` - registers agent tools (NEW)
- `src/mcp/tools/dbus_granular.rs` - `register_dbus_granular_tools()` - registers granular D-Bus tools (NEW)

## Current Registration Status

### dbus-mcp Server (`src/mcp/main.rs`)
**Currently registers**:
- ✅ Basic tools (systemd_status, file_read, etc.)
- ❌ Agent tools (NOT registered yet)
- ❌ Granular D-Bus tools (NOT registered yet)
- ❌ Introspection tools (NOT registered yet)

**Needs to register**:
```rust
// In register_default_tools()
crate::mcp::tools::agents::register_agent_tools(&registry).await?;
crate::mcp::tools::dbus_granular::register_dbus_granular_tools(&registry).await?;
crate::mcp::introspection_tools::register_introspection_tools(&registry).await?;
```

### mcp-chat Server (`src/mcp/chat_main.rs`)
**Currently registers**:
- ✅ Introspection tools (via `register_introspection_tools()`)
- ❌ Agent tools (NOT registered yet)
- ❌ Granular D-Bus tools (NOT registered yet)

**Needs to register**:
```rust
// After creating tool_registry
crate::mcp::tools::agents::register_agent_tools(&tool_registry).await?;
crate::mcp::tools::dbus_granular::register_dbus_granular_tools(&tool_registry).await?;
```

## Tool Registry Architecture

**Single ToolRegistry per server**:
- `dbus-mcp` has its own `ToolRegistry` instance
- `mcp-chat` has its own `ToolRegistry` instance
- They don't share state (independent)

**Tool Registration**:
- Tools are registered at server startup
- Dynamic registration possible via `ToolRegistry::register_tool()`
- Tools can be discovered via `ToolRegistry::list_tools()`

## Agent Registry Architecture

**Single AgentRegistry**:
- Managed by **Orchestrator** (D-Bus service)
- Both MCP servers connect to orchestrator via D-Bus proxy
- Agents are spawned/managed by orchestrator

**Agent Types**:
- Embedded agents (14 types in `src/mcp/agents/`)
- Custom agents (loaded from `/etc/op-dbus/agents`)
- Dynamic agents (registered at runtime)

## Next Steps

1. ✅ **Delete Python server** (`gpu-automation.py`)
2. ⏳ **Register all tools** in both MCP servers
3. ⏳ **Verify orchestrator** is running and accessible
4. ⏳ **Test chatbot** can access all tools via dbus-mcp
5. ⏳ **Count actual agents** (including dynamic registrations)

