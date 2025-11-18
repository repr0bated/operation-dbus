# Privacy Router Tunnel Architecture

**Complete multi-hop privacy tunnel with socket networking and function-based routing**

## Architecture Overview

```
Internet
   ↓
[VPS XRay Server]
   ↓
[Proxmox Node - OVS Bridge (vmbr0/ovsbr0)]
   ↓
┌─────────────────────────────────────────────────────────────┐
│ Entry Point: WireGuard Gateway (zero config)               │
│ Container: 100, Socket: internal_100                        │
└─────────────────────────────────────────────────────────────┘
   ↓
┌─────────────────────────────────────────────────────────────┐
│ wgcf WARP Tunnel                                             │
│ Interface: warp0 (on OVS bridge)                            │
└─────────────────────────────────────────────────────────────┘
   ↓
┌─────────────────────────────────────────────────────────────┐
│ XRay Client                                                  │
│ Container: 101, Socket: internal_101                         │
│ SOCKS Port: 1080                                             │
└─────────────────────────────────────────────────────────────┘
   ↓
[VPS XRay Server] → Internet
```

## Key Components

### 1. WireGuard Gateway (Entry Point)
- **Container ID**: 100
- **Socket Port**: `internal_100`
- **Zero Config**: Auto-generates keys
- **Listen Port**: 51820
- **Purpose**: Initial VPN entry point for client devices

### 2. WARP Tunnel (wgcf)
- **Interface**: `warp0`
- **Type**: System service (not container)
- **Purpose**: Cloudflare WARP for enhanced privacy and speed
- **Connection**: Directly on OVS bridge

### 3. XRay Client
- **Container ID**: 101
- **Socket Port**: `internal_101`
- **SOCKS Port**: 1080
- **VPS Connection**: Connects to VPS XRay server
- **Purpose**: Advanced proxy client with traffic obfuscation

### 4. VPS XRay Server
- **Endpoint**: Configurable (e.g., `vps.example.com:443`)
- **Purpose**: Final encrypted tunnel to internet

## Networking Architecture

### Socket Networking (LXC Module)
- **Separate from container socket network**: Privacy tunnel uses dedicated socket ports
- **Pattern**: `internal_{container_id}` (e.g., `internal_100`, `internal_101`)
- **Type**: OVS internal ports (no veth interfaces)
- **Purpose**: Containerless networking via OpenFlow routing

### OVS Bridge
- **Name**: `ovsbr0` or `vmbr0` (configurable)
- **Shared by**: All components (privacy tunnel + mesh networking)
- **Ports**:
  - `internal_100` (WireGuard gateway)
  - `internal_101` (XRay client)
  - `warp0` (WARP tunnel interface)
  - `nm-privacy` (Netmaker mesh interface)
  - Additional container sockets (vector DB, bucket storage, etc.)

### OpenFlow Routing (Rust Implementation)

#### Privacy Flow Routing
Rewritten in Rust for performance and security:

1. **WireGuard → WARP**
   ```
   priority=100, in_port=internal_100, actions=output:warp0
   ```

2. **WARP → XRay**
   ```
   priority=100, in_port=warp0, actions=output:internal_101
   ```

3. **XRay → WARP (return)**
   ```
   priority=100, in_port=internal_101, actions=output:warp0
   ```

4. **Security Flows** (if enabled):
   - Drop invalid TCP flags (NULL/Xmas/FIN scans)
   - Drop IP fragmentation attacks
   - Rate limit ARP/ICMP
   - Drop invalid source IPs
   - Connection tracking (stateful firewall)
   - Egress filtering (block TTL <=1)

5. **Obfuscation Flows** (Level 2 - Pattern Hiding):
   - TTL normalization
   - Packet padding
   - Timing randomization

#### Function-Based Routing to Sockets (Netmaker Mesh)

Routing by function to sockets on Netmaker mesh:

- **Vector Database Container**: Function `vector_db` → Socket `internal_200`
- **Bucket Storage Container**: Function `bucket_storage` → Socket `internal_201`
- **Other Services**: Additional containers with function-based routing

