# MCP Architecture: Separate Servers Per Domain

## Overview

Instead of one monolithic MCP server with all tools, op-dbus uses **multiple specialized MCP servers**, each focusing on a specific domain. The **MCP Manager** orchestrates these servers and routes requests.

```
┌─────────────────────────────────────────────────────────┐
│                   MCP Manager (Orchestrator)             │
│           Routes requests to appropriate servers          │
│              Manages server lifecycle & discovery         │
└───────────┬────────────┬────────────┬───────────────────┘
            │            │            │
┌───────────┴──┐  ┌──────┴────┐  ┌───┴───────────┐
│ Introspection│  │  Netmaker  │  │   Hardware    │
│ MCP Server   │  │ MCP Server │  │  MCP Server   │
│              │  │            │  │               │
│ Tools:       │  │ Tools:     │  │ Tools:        │
│ - discover_  │  │ - net_     │  │ - get_cpu_    │
│   system     │  │   create   │  │   info        │
│ - analyze_   │  │ - net_     │  │ - get_mem_    │
│   cpu        │  │   delete   │  │   info        │
│ - analyze_   │  │ - net_     │  │ - get_disk_   │
│   isp        │  │   status   │  │   info        │
└──────────────┘  └───────────┘  └───────────────┘
```

## Benefits

### 1. **Isolation**
- Each server runs independently
- Crash in one doesn't affect others
- Can restart individual servers without full system restart

### 2. **Specialization**
- Introspection server: Deep system analysis
- Netmaker server: VPN mesh management
- Hardware server: Direct hardware access
- Storage server: BTRFS, snapshots, blockchain

### 3. **Scalability**
- Can run servers on different machines
- Introspection server on the machine being analyzed
- Manager on central control machine
- Hardware-intensive servers on bare metal

### 4. **Security**
- Each server has minimal permissions
- Introspection server: read-only system access
- Netmaker server: network permissions only
- Hardware server: direct device access only

### 5. **Development**
- Teams can work on separate servers independently
- Add new servers without modifying existing code
- Test one server without affecting others

---

## MCP Server Types

### 1. Introspection MCP Server

**Purpose**: System discovery and analysis
**Port**: 3001
**Tools**:
- `discover_system` - Full system introspection
- `analyze_cpu_features` - CPU feature & BIOS lock detection
- `analyze_isp` - ISP restriction analysis
- `generate_isp_request` - Support request generation
- `compare_hardware` - Configuration comparison

**Use Cases**:
- Hardware inventory
- Migration planning
- Cost optimization
- Provider comparison

### 2. Netmaker MCP Server

**Purpose**: VPN mesh management
**Port**: 3002
**Tools**:
- `netmaker_create_network` - Create VPN network
- `netmaker_add_node` - Add node to mesh
- `netmaker_get_status` - Network status
- `netmaker_list_nodes` - List all nodes
- `netmaker_remove_node` - Remove node

**Use Cases**:
- VPN provisioning
- Node management
- Network monitoring

### 3. Hardware MCP Server

**Purpose**: Direct hardware access
**Port**: 3003
**Tools**:
- `get_cpu_info` - CPU specs
- `get_mem_info` - Memory configuration
- `get_disk_info` - Storage devices
- `get_network_info` - Network interfaces
- `get_gpu_info` - GPU detection

**Use Cases**:
- Hardware inventory
- Performance monitoring
- Capacity planning

### 4. Storage MCP Server

**Purpose**: BTRFS, snapshots, blockchain
**Port**: 3004
**Tools**:
- `create_snapshot` - BTRFS snapshot
- `list_snapshots` - List all snapshots
- `restore_snapshot` - Rollback to snapshot
- `get_blockchain_state` - Blockchain audit trail
- `verify_integrity` - Check snapshot integrity

**Use Cases**:
- Disaster recovery
- State management
- Audit trails

### 5. Plugin MCP Server

**Purpose**: D-Bus plugin management
**Port**: 3005
**Tools**:
- `list_plugins` - List installed plugins
- `plugin_status` - Get plugin state
- `apply_plugin_state` - Apply configuration
- `generate_plugin` - Auto-generate new plugin

**Use Cases**:
- Infrastructure as code
- Configuration management
- Service orchestration

---

## MCP Manager API

The Manager orchestrates all MCP servers and provides unified access.

### Endpoints

#### GET /api/mcp/servers
List all registered MCP servers

**Response**:
```json
{
  "servers": [
    {
      "id": "introspection",
      "name": "Introspection Server",
      "status": "running",
      "endpoint": "http://localhost:3001",
      "tools_count": 5,
      "last_heartbeat": "2025-11-06T12:00:00Z"
    },
    {
      "id": "netmaker",
      "name": "Netmaker Server",
      "status": "running",
      "endpoint": "http://localhost:3002",
      "tools_count": 5,
      "last_heartbeat": "2025-11-06T12:00:00Z"
    }
  ]
}
```

#### POST /api/mcp/servers/:id/start
Start a specific MCP server

#### POST /api/mcp/servers/:id/stop
Stop a specific MCP server

#### POST /api/mcp/execute
Execute a tool on any server (auto-routed)

**Request**:
```json
{
  "tool": "discover_system",
  "parameters": {
    "include_packages": false,
    "detect_provider": true
  }
}
```

