# MCP Architecture Analysis: Two Servers + Chatbot Control

## Overview

There are **2 separate MCP servers** with different functions, both controlled by the chatbot:

1. **`dbus-mcp`** - Standard MCP server (JSON-RPC via stdin/stdout)
2. **`mcp-chat`** - Chat server (WebSocket/HTTP with natural language)

Both use **ToolRegistry** but are **independent instances** - they don't communicate with each other directly.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Chatbot (Ollama AI)                       │
│              Controls everything via natural language        │
└───────────────┬───────────────────────┬─────────────────────┘
                │                       │
                │                       │
    ┌───────────▼──────────┐  ┌────────▼──────────────┐
    │   mcp-chat Server    │  │   dbus-mcp Server     │
    │  (WebSocket/HTTP)    │  │  (JSON-RPC stdio)    │
    │                      │  │                      │
    │  • Natural Language  │  │  • Standard MCP     │
    │  • WebSocket         │  │  • stdin/stdout      │
    │  • HTTP REST API     │  │  • External clients  │
    │                      │  │  • Cursor/Claude     │
    └───────────┬──────────┘  └────────┬─────────────┘
                │                       │
                │                       │
    ┌───────────▼──────────┐  ┌────────▼──────────────┐
    │  ToolRegistry (A)    │  │  ToolRegistry (B)     │
    │  (Chat Server)       │  │  (MCP Server)         │
    │                      │  │                      │
    │  • Same tools        │  │  • Same tools        │
    │  • Independent       │  │  • Independent       │
    │  • Direct execution  │  │  • Via MCP protocol  │
    └───────────┬──────────┘  └────────┬─────────────┘
                │                       │
                └───────────┬───────────┘
                            │
                ┌───────────▼──────────┐
                │   System Operations   │
                │  (D-Bus, OVSDB, etc.)│
                └──────────────────────┘
```

## Server 1: `dbus-mcp` (Standard MCP Server)

**File**: `src/mcp/main.rs`  
**Binary**: `dbus-mcp`  
**Protocol**: JSON-RPC 2.0 via stdin/stdout

### Function
- Standard MCP protocol implementation
- Used by external MCP clients (Cursor, Claude Desktop, etc.)
- Provides tools/resources via MCP protocol

### Communication
- **Input**: JSON-RPC requests from stdin
- **Output**: JSON-RPC responses to stdout
- **Protocol**: Standard MCP (tools/list, tools/call, resources/list, etc.)

### Key Features
```rust
struct McpServer {
    registry: Arc<ToolRegistry>,      // Tool registry instance A
    resources: Arc<ResourceRegistry>,  // Embedded documentation
    orchestrator: Option<zbus::Proxy>, // D-Bus orchestrator connection
}
```

### Methods Handled
- `tools/list` - List all available tools
- `tools/call` - Execute a tool
- `resources/list` - List embedded resources
- `resources/read` - Read resource content

## Server 2: `mcp-chat` (Chat Server)

**File**: `src/mcp/chat_main.rs` + `src/mcp/chat_server.rs`  
**Binary**: `mcp-chat`  
**Protocol**: WebSocket + HTTP REST

### Function
- Natural language chat interface
- WebSocket for real-time chat
- HTTP REST API for suggestions/history
- Integrates with Ollama chatbot

### Communication
- **Input**: Natural language from WebSocket/HTTP
- **Output**: Chat messages (User/Assistant/System/Error)
- **Protocol**: Custom chat protocol over WebSocket

### Key Features
```rust
struct ChatServerState {
    tool_registry: Arc<ToolRegistry>,      // Tool registry instance B
    agent_registry: Arc<AgentRegistry>,    // Agent management
    conversations: Arc<RwLock<HashMap>>,   // Conversation history
    ollama_client: Option<Arc<OllamaClient>>, // Chatbot integration
}
```

### Natural Language Processing
- Parses commands: "run discover_system", "start agent executor"
- Intent detection: ExecuteTool, ManageAgent, QueryStatus, AIChat
- Tool parameter extraction from natural language

### Chatbot Integration
- Uses Ollama client for AI responses
- Builds system context for chatbot
- Suggests tools based on AI responses
- Executes tools when chatbot recommends them

## How Chatbot Controls Everything

### Flow 1: Direct Tool Execution (Chat Server)

```
User: "discover hardware"
  ↓
