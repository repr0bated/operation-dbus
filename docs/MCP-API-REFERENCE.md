# MCP API Reference

## Complete API Documentation

### Table of Contents
1. [JSON-RPC Protocol](#json-rpc-protocol)
2. [Tool Definitions](#tool-definitions)
3. [Agent APIs](#agent-apis)
4. [Error Codes](#error-codes)
5. [Examples](#examples)

---

## JSON-RPC Protocol

### Base Protocol Structure

All MCP communication uses JSON-RPC 2.0 format:

```typescript
interface Request {
  jsonrpc: "2.0";
  id?: string | number | null;
  method: string;
  params?: any;
}

interface Response {
  jsonrpc: "2.0";
  id?: string | number | null;
  result?: any;
  error?: ErrorObject;
}

interface ErrorObject {
  code: number;
  message: string;
  data?: any;
}
```

### Core Methods

#### initialize

Establishes MCP session and capabilities negotiation.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {
        "call": true
      }
    },
    "clientInfo": {
      "name": "claude-desktop",
      "version": "1.0.0"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {
        "list": true,
        "call": true
      }
    },
    "serverInfo": {
      "name": "dbus-mcp",
      "version": "0.1.0",
      "description": "D-Bus MCP Bridge Server"
    }
  }
}
```

#### tools/list

Returns available tools with schemas.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "systemd_status",
        "description": "Get the status of a systemd service",
        "inputSchema": {
          "type": "object",
          "properties": {
            "service": {
              "type": "string",
              "description": "Name of the systemd service"
            }
          },
          "required": ["service"]
        }
      },
      {
        "name": "file_read",
        "description": "Read contents of a file",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {
              "type": "string",
              "description": "Absolute path to the file"
            },
            "encoding": {
              "type": "string",
              "description": "File encoding (default: utf-8)",
              "enum": ["utf-8", "ascii", "base64"],
              "default": "utf-8"
            }
          },
          "required": ["path"]
        }
      }
    ]
  }
}
```

#### tools/call

Invokes a specific tool with arguments.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "systemd_status",
    "arguments": {
      "service": "nginx.service"
    }
  }
}
```

**Response (Success):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "● nginx.service - A high performance web server\n   Loaded: loaded (/usr/lib/systemd/system/nginx.service; enabled)\n   Active: active (running) since Mon 2024-01-15 10:23:45 UTC; 2h ago\n Main PID: 1234 (nginx)\n   Memory: 12.5M\n   CGroup: /system.slice/nginx.service\n           ├─1234 nginx: master process\n           └─1235 nginx: worker process"
      }
    ]
  }
}
```

**Response (Error):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "error": {
    "code": -32602,
    "message": "Invalid params",
    "data": {
      "details": "Service 'nginx.service' not found"
    }
  }
}
```

---

## Tool Definitions

### System Management Tools

#### systemd_status
Get status of a systemd service.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "service": {
      "type": "string",
      "description": "Service name (e.g., 'nginx', 'nginx.service')"
    }
  },
  "required": ["service"]
}
```

**Output:**
```json
{
  "type": "object",
  "properties": {
    "status": {
      "type": "string",
      "enum": ["active", "inactive", "failed", "activating", "deactivating"]
    },
    "description": {
      "type": "string"
    },
    "loaded": {
      "type": "boolean"
    },
    "pid": {
      "type": "integer"
    },
    "memory": {
      "type": "string"
    }
  }
}
```

#### systemd_start
Start a systemd service.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "service": {
      "type": "string"
    },
    "no_block": {
      "type": "boolean",
      "default": false,
      "description": "Don't wait for operation to complete"
    }
  },
  "required": ["service"]
}
```

#### systemd_stop
Stop a systemd service.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "service": {
      "type": "string"
    },
    "mode": {
      "type": "string",
      "enum": ["replace", "fail", "ignore-dependencies"],
      "default": "replace"
    }
  },
  "required": ["service"]
}
```

#### systemd_restart
Restart a systemd service.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "service": {
      "type": "string"
    }
  },
  "required": ["service"]
}
```

#### systemd_logs
Get logs for a systemd service.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "service": {
      "type": "string"
    },
    "lines": {
      "type": "integer",
      "default": 50,
      "description": "Number of log lines to retrieve"
    },
    "since": {
      "type": "string",
      "description": "Show logs since timestamp (e.g., '2024-01-15 10:00:00')"
    }
  },
  "required": ["service"]
}
```

### File Operations Tools

#### file_read
Read file contents.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "Absolute file path"
    },
    "encoding": {
      "type": "string",
      "enum": ["utf-8", "ascii", "base64"],
      "default": "utf-8"
    }
  },
  "required": ["path"]
}
```

#### file_write
Write content to a file.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string"
    },
    "content": {
      "type": "string"
    },
    "encoding": {
      "type": "string",
      "enum": ["utf-8", "ascii", "base64"],
      "default": "utf-8"
    },
    "mode": {
      "type": "string",
      "enum": ["overwrite", "append", "create_new"],
      "default": "overwrite"
    }
  },
  "required": ["path", "content"]
}
```

#### file_list
List directory contents.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "default": "."
    },
    "recursive": {
      "type": "boolean",
      "default": false
    },
    "include_hidden": {
      "type": "boolean",
      "default": false
    },
    "pattern": {
      "type": "string",
      "description": "Glob pattern for filtering"
    }
  }
}
```

#### file_delete
Delete a file or directory.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string"
    },
    "recursive": {
      "type": "boolean",
      "default": false,
      "description": "Required for directory deletion"
    }
  },
  "required": ["path"]
}
```

### Network Tools

#### network_interfaces
List network interfaces.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "include_loopback": {
      "type": "boolean",
      "default": false
    },
    "include_virtual": {
      "type": "boolean",
      "default": true
    }
  }
}
```

**Output Example:**
```json
{
  "interfaces": [
    {
      "name": "eth0",
      "type": "ethernet",
      "state": "up",
      "addresses": [
        {
          "address": "192.168.1.100",
          "prefix": 24,
          "family": "inet"
        }
      ],
      "mac": "00:11:22:33:44:55",
      "mtu": 1500
    }
  ]
}
```

#### network_connections
List NetworkManager connections.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "active_only": {
      "type": "boolean",
      "default": false
    }
  }
}
```

#### network_connect
Activate a network connection.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "connection": {
      "type": "string",
      "description": "Connection UUID or name"
    },
    "device": {
      "type": "string",
      "description": "Network device (optional)"
    }
  },
  "required": ["connection"]
}
```

### Process Management Tools

#### process_list
List running processes.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "user": {
      "type": "string",
      "description": "Filter by user"
    },
    "name": {
      "type": "string",
      "description": "Filter by process name"
    },
    "sort_by": {
      "type": "string",
      "enum": ["cpu", "memory", "pid", "name"],
      "default": "cpu"
    },
    "limit": {
      "type": "integer",
      "default": 50
    }
  }
}
```

#### process_kill
Terminate a process.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "pid": {
      "type": "integer"
    },
    "signal": {
      "type": "string",
      "enum": ["TERM", "KILL", "HUP", "INT"],
      "default": "TERM"
    }
  },
  "required": ["pid"]
}
```

### Command Execution Tools

#### exec_command
Execute a shell command.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Command to execute"
    },
    "args": {
      "type": "array",
      "items": {
        "type": "string"
      },
      "description": "Command arguments"
    },
    "cwd": {
      "type": "string",
      "description": "Working directory"
    },
    "env": {
      "type": "object",
      "description": "Environment variables"
    },
    "timeout": {
      "type": "integer",
      "description": "Timeout in seconds",
      "default": 30
    },
    "shell": {
      "type": "boolean",
      "description": "Execute through shell",
      "default": false
    }
  },
  "required": ["command"]
}
```

**Security Note:** Commands are filtered through allowlist in `dbus-executor.json`.

---

## Agent APIs

### Orchestrator D-Bus Interface

**Service:** `org.dbusmcp.Orchestrator`  
**Path:** `/org/dbusmcp/Orchestrator`

#### Methods

##### SpawnAgent
Create a new agent instance.

```xml
<method name="SpawnAgent">
  <arg name="agent_type" type="s" direction="in"/>
  <arg name="config_json" type="s" direction="in"/>
  <arg name="agent_id" type="s" direction="out"/>
</method>
```

**D-Bus Call Example:**
```bash
busctl call org.dbusmcp.Orchestrator \
  /org/dbusmcp/Orchestrator \
  org.dbusmcp.Orchestrator \
  SpawnAgent ss "systemd" "{}"
```

##### SendTask
Send task to agent.

```xml
<method name="SendTask">
  <arg name="agent_id" type="s" direction="in"/>
  <arg name="task_json" type="s" direction="in"/>
  <arg name="result_json" type="s" direction="out"/>
</method>
```

##### ListAgents
Get list of active agents.

```xml
<method name="ListAgents">
  <arg name="agents" type="as" direction="out"/>
</method>
```

##### GetAgentStatus
Get agent status information.

```xml
<method name="GetAgentStatus">
  <arg name="agent_id" type="s" direction="in"/>
  <arg name="status_json" type="s" direction="out"/>
</method>
```

#### Signals

##### AgentSpawned
Emitted when new agent is created.

```xml
<signal name="AgentSpawned">
  <arg name="agent_id" type="s"/>
  <arg name="agent_type" type="s"/>
  <arg name="timestamp" type="x"/>
</signal>
```

##### AgentDied
Emitted when agent terminates.

```xml
<signal name="AgentDied">
  <arg name="agent_id" type="s"/>
  <arg name="exit_code" type="i"/>
  <arg name="reason" type="s"/>
</signal>
```

### Agent Base Interface

All agents implement this base interface.

**Interface:** `org.dbusmcp.Agent`  
**Path:** `/org/dbusmcp/Agent/<type>/<id>`

#### Methods

##### ExecuteTask
Execute a task.

```xml
<method name="ExecuteTask">
  <arg name="task_json" type="s" direction="in"/>
  <arg name="result_json" type="s" direction="out"/>
</method>
```

**Task JSON Format:**
```json
{
  "id": "task-123",
  "type": "command",
  "params": {
    "command": "ls",
    "args": ["-la"]
  },
  "timeout": 30
}
```

**Result JSON Format:**
```json
{
  "id": "task-123",
  "status": "success",
  "output": "...",
  "error": null,
  "execution_time": 1.23
}
```

##### GetCapabilities
Get agent capabilities.

```xml
<method name="GetCapabilities">
  <arg name="capabilities_json" type="s" direction="out"/>
</method>
```

##### Shutdown
Gracefully shutdown agent.

```xml
<method name="Shutdown">
  <arg name="force" type="b" direction="in"/>
  <arg name="success" type="b" direction="out"/>
</method>
```

---

## Error Codes

### Standard JSON-RPC Errors

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Not a valid Request object |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal JSON-RPC error |

### MCP-Specific Errors

| Code | Message | Description |
|------|---------|-------------|
| -32001 | Not initialized | Client must call initialize first |
| -32002 | Already initialized | Initialize already called |
| -32003 | Unsupported protocol | Protocol version not supported |
| -32004 | Tool not found | Requested tool doesn't exist |
| -32005 | Tool execution failed | Tool threw an error |

### Agent Errors

| Code | Message | Description |
|------|---------|-------------|
| -32100 | Agent spawn failed | Could not create agent |
| -32101 | Agent not found | Agent ID doesn't exist |
| -32102 | Agent busy | Agent is processing another task |
| -32103 | Agent timeout | Agent didn't respond in time |
| -32104 | Agent crashed | Agent process terminated |

### D-Bus Errors

| Code | Message | Description |
|------|---------|-------------|
| -32200 | D-Bus connection failed | Cannot connect to D-Bus |
| -32201 | Service not found | D-Bus service doesn't exist |
| -32202 | Method call failed | D-Bus method invocation error |
| -32203 | Access denied | D-Bus security policy violation |

---

## Examples

### Complete Session Example

```bash
# 1. Initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}' | ./dbus-mcp

# 2. List tools
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | ./dbus-mcp

# 3. Check nginx status
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"systemd_status","arguments":{"service":"nginx"}}}' | ./dbus-mcp

# 4. Read configuration
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"file_read","arguments":{"path":"/etc/nginx/nginx.conf"}}}' | ./dbus-mcp

# 5. List network interfaces
echo '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"network_interfaces","arguments":{}}}' | ./dbus-mcp
```

### Python Client Example

```python
import json
import subprocess

class MCPClient:
    def __init__(self, server_path="/usr/local/bin/dbus-mcp"):
        self.server_path = server_path
        self.process = subprocess.Popen(
            [server_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        self.request_id = 0
    
    def call(self, method, params=None):
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params or {}
        }
        
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        
        response_line = self.process.stdout.readline()
        return json.loads(response_line)
    
    def initialize(self):
        return self.call("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {"call": True}}
        })
    
    def list_tools(self):
        return self.call("tools/list")
    
    def call_tool(self, name, arguments):
        return self.call("tools/call", {
            "name": name,
            "arguments": arguments
        })

# Usage
client = MCPClient()
client.initialize()

# Get systemd service status
result = client.call_tool("systemd_status", {"service": "nginx"})
print(result["result"]["content"][0]["text"])

# Read file
result = client.call_tool("file_read", {"path": "/etc/hostname"})
print(result["result"]["content"][0]["text"])
```

### JavaScript/TypeScript Client Example

