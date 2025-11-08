# MCP Chat Console Architecture

**Universal administrative interface via MCP JSON-RPC**

## Overview

All op-dbus operations are exposed as MCP tools callable from a chat interface:
- **Container Management**: Create, start, stop, destroy containers
- **Network Operations**: Configure Netmaker, OVS, bridges
- **System Monitoring**: Logs, users, processes, resources
- **State Management**: Apply/query/diff declarative state

## Architecture

```
Chat Interface (Natural Language)
    ↓
MCP JSON-RPC Protocol
    ↓
MCP Bridge (Introspection-based)
    ↓
op-dbus D-Bus Service
    ↓
System Operations (Containers, Network, etc.)
```

### Auto-Discovery Flow

```
1. op-dbus exposes D-Bus interface
   └─ busctl introspect org.freedesktop.DBus /org/freedesktop/DBus

2. MCP bridge introspects capabilities
   └─ Discovers: container.create, logs.rotate, users.active, etc.

3. Chat console lists available tools
   └─ User sees: "Available commands: create container, rotate logs, check users..."

4. User types natural language
   └─ "create a container with netmaker"

5. Chat maps to MCP tool call
   └─ container.create(network_type="netmaker")

6. MCP bridge executes via D-Bus
   └─ Returns result to chat
```

## Container Profile Commands

### Profile 1: None (Socket Networking Only)

**Create OVS Bridge:**
```json
// Chat: "create ovs bridge for socket networking"
{
  "method": "tools/call",
  "params": {
    "name": "bridge.create",
    "arguments": {
      "name": "ovsbr0",
      "datapath_type": "netdev",
      "socket_ports": [
        {"name": "internal_100", "container_id": "100"},
        {"name": "internal_101", "container_id": "101"}
      ]
    }
  }
}
```

### Profile 2: Privacy Client (WireGuard + Warp + XRay)

**Deploy Privacy Chain:**
```json
// Chat: "deploy privacy client containers"
{
  "method": "tools/call",
  "params": {
    "name": "profile.deploy",
    "arguments": {
      "profile": "privacy-client",
      "obfuscation_level": 3,
      "containers": [
        {
          "id": 100,
          "name": "wireguard-gateway",
          "service": "wg-quick@wg0"
        },
        {
          "id": 101,
          "name": "warp-tunnel",
          "service": "wg-quick@wg-warp",
          "wg_tunnel": true
        },
        {
          "id": 102,
          "name": "xray-client",
          "service": "xray"
        }
      ]
    }
  }
}

// Response:
{
  "result": {
    "status": "deployed",
    "containers_created": 3,
    "ovs_ports": ["internal_100", "wg-warp", "internal_102"],
    "flows_installed": 18,
    "obfuscation_active": true
  }
}
```

**Check Privacy Chain Status:**
```json
// Chat: "check privacy chain status"
{
  "method": "tools/call",
  "params": {
    "name": "privacy.status",
    "arguments": {}
  }
}

// Response:
{
  "result": {
    "chain_active": true,
    "containers": {
      "100": {"status": "running", "service": "active"},
      "101": {"status": "running", "service": "active", "warp_connected": true},
      "102": {"status": "running", "service": "active"}
    },
    "traffic_flow": "client → wg(100) → warp(101) → xray(102) → internet",
    "obfuscation_level": 3
  }
}
```

### Profile 3: Privacy VPS (XRay Server)

**Deploy VPS Server:**
```json
// Chat: "deploy xray server on vps"
{
  "method": "tools/call",
  "params": {
    "name": "profile.deploy",
    "arguments": {
      "profile": "privacy-vps",
      "obfuscation_level": 2,
      "containers": [
        {
          "id": 100,
          "name": "xray-server",
          "service": "xray",
          "ports": [443, 80]
        }
      ]
    }
  }
}
```

### Profile 4: General + Netmaker

**Create Container with Netmaker:**
```json
// Chat: "create web server container with netmaker"
{
  "method": "tools/call",
  "params": {
    "name": "container.create",
    "arguments": {
      "id": 103,
      "name": "web-server",
      "template": "debian-13-netmaker",
      "network_type": "netmaker",
      "autostart": true,
      "services": ["nginx"]
    }
  }
}

// Response:
{
  "result": {
    "container_id": 103,
    "status": "created",
    "network": {
      "type": "netmaker",
      "interface": "nm-mesh0",
      "mesh_ip": "10.10.10.3",
      "enrolled": true
    },
    "services": {
      "nginx": "active"
    }
  }
}
```