Chat Server: parse_command() → ExecuteTool { tool_name: "discover_system" }
  ↓
Chat Server: execute_tool() → tool_registry.execute_tool()
  ↓
ToolRegistry: Execute tool handler
  ↓
System: Perform introspection
  ↓
Chat Server: Return result as ChatMessage
  ↓
User: See result in chat
```

### Flow 2: AI-Powered Tool Execution (Chatbot)

```
User: "what is my CPU capable of?"
  ↓
Chat Server: parse_command() → AIChat { message: "..." }
  ↓
Chat Server: handle_ai_chat()
  ↓
Chatbot (Ollama): 
  - Receives system context
  - Receives available tools list
  - Generates response suggesting tools
  ↓
Chat Server: extract_tool_suggestions() from AI response
  ↓
Chat Server: If tool mentioned, execute it
  ↓
User: See AI response + tool results
```

### Flow 3: Standard MCP Client (External)

```
Cursor/Claude Desktop:
  ↓
dbus-mcp: Receive JSON-RPC request
  ↓
dbus-mcp: tools/call { name: "discover_system", arguments: {} }
  ↓
dbus-mcp: tool_registry.execute_tool()
  ↓
System: Perform introspection
  ↓
dbus-mcp: Return JSON-RPC response
  ↓
Cursor: Display result
```

## Key Differences

| Aspect | `dbus-mcp` | `mcp-chat` |
|--------|-----------|------------|
| **Protocol** | JSON-RPC 2.0 (stdio) | WebSocket + HTTP |
| **Interface** | Standard MCP | Natural language |
| **Clients** | External MCP clients | Web browser |
| **Tool Execution** | Via MCP protocol | Direct ToolRegistry call |
| **AI Integration** | None | Full Ollama integration |
| **Conversation** | Stateless | Stateful (conversation history) |
| **Natural Language** | No | Yes (command parsing) |

## Shared Components

Both servers use:
- **ToolRegistry** - Same tool definitions (but separate instances)
- **AgentRegistry** - Agent management (chat server only)
- **System Operations** - D-Bus, OVSDB, etc.

## Current State Analysis

### ✅ What's Working
- Both servers have ToolRegistry
- Chat server has natural language processing
- Chat server integrates with Ollama chatbot
- Both can execute tools independently

### ⚠️ Potential Issues
1. **Tool Registry Duplication**: Two separate instances mean:
   - Tools registered in one don't appear in the other
   - Tool updates must be done in both places
   - Inconsistent tool availability

2. **No Communication Between Servers**: 
   - Chat server can't use MCP server
   - MCP server can't use chat features
   - No shared state

3. **Chatbot Data Access**:
   - Chatbot gets system context via `build_system_context()`
   - Gets tools list via `build_tools_description()`
   - But may not have granular D-Bus introspection data

## Recommendations for Chatbot Data Improvements

### Before Modularization ✅

**Why now:**
1. Both servers already exist and work
2. Chatbot integration is in place
3. Tool registry architecture is clear
4. Can improve data presentation without breaking architecture

**What to improve:**
1. **Unified Introspection API** - Single source of truth for D-Bus data
2. **Granular MCP Tools** - Add tools for specific D-Bus services/methods
3. **Better Data Presentation** - Format introspection data for chatbot
4. **Tool Registry Sync** - Ensure both servers have same tools

### After Integration
- Consider shared ToolRegistry instance
- Add communication channel between servers
- Unified introspection cache

## Next Steps

1. ✅ Document architecture (this file)
2. ⏳ Improve introspection data presentation for chatbot
3. ⏳ Add granular D-Bus tools to both servers
4. ⏳ Ensure tool registry consistency
5. ⏳ Test chatbot with improved data access