```typescript
import { spawn } from 'child_process';
import * as readline from 'readline';

class MCPClient {
  private process;
  private rl;
  private requestId = 0;
  private pendingRequests = new Map();

  constructor(serverPath = '/usr/local/bin/dbus-mcp') {
    this.process = spawn(serverPath);
    this.rl = readline.createInterface({
      input: this.process.stdout,
      output: this.process.stdin
    });

    this.rl.on('line', (line) => {
      const response = JSON.parse(line);
      const resolver = this.pendingRequests.get(response.id);
      if (resolver) {
        resolver(response);
        this.pendingRequests.delete(response.id);
      }
    });
  }

  async call(method: string, params?: any): Promise<any> {
    this.requestId++;
    const request = {
      jsonrpc: "2.0",
      id: this.requestId,
      method,
      params: params || {}
    };

    return new Promise((resolve) => {
      this.pendingRequests.set(this.requestId, resolve);
      this.process.stdin.write(JSON.stringify(request) + '\n');
    });
  }

  async initialize() {
    return this.call('initialize', {
      protocolVersion: '2024-11-05',
      capabilities: { tools: { call: true } }
    });
  }

  async listTools() {
    return this.call('tools/list');
  }

  async callTool(name: string, arguments: any) {
    return this.call('tools/call', { name, arguments });
  }
}

// Usage
async function main() {
  const client = new MCPClient();
  
  await client.initialize();
  
  // Check service status
  const status = await client.callTool('systemd_status', {
    service: 'nginx'
  });
  console.log(status.result.content[0].text);
  
  // List network interfaces
  const interfaces = await client.callTool('network_interfaces', {});
  console.log(JSON.stringify(interfaces, null, 2));
}

main();
```

### Curl Examples

```bash
# Using web interface
# Start web server first: ./dbus-mcp-web

# Initialize session
curl -X POST http://localhost:8080/api/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'

# List tools
curl -X POST http://localhost:8080/api/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'

# Call a tool
curl -X POST http://localhost:8080/api/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"systemd_status","arguments":{"service":"nginx"}}}'
```

### WebSocket Example

```javascript
// Connect to WebSocket interface
const ws = new WebSocket('ws://localhost:8080/ws');

ws.on('open', () => {
  // Send initialize
  ws.send(JSON.stringify({
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: {}
  }));
});

ws.on('message', (data) => {
  const response = JSON.parse(data);
  console.log('Response:', response);
  
  if (response.id === 1) {
    // Initialized, now list tools
    ws.send(JSON.stringify({
      jsonrpc: "2.0",
      id: 2,
      method: "tools/list"
    }));
  }
});
```

---

## Rate Limiting and Quotas

### Default Limits

| Resource | Limit | Configurable |
|----------|-------|--------------|
| Max concurrent requests | 10 | Yes |
| Max request size | 1 MB | Yes |
| Max response size | 10 MB | Yes |
| Request timeout | 30 seconds | Yes |
| Max agents | 10 | Yes |
| Agent memory limit | 100 MB | Yes |

### Configuration

```json
{
  "limits": {
    "max_concurrent_requests": 10,
    "max_request_size_bytes": 1048576,
    "max_response_size_bytes": 10485760,
    "request_timeout_seconds": 30,
    "max_agents": 10,
    "agent_memory_limit_mb": 100
  }
}
```

---

## Versioning

### Protocol Version

Current: `2024-11-05`

The protocol version follows the MCP specification versioning scheme.

### API Stability

| Component | Stability | Notes |
|-----------|-----------|-------|
| Core JSON-RPC methods | Stable | Won't change |
| Tool schemas | Stable | Backward compatible |
| Agent interfaces | Beta | May change |
| Web API | Beta | May change |

### Deprecation Policy

- Deprecated features marked in documentation
- Minimum 3 months deprecation period
- Migration guide provided
- Old versions supported for 6 months

---

## Integration Patterns

### Service Discovery Pattern

```python
# Discover and use all available services
client = MCPClient()
client.initialize()

tools = client.list_tools()
for tool in tools["result"]["tools"]:
    print(f"Found tool: {tool['name']}")
    print(f"  Description: {tool['description']}")
    print(f"  Schema: {json.dumps(tool['inputSchema'], indent=2)}")
```

### Error Handling Pattern

```python
try:
    result = client.call_tool("systemd_status", {"service": "nginx"})
    if "error" in result:
        print(f"Tool error: {result['error']['message']}")
    else:
        print(f"Success: {result['result']}")
except Exception as e:
    print(f"Communication error: {e}")
```

### Batch Operations Pattern

```python
# Execute multiple operations efficiently
tasks = [
    ("systemd_status", {"service": "nginx"}),
    ("systemd_status", {"service": "postgresql"}),
    ("network_interfaces", {}),
    ("file_read", {"path": "/etc/hostname"})
]

results = []
for tool_name, args in tasks:
    result = client.call_tool(tool_name, args)
    results.append(result)

# Process all results
for result in results:
    if "error" not in result:
        print(result["result"]["content"][0]["text"])
```

### Streaming Pattern

```python
# For operations that support streaming
import asyncio

async def stream_logs(service):
    async for chunk in client.stream_tool("systemd_logs_stream", {
        "service": service,
        "follow": True
    }):
        print(chunk["content"], end="")
```

---

This completes the comprehensive API reference documentation for the MCP integration.