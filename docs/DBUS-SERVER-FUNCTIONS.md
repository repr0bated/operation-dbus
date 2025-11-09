# D-Bus Server Functions Reference

**All D-Bus interfaces exposed by op-dbus for MCP chat console integration**

## Overview

op-dbus exposes multiple D-Bus services on both **system bus** and **session bus** that can be called via:
- Direct D-Bus clients (busctl, d-feet, etc.)
- MCP JSON-RPC bridge → Chat console
- Programming language bindings (Python, Rust, etc.)

## Service 1: State Manager

**Service Name**: `org.opdbus`
**Object Path**: `/org/opdbus/state`
**Interface**: `org.opdbus.StateManager`
**Bus**: System bus

### Methods

#### apply_state
Apply declarative state to the system.

**Signature**: `apply_state(state_json: String) -> String`

**Input**:
```json
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [...]
    },
    "openflow": {
      "bridges": [...]
    }
  }
}
```

**Output**:
```
"Applied successfully: 5"
```

**D-Bus Call**:
```bash
busctl call org.opdbus /org/opdbus/state org.opdbus.StateManager \
  apply_state s '{"version":1,"plugins":{}}'
```

**MCP JSON-RPC**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "state.apply",
    "arguments": {
      "state_json": "{...}"
    }
  }
}
```

#### query_state
Query current system state from all plugins.

**Signature**: `query_state() -> String`

**Output**:
```json
{
  "version": 1,
  "plugins": {
    "lxc": {...},
    "net": {...},
    "systemd": {...}
  }
}
```

**D-Bus Call**:
```bash
busctl call org.opdbus /org/opdbus/state org.opdbus.StateManager query_state
```

**MCP JSON-RPC**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "state.query",
    "arguments": {}
  }
}
```

## Service 2: Orchestrator

**Service Name**: `org.dbusmcp`
**Object Path**: `/org/dbusmcp/orchestrator`
**Interface**: `org.dbusmcp.Orchestrator`
**Bus**: Session bus

### Methods

#### spawn_agent
Dynamically spawn a new agent instance.

**Signature**: `spawn_agent(agent_type: String, config_json: String) -> String`

**Input**:
- `agent_type`: "file", "network", "systemd", "monitor", "executor"
- `config_json`: Agent-specific configuration (optional, can be empty string)

**Output**:
```
"agent-uuid-12345"
```

**D-Bus Call**:
```bash
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" ""
```

**MCP JSON-RPC**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "agent.spawn",
    "arguments": {
      "agent_type": "file",
      "config_json": ""
    }
  }
}
```

#### send_task
Send a task to an agent.

**Signature**: `send_task(agent_id: String, task_json: String) -> String`

**Input**:
```json
{
  "type": "file_operation",
  "operation": "read",
  "path": "/tmp/test.txt"
}
```

**Output**:
```
"task-uuid-67890"
```

**D-Bus Call**:
```bash
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator send_task ss \
  "agent-uuid-12345" '{"type":"file_operation","operation":"read","path":"/tmp/test.txt"}'
```

**MCP JSON-RPC**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "task.send",
    "arguments": {
      "agent_id": "agent-uuid-12345",
      "task_json": "{...}"
    }
  }
}
```

#### get_agent_status
Get status of an agent.

**Signature**: `get_agent_status(agent_id: String) -> String`

**Output**:
```json
{
  "agent_id": "agent-uuid-12345",
  "agent_type": "file",
  "status": "running",
  "created_at": "2025-11-08T06:00:00Z"
}
```

**D-Bus Call**:
```bash
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator get_agent_status s "agent-uuid-12345"
```

**MCP JSON-RPC**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "agent.status",
    "arguments": {
      "agent_id": "agent-uuid-12345"
    }
  }
}
```

#### list_agents
List all spawned agents.

**Signature**: `list_agents() -> String`

**Output**:
```json
{
  "agents": [
    {
      "id": "agent-uuid-1",
      "type": "file",
      "status": "running"
    },
    {
      "id": "agent-uuid-2",
      "type": "network",
      "status": "idle"
    }
  ]
}
```

## Service 3: File Agent

**Service Name**: `org.dbusmcp.agents`
**Object Path**: `/org/dbusmcp/agents/{agent_id}`
**Interface**: `org.dbusmcp.Agent.File`
**Bus**: Session bus

### Methods

#### execute
Execute a file operation task safely.

**Signature**: `execute(task_json: String) -> String`

**Supported Operations**:
- `read`: Read file contents
- `write`: Write file contents
- `delete`: Delete file or directory
- `exists`: Check if file exists
- `list`: List directory contents
- `mkdir`: Create directory

**Input (Read)**:
```json
{
  "type": "file_operation",
  "operation": "read",
  "path": "/tmp/test.txt"
}
```

**Output (Read)**:
```json
{
  "success": true,
  "operation": "read",
  "path": "/tmp/test.txt",
  "data": "file contents here"
}
```

**Input (Write)**:
```json
{
  "type": "file_operation",
  "operation": "write",
  "path": "/tmp/test.txt",
  "content": "new file contents"
}
```

**Output (Write)**:
```json
{
  "success": true,
  "operation": "write",
  "path": "/tmp/test.txt",
  "data": "Wrote 17 bytes"
}
```

**Input (List)**:
```json
{
  "type": "file_operation",
  "operation": "list",
  "path": "/tmp"
}
```

**Output (List)**:
```json
{
  "success": true,
  "operation": "list",
  "path": "/tmp",
  "data": "[\"file1.txt\",\"file2.txt\",\"dir1\"]"
}
```

**Security Features**:
- Whitelisted directories: /home, /tmp, /var/log, /opt
- Blacklisted directories: /etc, /root, /boot, /sys, /proc, /dev, /usr/bin, /usr/sbin
- Forbidden files: SSH keys, shadow, passwd, .env, .git/config
- Max file size: 10MB
- Max path length: 4096 chars
- Recursive deletion limited to 100 files

**D-Bus Call**:
```bash
# First spawn a file agent
AGENT_ID=$(busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" "")

# Then execute operation
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"read","path":"/tmp/test.txt"}'
```

**MCP JSON-RPC**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "file.read",
    "arguments": {
      "path": "/tmp/test.txt"
    }
  }
}
```