**Example Flow**:
```
priority=90, tcp, tp_dst=8080, actions=output:internal_200  # Vector DB
priority=90, tcp, tp_dst=9000, actions=output:internal_201  # Bucket Storage
```

### Netmaker Mesh Integration

- **Interface**: `nm-privacy` (one per Proxmox node)
- **Network**: `privacy-mesh`
- **Location**: Same OVS bridge as privacy tunnel
- **Purpose**: Mesh networking for containers (vector DB, bucket storage, etc.)
- **Routing**: Function-based routing via OpenFlow flows

**Per-Node Configuration**:
- Each Proxmox node has one Netmaker interface
- All interfaces on the same OVS bridge
- OpenFlow rules route mesh traffic appropriately

## Container Architecture

### Privacy Tunnel Containers
1. **Container 100**: WireGuard Gateway
   - Socket: `internal_100`
   - Network Type: `privacy`
   - Purpose: Entry point

2. **Container 101**: XRay Client
   - Socket: `internal_101`
   - Network Type: `privacy`
   - Purpose: Proxy client

### Mesh Containers (Netmaker)
3. **Container 200+**: Vector Database
   - Socket: `internal_200`
   - Network Type: `mesh`
   - Function: `vector_db`

4. **Container 201+**: Bucket Storage
   - Socket: `internal_201`
   - Network Type: `mesh`
   - Function: `bucket_storage`

5. **Additional Containers**: As needed
   - Each with socket networking
   - Function-based routing via OpenFlow

## Configuration Example

```json
{
  "version": 1,
  "plugins": {
    "privacy_router": {
      "bridge_name": "ovsbr0",
      "wireguard": {
        "enabled": true,
        "container_id": 100,
        "socket_port": "internal_100",
        "zero_config": true,
        "listen_port": 51820
      },
      "warp": {
        "enabled": true,
        "interface": "warp0",
        "wgcf_config": null
      },
      "xray": {
        "enabled": true,
        "container_id": 101,
        "socket_port": "internal_101",
        "socks_port": 1080,
        "vps_address": "vps.example.com",
        "vps_port": 443
      },
      "vps": {
        "xray_server": "vps.example.com",
        "xray_port": 443
      },
      "socket_networking": {
        "enabled": true,
        "network_type": "privacy",
        "privacy_sockets": [
          {"name": "internal_100", "container_id": 100, "port_type": "privacy"},
          {"name": "internal_101", "container_id": 101, "port_type": "privacy"}
        ],
        "mesh_sockets": [
          {"name": "internal_200", "container_id": 200, "port_type": "mesh"},
          {"name": "internal_201", "container_id": 201, "port_type": "mesh"}
        ]
      },
      "openflow": {
        "enabled": true,
        "enable_security_flows": true,
        "obfuscation_level": 2,
        "privacy_flows": [
          {
            "priority": 100,
            "match_fields": {"in_port": "internal_100"},
            "actions": ["output:warp0"],
            "description": "WireGuard gateway → WARP tunnel"
          },
          {
            "priority": 100,
            "match_fields": {"in_port": "warp0"},
            "actions": ["output:internal_101"],
            "description": "WARP → XRay client"
          },
          {
            "priority": 100,
            "match_fields": {"in_port": "internal_101"},
            "actions": ["output:warp0"],
            "description": "XRay client → WARP (return)"
          }
        ],
        "function_routing": [
          {
            "function": "vector_db",
            "target_socket": "internal_200",
            "match_fields": {"tcp_dst": "8080"}
          },
          {
            "function": "bucket_storage",
            "target_socket": "internal_201",
            "match_fields": {"tcp_dst": "9000"}
          }
        ]
      },
      "netmaker": {
        "enabled": true,
        "interface": "nm-privacy",
        "network_name": "privacy-mesh",
        "per_node_interface": true,
        "node_id": null
      },
      "containers": [
        {
          "id": 200,
          "name": "vector-db",
          "container_type": "vector_db",
          "socket_port": "internal_200",
          "network_type": "mesh"
        },
        {
          "id": 201,
          "name": "bucket-storage",
          "container_type": "bucket_storage",
          "socket_port": "internal_201",
          "network_type": "mesh"
        }
      ]
    }
  }
}
```

## Implementation Details

### op-dbus Binary Controller
- **Location**: `src/main.rs`
- **Purpose**: Main orchestrator that registers and coordinates all plugins
- **Registered Plugins**:
  - `net` - Network/OVS bridge management
  - `systemd` - Systemd service management
  - `login1` - Login1 D-Bus interface
  - `lxc` - LXC container management with socket networking
  - `sessdecl` - Session declaration
  - `dns` - DNS resolver
  - `pcidecl` - PCI device declaration
  - `packagekit` - Package management
  - `openflow` - OpenFlow flow management (if `openflow` feature enabled) - **Core routing engine**
  - `privacy` - Privacy router coordination (if `openflow` feature enabled)
  - `netmaker` - Netmaker mesh networking (if `openflow` feature enabled)
  - `privacy_router` - Complete privacy router tunnel orchestration (if `openflow` feature enabled)

### OpenFlow Plugin (Core Routing Engine)
- **Location**: `src/state/plugins/openflow.rs`
- **Purpose**: **Core routing engine** - Manages OpenFlow flows for socket-based container networking
- **Registration**: Registered by `op-dbus` binary controller in `main.rs` (line ~500)
- **Features**:
  - Policy-based flow generation
  - Automatic container discovery via OVSDB introspection
  - Security hardening flows (drop invalid, rate limit, etc.)
  - Traffic obfuscation (Level 1-3)
  - Flow templates for policy-based routing
  - Socket port management
  - Privacy flow routing (WireGuard → WARP → XRay)
  - Function-based routing to sockets (vector DB, bucket storage, etc.)
- **Integration**: 
  - Used by `privacy_router` plugin to set up privacy flows
  - Used by `netmaker` plugin for mesh routing
  - Directly accessible via D-Bus (`org.opdbus` service)

### Rust OpenFlow Native Implementation
- **Location**: `src/native/openflow.rs`
- **Purpose**: Low-level OpenFlow protocol implementation
- **Features**:
  - Direct OpenFlow protocol communication (no CLI tools)
  - Flow match/action encoding
  - OpenFlow 1.3 support
  - High-performance flow management

### LXC Socket Networking
- **Location**: `src/state/plugins/lxc.rs`
- **Pattern**: `internal_{container_id}`
- **Type**: OVS internal ports
- **Separation**: Privacy sockets vs. mesh sockets

### Netmaker Integration
- **Location**: `src/state/plugins/netmaker.rs`
- **Interface**: One per Proxmox node
- **Bridge**: Same OVS bridge as privacy tunnel
- **Purpose**: Mesh networking for containers

## Traffic Flow

1. **Client Device** → WireGuard Gateway (Container 100, `internal_100`)
2. **OpenFlow Rule**: `internal_100` → `warp0` (WARP tunnel)
3. **WARP Tunnel** → XRay Client (Container 101, `internal_101`)
4. **XRay Client** → VPS XRay Server (via WARP)
5. **VPS XRay Server** → Internet

**Return Path**:
1. Internet → VPS XRay Server
2. VPS → WARP → XRay Client (`internal_101`)
3. OpenFlow Rule: `internal_101` → `warp0` → `internal_100`
4. WireGuard Gateway → Client Device

**Mesh Traffic**:
1. Function request (e.g., `vector_db`) → OpenFlow routing
2. OpenFlow Rule: Function match → Target socket (e.g., `internal_200`)
3. Container receives traffic via socket networking

## Security Features

1. **Security Flows** (Level 1):
   - Drop invalid packets
   - Rate limiting
   - Connection tracking

2. **Pattern Hiding** (Level 2):
   - TTL normalization
   - Packet padding
   - Timing randomization

3. **Advanced Obfuscation** (Level 3):
   - Traffic morphing
   - Protocol mimicry
   - Decoy traffic

## Benefits

1. **Privacy**: Multi-hop tunnel with obfuscation
2. **Performance**: Socket networking (no veth overhead)
3. **Flexibility**: Function-based routing
4. **Scalability**: One Netmaker interface per node
5. **Security**: OpenFlow-based security flows
6. **Isolation**: Separate privacy and mesh networks on same bridge