**Response**:
```json
{
  "server": "introspection",
  "result": { ... }
}
```

---

## Configuration

### /etc/op-dbus/mcp.toml

```toml
[manager]
bind = "0.0.0.0:3000"
web_ui = true

[servers.introspection]
enabled = true
port = 3001
auto_start = true
restart_on_failure = true

[servers.netmaker]
enabled = true
port = 3002
auto_start = true
restart_on_failure = true

[servers.hardware]
enabled = true
port = 3003
auto_start = true

[servers.storage]
enabled = true
port = 3004
auto_start = true

[servers.plugin]
enabled = true
port = 3005
auto_start = false  # Start on demand
```

---

## Deployment Scenarios

### Scenario 1: Single Machine (Development)

```
Local Machine
├── MCP Manager (port 3000)
├── Introspection Server (port 3001)
├── Netmaker Server (port 3002)
├── Hardware Server (port 3003)
├── Storage Server (port 3004)
└── Plugin Server (port 3005)
```

All servers run on localhost, manager orchestrates them.

### Scenario 2: Distributed (Production)

```
Control Machine (Laptop/Desktop)
└── MCP Manager (port 3000)
    ├── Web UI (for human interaction)
    └── Routes to remote servers

Remote Server 1 (HostKey VPS - being migrated)
├── Introspection Server (port 3001)
└── Netmaker Server (port 3002)

Remote Server 2 (Hetzner Bare Metal - migration target)
├── Introspection Server (port 3001)
├── Hardware Server (port 3003)
└── Storage Server (port 3004)

Samsung 360 Pro Laptop (Reference implementation)
├── Introspection Server (port 3001)
└── Hardware Server (port 3003)
```

Manager on control machine connects to MCP servers on remote machines.

### Scenario 3: Enterprise (Multi-Tenant)

```
Central MCP Manager Cluster
└── Routes requests to tenant-specific servers

Tenant 1 Servers
├── Introspection Server (isolated)
├── Netmaker Server (isolated)
└── Storage Server (isolated)

Tenant 2 Servers
├── Introspection Server (isolated)
├── Netmaker Server (isolated)
└── Storage Server (isolated)
```

Complete isolation between tenants, shared manager.

---

## Tool Discovery

Each MCP server exposes its tools via JSON-RPC `tools/list`:

**Introspection Server** (`http://localhost:3001`):
```json
{
  "tools": [
    {
      "name": "discover_system",
      "description": "Full system introspection",
      "inputSchema": { ... }
    },
    {
      "name": "analyze_cpu_features",
      "description": "CPU feature detection",
      "inputSchema": { ... }
    }
  ]
}
```

Manager aggregates all tools from all servers into unified catalog.

---

## Chat Interface Integration

Chat interface can target specific servers or let manager route:

```
User: "Show me CPU features locked by BIOS"

Manager:
1. Parse intent → wants CPU feature analysis
2. Route to Introspection Server (port 3001)
3. Call "analyze_cpu_features" tool
4. Return results to chat

User: "Create a Netmaker network called 'prod'"

Manager:
1. Parse intent → Netmaker operation
2. Route to Netmaker Server (port 3002)
3. Call "netmaker_create_network" tool
4. Return results to chat
```

---

## Samsung 360 Pro Testing

On your laptop, you'll run:

```bash
# Start MCP Manager
op-dbus mcp-manager start

# Start Introspection Server (discovers hardware, BIOS locks)
op-dbus mcp-server start introspection --port 3001

# Start Hardware Server (direct hardware access)
op-dbus mcp-server start hardware --port 3003

# Access web UI
http://localhost:3000/

# In chat or via tools:
discover_system → detects Samsung 360 Pro buggy BIOS
analyze_cpu_features → shows VT-x lock, suggests unlock methods
```

This proves the architecture works on problematic hardware before deploying to production.

---

## Comparison: Monolithic vs Microservices

### Monolithic (Old Way)
```
One MCP Server
├── 50+ tools lumped together
├── One crash = total failure
├── Hard to maintain
└── Can't distribute
```

### Microservices (New Way)
```
MCP Manager + 5 Specialized Servers
├── 10 tools per server (focused)
├── Isolated failures
├── Easy to maintain individual servers
└── Can distribute across machines
```

---

## Implementation Plan

### Phase 1: Core Infrastructure (This session)
- [x] MCP Manager server
- [x] Introspection tools registered
- [ ] Server spawning system
- [ ] Tool routing logic

### Phase 2: Specialized Servers
- [ ] Introspection Server (discover_system, analyze_cpu, analyze_isp)
- [ ] Hardware Server (cpu_info, mem_info, disk_info)
- [ ] Storage Server (snapshots, blockchain)

### Phase 3: Integration
- [ ] Web UI shows all servers and tools
- [ ] Chat interface routes to correct server
- [ ] Cross-server workflows

### Phase 4: Testing
- [ ] Test on Samsung 360 Pro
- [ ] Test distributed deployment (laptop manager → VPS servers)
- [ ] Test failure scenarios (one server crashes)

---

## Summary

**Architecture**: Microservices pattern for MCP servers
**Manager**: Central orchestrator + web UI
**Servers**: Specialized, isolated, independently scalable
**Benefits**: Isolation, specialization, scalability, security

**Next steps**: Finish manager implementation, create server spawning system, test on Samsung 360 Pro.