## Service 4: Network Agent

**Service Name**: `org.dbusmcp.agents`
**Object Path**: `/org/dbusmcp/agents/{agent_id}`
**Interface**: `org.dbusmcp.Agent.Network`
**Bus**: Session bus

### Methods

#### execute
Execute network operation tasks.

**Signature**: `execute(task_json: String) -> String`

**Supported Operations**:
- Configure network interfaces
- Set IP addresses
- Manage routes
- Configure OVS bridges
- Inspect network state

**Input Example**:
```json
{
  "type": "network_operation",
  "operation": "configure_interface",
  "interface": "eth0",
  "ip": "10.0.0.100/24"
}
```

## Service 5: Systemd Agent

**Service Name**: `org.dbusmcp.agents`
**Object Path**: `/org/dbusmcp/agents/{agent_id}`
**Interface**: `org.dbusmcp.Agent.Systemd`
**Bus**: Session bus

### Methods

#### execute
Execute systemd operations.

**Signature**: `execute(task_json: String) -> String`

**Supported Operations**:
- Start/stop/restart services
- Enable/disable services
- Query service status
- Manage units

**Input Example**:
```json
{
  "type": "systemd_operation",
  "operation": "start",
  "unit": "wg-quick@wg0.service"
}
```

## Service 6: Monitor Agent

**Service Name**: `org.dbusmcp.agents`
**Object Path**: `/org/dbusmcp/agents/{agent_id}`
**Interface**: `org.dbusmcp.Agent.Monitor`
**Bus**: Session bus

### Methods

#### execute
Execute monitoring operations.

**Signature**: `execute(task_json: String) -> String`

**Supported Operations**:
- Monitor system resources
- Track process metrics
- Monitor network traffic
- Query system health

**Input Example**:
```json
{
  "type": "monitor_operation",
  "operation": "cpu_usage"
}
```

## Service 7: Executor Agent

**Service Name**: `org.dbusmcp.agents`
**Object Path**: `/org/dbusmcp/agents/{agent_id}`
**Interface**: `org.dbusmcp.Agent.Executor`
**Bus**: Session bus

### Methods

#### execute
Execute arbitrary commands (with restrictions).

**Signature**: `execute(task_json: String) -> String`

**Supported Operations**:
- Run shell commands (whitelisted)
- Execute scripts
- Run system utilities

**Input Example**:
```json
{
  "type": "command_execution",
  "operation": "run",
  "command": "uptime"
}
```

## MCP Chat Console Integration

All D-Bus methods are automatically exposed as MCP tools through the introspection bridge.

### Chat Examples

**Example 1: Apply Privacy Client State**
```
User: "deploy privacy client containers with level 3 obfuscation"

→ MCP maps to: state.apply
→ D-Bus call: org.opdbus.StateManager.apply_state
→ Response: ✓ Applied successfully: 18 changes
```

**Example 2: Check Current State**
```
User: "what's the current state?"

→ MCP maps to: state.query
→ D-Bus call: org.opdbus.StateManager.query_state
→ Response: Shows JSON with all plugin states
```

**Example 3: Read Container Config**
```
User: "read /etc/op-dbus/privacy-client.json"

→ MCP maps to: file.read
→ Spawns file agent if needed
→ D-Bus call: org.dbusmcp.Agent.File.execute
→ Response: Shows file contents
```

**Example 4: Check Service Status**
```
User: "is wg-quick@wg0 running?"

→ MCP maps to: systemd.status
→ Spawns systemd agent if needed
→ D-Bus call: org.dbusmcp.Agent.Systemd.execute
→ Response: ✓ Active: active (running)
```

## Testing D-Bus Services

### Test 1: State Manager (System Bus)

```bash
# Start op-dbus daemon (if not running)
sudo op-dbus run &

# Wait for D-Bus service to register
sleep 2

# Test query_state
sudo busctl call org.opdbus /org/opdbus/state \
  org.opdbus.StateManager query_state

# Test apply_state
sudo busctl call org.opdbus /org/opdbus/state \
  org.opdbus.StateManager apply_state s \
  '{"version":1,"plugins":{"systemd":{"units":{}}}}'
```

### Test 2: Orchestrator (Session Bus)

```bash
# Start orchestrator (session bus - no sudo)
op-dbus orchestrator &

# List available agents
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator list_agents

# Spawn a file agent
AGENT_ID=$(busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" "" | awk '{print $2}' | tr -d '"')

echo "Spawned agent: $AGENT_ID"

# Check agent status
busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator get_agent_status s "$AGENT_ID"
```

### Test 3: File Agent Operations

```bash
# Create test file
echo "test content" > /tmp/test-opdbus.txt

# Spawn file agent
AGENT_ID=$(busctl --user call org.dbusmcp /org/dbusmcp/orchestrator \
  org.dbusmcp.Orchestrator spawn_agent ss "file" "" | awk '{print $2}' | tr -d '"')

# Read file
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"read","path":"/tmp/test-opdbus.txt"}'

# List directory
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"list","path":"/tmp"}'

# Check if file exists
busctl --user call org.dbusmcp.agents "/org/dbusmcp/agents/$AGENT_ID" \
  org.dbusmcp.Agent.File execute s \
  '{"type":"file_operation","operation":"exists","path":"/tmp/test-opdbus.txt"}'
```

### Test 4: MCP JSON-RPC Bridge

```bash
# Start MCP bridge
op-dbus mcp-bridge &

# Send JSON-RPC request (via stdio or socket)
echo '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "state.query",
    "arguments": {}
  }
}' | op-dbus mcp-bridge
```

## Introspection

All services support D-Bus introspection for auto-discovery:

```bash
# Introspect StateManager
busctl introspect --xml-interface org.opdbus /org/opdbus/state

# Introspect Orchestrator
busctl --user introspect --xml-interface org.dbusmcp /org/dbusmcp/orchestrator

# Introspect File Agent (after spawning)
AGENT_ID="..."
busctl --user introspect --xml-interface org.dbusmcp.agents \
  "/org/dbusmcp/agents/$AGENT_ID"
```

## Service Discovery

MCP bridge auto-discovers all services on startup:

```rust
// Discovers all org.dbusmcp.* services
// Discovers all org.opdbus.* services
// Parses introspection XML
// Generates MCP tool schemas
// Exposes via JSON-RPC
```

## Container Profile Integration

### Profile 2: Privacy Client

**Chat**: "deploy privacy client"

**D-Bus Call Flow**:
```
1. state.apply → org.opdbus.StateManager.apply_state
   ├─ Reads /etc/op-dbus/privacy-client.json
   ├─ Applies LXC containers (100, 101, 102)
   ├─ Configures OVS bridge + flows
   └─ Starts services in containers

2. Response: Applied successfully: 18
```

### Profile 4: General + Netmaker

**Chat**: "create container with netmaker"

**D-Bus Call Flow**:
```
1. container.create → Custom method (to be added)
   ├─ Creates LXC container via pct/lxc-create
   ├─ Injects Netmaker token
   ├─ Starts container
   └─ Auto-joins Netmaker mesh

2. Response: {
     "container_id": 103,
     "mesh_ip": "10.10.10.3"
   }
```

## Future Enhancements

### Additional Methods to Add

**Container Management** (org.opdbus.Container):
- `create_container(config_json: String) -> String`
- `start_container(container_id: String) -> String`
- `stop_container(container_id: String) -> String`
- `destroy_container(container_id: String) -> String`
- `list_containers() -> String`

**Netmaker Operations** (org.opdbus.Netmaker):
- `join_mesh(container_id: String, token: String) -> String`
- `leave_mesh(container_id: String) -> String`
- `mesh_status() -> String`
- `generate_token(network: String) -> String`

**OpenFlow Operations** (org.opdbus.OpenFlow):
- `add_flow(bridge: String, flow_json: String) -> String`
- `delete_flow(bridge: String, flow_id: String) -> String`
- `list_flows(bridge: String) -> String`
- `obfuscation_status() -> String`

**Log Management** (org.opdbus.Logs):
- `rotate_logs(service: String) -> String`
- `tail_logs(service: String, lines: i32) -> String`
- `clear_logs(service: String) -> String`

**User Monitoring** (org.opdbus.Users):
- `active_users() -> String`
- `user_sessions(username: String) -> String`
- `login_history() -> String`

## Related Documentation

- **MCP-CHAT-CONSOLE.md**: Chat console UX and examples
- **MCP-INTROSPECTION-FLOW.md**: Introspection architecture
- **CONTAINER-PROFILES.md**: Container deployment profiles
- **CONTAINER-CLI.md**: CLI usage inside containers

---

**Version**: 1.0.0
**Last Updated**: 2025-11-08
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx
