# Privacy Router Tunnel - Complete Architecture

## Overview

Complete privacy router tunnel chain with socket networking, OpenFlow routing, and Netmaker mesh integration.

**Chain**: WireGuard Gateway (zero config) → wgcf WARP → XRay Client → (VPS) → XRay Server → Internet

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    OVS Bridge (ovsbr0)                      │
│                                                              │
│  ┌──────────────┐    ┌──────────┐    ┌──────────────┐     │
│  │ WireGuard    │───▶│  WARP    │───▶│ XRay Client  │     │
│  │ Gateway      │    │ Tunnel   │    │ (Container)  │     │
│  │ (Container)  │    │ (warp0)  │    │              │     │
│  │ internal_100 │    │          │    │ internal_101 │     │
│  └──────────────┘    └──────────┘    └──────────────┘     │
│         │                  │                  │            │
│         └──────────────────┴──────────────────┘            │
│                            │                                 │
│                    OpenFlow Privacy Flows                    │
│                            │                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Netmaker Mesh Interface                │   │
│  │              (nm-privacy, per node)                 │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│  ┌─────────────────────────────────────────────────────┐   │
│  │         Container Socket Network (Mesh)              │   │
│  │  - Vector DB (internal_200)                         │   │
│  │  - Bucket Storage (internal_201)                    │   │
│  │  - Other containers...                              │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│                    Function-based Routing                   │
│                    (by socket to Netmaker)                   │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
                    VPS XRay Server
                            │
                            ▼
                        Internet
```

## Key Components

### 1. Socket Networking (Separate from Container Networking)

**Privacy Socket Network:**
- `internal_100` - WireGuard Gateway container
- `internal_101` - XRay Client container
- Both on same OVS bridge, isolated from mesh

**Mesh Socket Network:**
- `internal_200` - Vector DB container
- `internal_201` - Bucket Storage container
- `internal_XXX` - Other containers
- All connected via Netmaker mesh

### 2. OpenFlow Privacy Flows (Rust Implementation)

Privacy flows route traffic through the tunnel chain:

```rust
// WireGuard → WARP
priority=100,in_port=internal_100,actions=output:warp0

// WARP → XRay
priority=100,in_port=warp0,actions=output:internal_101

// XRay → WARP (return)
priority=100,in_port=internal_101,actions=output:warp0

// WARP → WireGuard (return)
priority=100,in_port=warp0,actions=output:internal_100
```

### 3. Function-Based Routing to Sockets

Containers route by function to Netmaker mesh:

```rust
// Vector DB function → internal_200 → Netmaker mesh
priority=90,ip,nw_dst=100.104.70.10,actions=output:internal_200

// Bucket Storage function → internal_201 → Netmaker mesh
priority=90,ip,nw_dst=100.104.70.11,actions=output:internal_201
```

### 4. Netmaker Mesh (One Interface Per Node)

- One Netmaker interface (`nm-privacy`) per Proxmox node
- All nodes in same mesh network
- Mesh interface enslaved to OVS bridge
- Containers communicate via mesh

## Configuration Example

```json
{
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
        }
      ],
      "function_routing": [
        {
          "function": "vector_db",
          "target_socket": "internal_200",
          "match_fields": {"ip": "nw_dst=100.104.70.10"}
        },
        {
          "function": "bucket_storage",
          "target_socket": "internal_201",
          "match_fields": {"ip": "nw_dst=100.104.70.11"}
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
```

## Implementation Details

### Socket Networking in LXC Module

Socket networking is separate from container networking:

1. **Privacy Socket Network**: Containers 100-199 use `internal_XXX` ports for privacy tunnel
2. **Mesh Socket Network**: Containers 200+ use `internal_XXX` ports for Netmaker mesh

Both networks use OVS internal ports, but routing is different:
- Privacy sockets → OpenFlow privacy flows → WARP → Internet
- Mesh sockets → Function routing → Netmaker mesh → Other nodes

### OpenFlow Privacy Flows (Rust)

Privacy flows are implemented in Rust using the OpenFlow plugin:

```rust
// Privacy flow rule
PrivacyFlowRule {
    priority: 100,
    match_fields: {
        let mut m = HashMap::new();
        m.insert("in_port".to_string(), "internal_100".to_string());
        m
    },
    actions: vec!["output:warp0".to_string()],
    description: Some("WireGuard gateway → WARP tunnel".to_string()),
}
```

### Function-Based Routing

Containers route by function to specific sockets:

```rust
FunctionRoute {
    function: "vector_db".to_string(),
    target_socket: "internal_200".to_string(),
    match_fields: {
        let mut m = HashMap::new();
        m.insert("ip".to_string(), "nw_dst=100.104.70.10".to_string());
        m
    },
}
```

### Netmaker Mesh Integration

- One Netmaker interface per Proxmox node
- Interface name: `nm-privacy` (or `nm-{node_id}`)
- All interfaces on same OVS bridge
- Mesh network: `privacy-mesh`
- Containers communicate via mesh IPs (100.104.70.0/24)

## Traffic Flow

### Privacy Tunnel Flow

1. Client → WireGuard Gateway (`internal_100`)
2. OpenFlow: `internal_100` → `warp0`
3. WARP tunnel → XRay Client (`internal_101`)
4. OpenFlow: `internal_101` → `warp0` (return)
5. XRay Client → VPS XRay Server
6. VPS → Internet

### Mesh Container Flow

1. Container → Function route → `internal_200` (Vector DB)
2. OpenFlow: Function match → `internal_200`
3. `internal_200` → Netmaker mesh (`nm-privacy`)
4. Netmaker mesh → Other nodes
5. Other nodes → Target container

## Benefits

1. **Isolation**: Privacy tunnel separate from mesh networking
2. **Flexibility**: Function-based routing allows dynamic container placement
3. **Scalability**: One Netmaker interface per node, all on same bridge
4. **Performance**: Socket networking (no veth overhead)
5. **Security**: OpenFlow privacy flows provide traffic control

