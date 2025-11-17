# Model Context Protocol (MCP) Specification Reference

Condensed reference for the Model Context Protocol (2024-11-05 version).
Full specification: https://spec.modelcontextprotocol.io/

## Core Concepts

**MCP** = JSON-RPC 2.0 based protocol for AI <-> Tool communication
**Server** = Exposes tools, resources, and prompts to AI clients
**Client** = AI system that uses MCP servers (Claude, GPT, etc.)
**Transport** = stdio, HTTP, WebSocket

## Message Format

All messages use JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "method_name",
  "params": { }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { }
}
```

Error:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid Request"
  }
}
```

## Lifecycle Methods

### initialize
Client → Server handshake

**Request**:
```json
{
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "roots": {"listChanged": true},
      "sampling": {}
    },
    "clientInfo": {
      "name": "claude-desktop",
      "version": "1.0.0"
    }
  }
}
```

**Response**:
```json
{
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {"listChanged": true},
      "resources": {"subscribe": true, "listChanged": true},
      "prompts": {"listChanged": true}
    },
    "serverInfo": {
      "name": "my-mcp-server",
      "version": "1.0.0"
    }
  }
}
```

### initialized
Client → Server notification after initialization complete

```json
{
  "method": "notifications/initialized"
}
```

### ping
Keep-alive check

```json
{"method": "ping"}
→ {"result": {}}
```

## Tools

Tools are executable actions the AI can invoke.

### tools/list
Get available tools

**Request**:
```json
{"method": "tools/list"}
```

**Response**:
```json
{
  "result": {
    "tools": [
      {
        "name": "execute_command",
        "description": "Execute a shell command",
        "inputSchema": {
          "type": "object",
          "properties": {
            "command": {"type": "string"},
            "args": {"type": "array", "items": {"type": "string"}}
          },
          "required": ["command"]
        }
      }
    ]
  }
}
```

### tools/call
Execute a tool

**Request**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "execute_command",
    "arguments": {
      "command": "ls",
      "args": ["-la", "/tmp"]
    }
  }
}
```

**Response**:
```json
{
  "result": {
    "content": [
      {
        "type": "text",
        "text": "total 24\ndrwxrwxrwt 10 root root 4096 ...\n"
      }
    ]
  }
}
```

**Error Response**:
```json
{
  "result": {
    "isError": true,
    "content": [
      {
        "type": "text",
        "text": "Command failed: permission denied"
      }
    ]
  }
}
```

### Content Types

Tools can return multiple content types:

```typescript
type Content =
  | TextContent      // Plain text
  | ImageContent     // Base64 encoded images
  | ResourceContent  // Reference to a resource

interface TextContent {
  type: "text"
  text: string
}

interface ImageContent {
  type: "image"
  data: string        // base64
  mimeType: string
}

interface ResourceContent {
  type: "resource"
  resource: {
    uri: string
    mimeType?: string
    text?: string
  }
}
```

## Resources

Resources are read-only data sources (files, docs, etc.).

### resources/list
List available resources

**Response**:
```json
{
  "result": {
    "resources": [
      {
        "uri": "file:///etc/nginx/nginx.conf",
        "name": "Nginx Configuration",
        "description": "Main nginx config file",
        "mimeType": "text/plain"
      },
      {
        "uri": "dbus://service/org.freedesktop.NetworkManager",
        "name": "NetworkManager Service Info",
        "mimeType": "application/json"
      }
    ]
  }
}
```

### resources/read
Read resource content

**Request**:
```json
{
  "method": "resources/read",
  "params": {
    "uri": "file:///etc/nginx/nginx.conf"
  }
}
```

**Response**:
```json
{
  "result": {
    "contents": [
      {
        "uri": "file:///etc/nginx/nginx.conf",
        "mimeType": "text/plain",
        "text": "user nginx;\nworker_processes auto;\n..."
      }
    ]
  }
}
```

### resources/subscribe
Subscribe to resource changes (if supported)

```json
{
  "method": "resources/subscribe",
  "params": {
    "uri": "file:///var/log/system.log"
  }
}
```

Notifications:
```json
{
  "method": "notifications/resources/updated",
  "params": {
    "uri": "file:///var/log/system.log"
  }
}
```

## Prompts

Prompts are pre-defined conversation starters.

### prompts/list
List available prompts

**Response**:
```json
{
  "result": {
    "prompts": [
      {
        "name": "analyze_system",
        "description": "Analyze current system state",
        "arguments": [
          {
            "name": "focus",
            "description": "Area to focus on",
            "required": false
          }
        ]
      }
    ]
  }
}
```

### prompts/get
Get prompt content

**Request**:
```json
{
  "method": "prompts/get",
  "params": {
    "name": "analyze_system",
    "arguments": {
      "focus": "network"
    }
  }
}
```

**Response**:
```json
{
  "result": {
    "description": "System analysis prompt",
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Analyze the network configuration and identify any issues."
        }
      }
    ]
  }
}
```

## Sampling

Allows server to request LLM completions from client.

### sampling/createMessage
Request LLM completion

**Request** (Server → Client):
```json
{
  "method": "sampling/createMessage",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "What is the capital of France?"
        }
      }
    ],
    "modelPreferences": {
      "hints": [
        {"name": "claude-3-5-sonnet-20241022"}
      ],
      "costPriority": 0.5,
      "speedPriority": 0.5
    },
    "maxTokens": 1000
  }
}
```

**Response**:
```json
{
  "result": {
    "role": "assistant",
    "content": {
      "type": "text",
      "text": "The capital of France is Paris."
    },
    "model": "claude-3-5-sonnet-20241022",
    "stopReason": "endTurn"
  }
}
```

## Roots

Roots are filesystem/resource boundaries.

### roots/list
List root URIs

```json
{
  "result": {
    "roots": [
      {
        "uri": "file:///home/user/project",
        "name": "Project Directory"
      }
    ]
  }
}
```

## Logging

### logging/setLevel
Client can control server log verbosity

```json
{
  "method": "logging/setLevel",
  "params": {
    "level": "debug"  // "debug", "info", "warn", "error"
  }
}
```

### notifications/message
Server can send log messages to client

```json
{
  "method": "notifications/message",
  "params": {
    "level": "info",
    "logger": "dbus-indexer",
    "data": "Indexed 127 services in 2.3s"
  }
}
```

## Pagination

For large lists, use cursor-based pagination:

```json
{
  "method": "resources/list",
  "params": {
    "cursor": "page_2_token"
  }
}
```

Response:
```json
{
  "result": {
    "resources": [...],
    "nextCursor": "page_3_token"
  }
}
```

## Progress Tracking

Long-running operations can report progress:

```json
{
  "method": "notifications/progress",
  "params": {
    "progressToken": "index_build_123",
    "progress": 0.65,
    "total": 1.0
  }
}
```

## Cancellation

Client can cancel in-progress requests:

```json
{
  "method": "notifications/cancelled",
  "params": {
    "requestId": "req_123",
    "reason": "User cancelled operation"
  }
}
```

## Error Codes

Standard JSON-RPC 2.0 error codes:

- `-32700` Parse error
- `-32600` Invalid Request
- `-32601` Method not found
- `-32602` Invalid params
- `-32603` Internal error
- `-32000 to -32099` Server-defined errors

## Best Practices

1. **Validate input schemas**: Use JSON Schema for tool inputs
2. **Handle errors gracefully**: Return `isError: true` for tool failures
3. **Use appropriate content types**: Text for logs, images for diagrams
4. **Implement pagination**: For large resource lists
5. **Report progress**: For long-running operations
6. **Support cancellation**: Allow client to abort operations
7. **Version your protocol**: Include protocolVersion in initialize
8. **Document your tools**: Clear descriptions and examples

## Security Considerations

- **Validate all inputs**: Never trust client-provided parameters
- **Sandbox execution**: Isolate tool execution from server process
- **Limit resource access**: Only expose necessary files/data
- **Audit tool calls**: Log all tool invocations for security review
- **Rate limit**: Prevent abuse of expensive operations
- **Authenticate clients**: Verify client identity if needed

## Transport

### stdio
Standard input/output (recommended for local tools)

```bash
./mcp-server | client
```

### HTTP
RESTful HTTP transport

```
POST /message HTTP/1.1
Content-Type: application/json

{"jsonrpc": "2.0", ...}
```

### WebSocket
Bidirectional streaming

```javascript
const ws = new WebSocket('ws://localhost:8080/mcp');
ws.send(JSON.stringify({jsonrpc: "2.0", ...}));
```

## Example Server Implementation

```typescript
class McpServer {
  async handleRequest(request: JsonRpcRequest): Promise<JsonRpcResponse> {
    switch (request.method) {
      case "initialize":
        return this.handleInitialize(request);
      case "tools/list":
        return this.handleToolsList();
      case "tools/call":
        return this.handleToolsCall(request.params);
      case "resources/list":
        return this.handleResourcesList();
      case "resources/read":
        return this.handleResourcesRead(request.params);
      default:
        return {
          jsonrpc: "2.0",
          id: request.id,
          error: {code: -32601, message: "Method not found"}
        };
    }
  }
}
```

## References

- MCP Specification: https://spec.modelcontextprotocol.io/
- JSON-RPC 2.0: https://www.jsonrpc.org/specification
- JSON Schema: https://json-schema.org/