**Join Existing Container to Netmaker:**
```json
// Chat: "join container 104 to netmaker"
{
  "method": "tools/call",
  "params": {
    "name": "netmaker.join",
    "arguments": {
      "container_id": 104,
      "token_file": "/etc/op-dbus/netmaker.env"
    }
  }
}

// Response:
{
  "result": {
    "container_id": 104,
    "netmaker_interface": "nm-mesh0",
    "mesh_ip": "10.10.10.4",
    "connected_peers": 3
  }
}
```

## System Monitoring Commands

### Log Management

**Rotate Logs:**
```json
// Chat: "rotate op-dbus logs"
{
  "method": "tools/call",
  "params": {
    "name": "logs.rotate",
    "arguments": {
      "service": "op-dbus",
      "keep": 10,
      "compress": true
    }
  }
}
```

**View Recent Logs:**
```json
// Chat: "show last 50 op-dbus logs"
{
  "method": "tools/call",
  "params": {
    "name": "logs.tail",
    "arguments": {
      "service": "op-dbus",
      "lines": 50
    }
  }
}
```

### User Activity

**Active Users:**
```json
// Chat: "how many users are active?"
{
  "method": "tools/call",
  "params": {
    "name": "users.active_count",
    "arguments": {}
  }
}

// Response:
{
  "result": {
    "active_users": 3,
    "users": [
      {"name": "root", "sessions": 2},
      {"name": "admin", "sessions": 1}
    ]
  }
}
```

**User Sessions:**
```json
// Chat: "show admin user sessions"
{
  "method": "tools/call",
  "params": {
    "name": "users.sessions",
    "arguments": {
      "username": "admin"
    }
  }
}
```

### Resource Monitoring

**Container Resources:**
```json
// Chat: "show container 100 resource usage"
{
  "method": "tools/call",
  "params": {
    "name": "container.resources",
    "arguments": {
      "container_id": 100
    }
  }
}

// Response:
{
  "result": {
    "container_id": 100,
    "cpu_percent": 5.2,
    "memory_mb": 128,
    "disk_mb": 450,
    "network": {
      "rx_bytes": 1048576,
      "tx_bytes": 524288
    }
  }
}
```

## OpenFlow Operations

### Flow Management

**List Flows:**
```json
// Chat: "show openflow flows for container 100"
{
  "method": "tools/call",
  "params": {
    "name": "openflow.flows.list",
    "arguments": {
      "bridge": "ovsbr0",
      "match": {"in_port": "internal_100"}
    }
  }
}
```

**Add Flow:**
```json
// Chat: "add flow to allow container 100 to 101"
{
  "method": "tools/call",
  "params": {
    "name": "openflow.flows.add",
    "arguments": {
      "bridge": "ovsbr0",
      "priority": 100,
      "match": {
        "in_port": "internal_100",
        "dl_type": "0x0800"
      },
      "actions": [
        {"type": "output", "port": "internal_101"}
      ]
    }
  }
}
```

**Check Obfuscation Status:**
```json
// Chat: "check obfuscation flows"
{
  "method": "tools/call",
  "params": {
    "name": "obfuscation.status",
    "arguments": {}
  }
}

// Response:
{
  "result": {
    "level": 3,
    "flows_active": 18,
    "features": [
      "vlan_rotation",
      "mac_randomization",
      "ttl_obfuscation",
      "packet_padding",
      "timing_jitter"
    ]
  }
}
```

## Netmaker Operations

**Check Mesh Status:**
```json
// Chat: "check netmaker mesh status"
{
  "method": "tools/call",
  "params": {
    "name": "netmaker.status",
    "arguments": {}
  }
}

// Response:
{
  "result": {
    "enrolled_containers": 5,
    "connected_peers": 4,
    "network_name": "mesh",
    "containers": [
      {"id": 103, "mesh_ip": "10.10.10.3", "connected": true},
      {"id": 104, "mesh_ip": "10.10.10.4", "connected": true}
    ]
  }
}
```

**Generate Enrollment Token:**
```json
// Chat: "generate netmaker enrollment token"
{
  "method": "tools/call",
  "params": {
    "name": "netmaker.token.generate",
    "arguments": {
      "network": "mesh",
      "expires_hours": 24
    }
  }
}

// Response:
{
  "result": {
    "token": "eyJhbGc...",
    "expires_at": "2025-11-09T06:00:00Z",
    "save_to": "/etc/op-dbus/netmaker.env"
  }
}
```

## State Management

**Apply Declarative State:**
```json
// Chat: "apply state from /etc/op-dbus/privacy-client.json"
{
  "method": "tools/call",
  "params": {
    "name": "state.apply",
    "arguments": {
      "file": "/etc/op-dbus/privacy-client.json",
      "validate_only": false
    }
  }
}

// Response:
{
  "result": {
    "changes_applied": 12,
    "containers_created": 3,
    "flows_installed": 18,
    "services_started": 3
  }
}
```

**Query Current State:**
```json
// Chat: "what's the current state?"
{
  "method": "tools/call",
  "params": {
    "name": "state.query",
    "arguments": {}
  }
}

// Response:
{
  "result": {
    "version": 1,
    "containers": [
      {"id": 100, "name": "wireguard-gateway", "status": "running"},
      {"id": 101, "name": "warp-tunnel", "status": "running"},
      {"id": 102, "name": "xray-client", "status": "running"}
    ],
    "bridge": {
      "name": "ovsbr0",
      "ports": 3,
      "flows": 18
    }
  }
}
```

**Diff State:**
```json
// Chat: "compare current state to desired state"
{
  "method": "tools/call",
  "params": {
    "name": "state.diff",
    "arguments": {
      "desired_file": "/etc/op-dbus/privacy-client.json"
    }
  }
}

// Response:
{
  "result": {
    "drift_detected": true,
    "differences": [
      {
        "path": "containers.100.services",
        "current": ["wg-quick@wg0"],
        "desired": ["wg-quick@wg0", "fail2ban"]
      },
      {
        "path": "bridge.flows.count",
        "current": 15,
        "desired": 18
      }
    ]
  }
}
```

## MCP Tool Discovery

All tools are auto-discovered via introspection:

```bash
# op-dbus exposes D-Bus interface
busctl introspect org.freedesktop.DBus /org/freedesktop/DBus

# MCP bridge discovers available methods
{
  "tools": [
    {"name": "container.create", "description": "Create new container"},
    {"name": "container.start", "description": "Start container"},
    {"name": "container.stop", "description": "Stop container"},
    {"name": "netmaker.join", "description": "Join container to Netmaker mesh"},
    {"name": "logs.rotate", "description": "Rotate service logs"},
    {"name": "users.active_count", "description": "Count active users"},
    {"name": "openflow.flows.list", "description": "List OpenFlow flows"},
    {"name": "state.apply", "description": "Apply declarative state"},
    ... (100+ tools auto-discovered)
  ]
}
```

## Chat Console UX

**Natural Language Examples:**

```
User: "create a privacy client setup with level 3 obfuscation"
→ MCP: profile.deploy(profile="privacy-client", obfuscation_level=3)
→ Result: ✓ Created containers 100, 101, 102 with 18 obfuscation flows

User: "is container 101 running?"
→ MCP: container.status(container_id=101)
→ Result: ✓ Container 101 (warp-tunnel) is running, Warp connected

User: "join container 105 to netmaker"
→ MCP: netmaker.join(container_id=105)
→ Result: ✓ Container 105 joined mesh at 10.10.10.5

User: "rotate all logs"
→ MCP: logs.rotate_all()
→ Result: ✓ Rotated 5 log files, compressed 3 old logs

User: "show privacy chain traffic"
→ MCP: privacy.traffic_stats()
→ Result: ✓ 1.2GB through chain today, 98% uptime
```

## Implementation Status

### Phase 1: D-Bus Interface (Complete)
- [x] op-dbus D-Bus service
- [x] Introspection support
- [x] Method exposure

### Phase 2: MCP Bridge (In Progress)
- [x] JSON-RPC protocol
- [x] Introspection parser
- [x] Auto-discovery
- [ ] Tool registration
- [ ] Chat console integration

### Phase 3: Container Operations (Planned)
- [ ] container.create
- [ ] container.start/stop/destroy
- [ ] container.status
- [ ] container.resources

### Phase 4: Netmaker Operations (Planned)
- [ ] netmaker.join
- [ ] netmaker.status
- [ ] netmaker.token.generate
- [ ] netmaker.peers.list

### Phase 5: System Monitoring (Planned)
- [ ] logs.rotate
- [ ] logs.tail
- [ ] users.active_count
- [ ] users.sessions

### Phase 6: OpenFlow Operations (Planned)
- [ ] openflow.flows.list
- [ ] openflow.flows.add
- [ ] openflow.flows.delete
- [ ] obfuscation.status

### Phase 7: State Management (Planned)
- [ ] state.apply
- [ ] state.query
- [ ] state.diff
- [ ] state.validate

## Benefits

1. **Universal Interface**: All operations through one chat console
2. **Auto-Discovery**: New capabilities automatically exposed
3. **Natural Language**: Chat maps English to MCP tools
4. **Type Safety**: JSON Schema from D-Bus signatures
5. **Introspection**: System self-documents capabilities
6. **Extensibility**: New plugins = new chat commands automatically

## Related Documentation

- **MCP-INTROSPECTION-FLOW.md**: Technical flow details
- **CONTAINER-PROFILES.md**: Container profile configurations
- **CONTAINER-CLI.md**: CLI usage inside containers

---

**Version**: 1.0.0
**Last Updated**: 2025-11-08
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx
