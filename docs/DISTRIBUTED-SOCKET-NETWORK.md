# Distributed Socket Network Architecture

## Overview

The op-dbus privacy router implements a **distributed socket network** that spans multiple physical hosts using Netmaker mesh networking as the underlying transport layer.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Application Layer (Xray, WARP, Gateway, Vector DB, etc.)  │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Socket Network (OVS bridges + dynamic OpenFlow routing)    │
│  - Containers communicate via veth pairs                    │
│  - OpenFlow rules dynamically route based on service        │
│  - Service discovery updates flows automatically            │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Netmaker Mesh (WireGuard overlay)                          │
│  - Every node has nm0 interface on mesh bridge              │
│  - Defeats NAT/firewall restrictions                        │
│  - Enables cross-host container communication               │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Physical Network (Internet, VPS, home server)              │
└─────────────────────────────────────────────────────────────┘
```

## Network Topology

### DO Droplet (netmaker-gateway)
```
Public SOCKS5 (0.0.0.0:1080)
    ↓
Netmaker Server (nm-privacy on mesh bridge)
    ↓ (Mesh tunnel to oo1424oo)
Socket network routing
```

### oo1424oo (Proxmox host)
```
Netmaker Client (nm0 on mesh bridge)
    ↓ (Inter-bridge veth pair)
vmbr0 (socket network bridge)
    ├─ warp0 (WARP tunnel, via wg-quick PostUp)
    ├─ veth102 (Xray container)
    ├─ veth103 (Vector DB container) ← EXAMPLE
    ├─ veth104 (Any other service)
    └─ from-mesh (inter-bridge veth from mesh)
```

### Additional Proxmox Nodes (if added)
```
Each node:
    - Netmaker Client (nm0 on mesh bridge)
    - vmbr0 (socket network bridge)
    - Containers with veth pairs
    - OpenFlow rules for routing
```

## Smart OpenFlow Routing

### The Problem
When you dynamically create a container for a new function (e.g., vector database, Redis, custom service), OpenFlow needs to know:
1. Which port on the bridge is that container?
2. What traffic should route to it?
3. How to reach it from other nodes in the mesh?

### The Solution: Service Discovery + Dynamic Flow Updates

#### 1. Container Metadata
Each container registers itself via op-dbus API:
```json
{
  "container_id": "103",
  "veth": "veth103",
  "bridge": "vmbr0",
  "services": [
    {
      "name": "vector-db",
      "protocol": "tcp",
      "port": 6333,
      "exposed": true
    }
  ],
  "ipv4_address": "10.0.0.103/24"
}
```

#### 2. op-dbus Watches Container Events
- Listens on D-Bus for systemd container start/stop
- Monitors OVSDB for port additions
- Updates internal service registry

#### 3. Dynamic Flow Generation
When container 103 (vector-db) starts:
```bash
# op-dbus automatically adds flows:

# Traffic for vector-db → veth103
ovs-ofctl add-flow vmbr0 "priority=100,tcp,tp_dst=6333,actions=output:veth103"

# Return traffic from veth103 → based on source
ovs-ofctl add-flow vmbr0 "priority=100,in_port=veth103,actions=learn(
  table=1,
  hard_timeout=300,
  priority=110,
  NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],
  output:NXM_OF_IN_PORT[]
),output:normal"
```

#### 4. Cross-Node Discovery
When a container on **oo1424oo** needs to reach a container on **node2**:

```
1. Client queries op-dbus API: "Where is vector-db?"
2. op-dbus responds: "10.100.5.103 (node2 via Netmaker)"
3. Traffic flow:
   veth102 (Xray on oo1424oo)
   → vmbr0 OpenFlow rules
   → from-mesh (inter-bridge veth)
   → mesh bridge
   → nm0 (Netmaker interface)
   → [Netmaker mesh tunnel]
   → nm0 on node2
   → vmbr0 on node2
   → veth103 (vector-db on node2)
```

## Implementation in op-dbus

### State Manager Integration
```rust
// src/state/plugins/dynamic_routing.rs

pub struct DynamicRoutingPlugin {
    service_registry: Arc<RwLock<HashMap<String, ServiceInfo>>>,
    ovs_connection: OvsdbConnection,
}

impl DynamicRoutingPlugin {
    pub async fn on_container_start(&self, container: &ContainerInfo) {
        // 1. Register services
        for service in &container.services {
            self.service_registry.write().await.insert(
                service.name.clone(),
                ServiceInfo {
                    container_id: container.id.clone(),
                    veth: container.veth.clone(),
                    port: service.port,
                    protocol: service.protocol.clone(),
                    node: get_local_hostname(),
                }
            );
        }

        // 2. Add OpenFlow rules
        for service in &container.services {
            self.add_service_flows(&container.veth, service).await?;
        }

        // 3. Announce to mesh (via D-Bus signal)
        self.broadcast_service_announcement(container).await?;
    }

    async fn add_service_flows(&self, veth: &str, service: &Service) -> Result<()> {
        let flows = vec![
            // Inbound traffic to service
            format!(
                "priority=100,{},tp_dst={},actions=output:{}",
                service.protocol, service.port, veth
            ),

            // Return traffic with MAC learning
            format!(
                "priority=100,in_port={},actions=learn(table=1,hard_timeout=300,priority=110,\
                NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],output:NXM_OF_IN_PORT[]),output:normal",
                veth
            ),
        ];

        for flow in flows {
            self.ovs_connection.add_flow("vmbr0", &flow).await?;
        }

        Ok(())
    }
}
```

### NixOS Configuration
```nix
# Distributed socket network with Netmaker
services.op-dbus = {
  enable = true;
  mode = "full";

  stateConfig = {
    # Every node has Netmaker
    netmaker = {
      mode = "client";  # or "server" on DO droplet
      network = "privacy-mesh";
      interface = "nm0";
      bridge = "mesh";
      server = "https://netmaker-gateway:8081";
    };

    # Inter-bridge connection
    networking = {
      veth_pairs = {
        mesh_to_socket = {
          peer1 = { name = "to-socket"; bridge = "mesh"; };
          peer2 = { name = "from-mesh"; bridge = "vmbr0"; };
        };
      };
    };

    # Dynamic routing enabled
    openflow = {
      dynamic_routing = {
        enabled = true;
        service_discovery = true;
        auto_flows = true;
      };

      bridges = {
        # Mesh bridge - Netmaker only
        mesh = {
          flows = [
            "priority=100,in_port=nm0,actions=output:to-socket"
            "priority=100,in_port=to-socket,actions=output:nm0"
            "priority=1,actions=drop"
          ];
        };

        # Socket network - Dynamic flows + defaults
        vmbr0 = {
          flows = [
            # Static flows for WARP
            "priority=100,in_port=from-mesh,actions=output:warp0"
            "priority=100,in_port=warp0,actions=output:from-mesh"

            # Dynamic flows added by op-dbus for containers
            # (automatically managed, no manual config needed)

            # Default: normal switching
            "priority=10,actions=normal"
          ];
        };
      };
    };

    # Container template for dynamic creation
    lxc = {
      container_template = {
        template = "alpine-3.19";
        memory = 512;
        swap = 256;
        bridge = "vmbr0";
        network_type = "veth";
        features = {
          nesting = false;
        };
      };

      # Pre-defined containers
      containers = [
        {
          id = "102";
          veth = "veth102";
          properties = {
            name = "xray";
            ipv4_address = "10.0.0.102/24";
            services = [
              { name = "xray-vless"; protocol = "tcp"; port = 443; exposed = true; }
              { name = "xray-vmess"; protocol = "tcp"; port = 8443; exposed = true; }
            ];
          };
        }
      ];
    };
  };
};
```

## Dynamic Container Creation API

### Create New Container
```bash
# Via D-Bus
busctl call io.op_dbus.StateManager \
  /io/op_dbus/StateManager \
  io.op_dbus.StateManager \
  CreateContainer "s" '{
    "name": "vector-db",
    "services": [
      {
        "name": "vector-db",
        "protocol": "tcp",
        "port": 6333,
        "exposed": true
      }
    ]
  }'

# Via HTTP API
curl -X POST http://localhost:9573/api/containers/create \
  -H "Content-Type: application/json" \
  -d '{
    "name": "vector-db",
    "services": [
      {
        "name": "vector-db",
        "protocol": "tcp",
        "port": 6333,
        "exposed": true
      }
    ]
  }'
```

### Query Service Location
```bash
# Find where "vector-db" is running
busctl call io.op_dbus.StateManager \
  /io/op_dbus/StateManager \
  io.op_dbus.StateManager \
  QueryService "s" "vector-db"

# Response:
# {
#   "service": "vector-db",
#   "container_id": "103",
#   "node": "oo1424oo",
#   "ip": "10.0.0.103",
#   "port": 6333,
#   "protocol": "tcp"
# }
```

## Traffic Flow Examples

### Example 1: SOCKS5 Client → oo1424oo → Internet
```
Client (anywhere)
  → DO droplet SOCKS5 (1080)
  → Netmaker mesh (nm-privacy)
  → oo1424oo mesh bridge (nm0)
  → Inter-bridge veth (to-socket → from-mesh)
  → vmbr0 socket network
  → warp0 (WARP tunnel)
  → Cloudflare exit
  → Internet
```

### Example 2: Xray → Vector DB (cross-node)
```
Xray container (oo1424oo)
  → veth102 on vmbr0
  → OpenFlow: "tcp,tp_dst=6333 → from-mesh"
  → Inter-bridge veth (from-mesh → to-socket)
  → mesh bridge
  → nm0 (Netmaker)
  → [Netmaker mesh tunnel]
  → node2 nm0
  → node2 vmbr0
  → OpenFlow: "tcp,tp_dst=6333 → veth103"
  → veth103 (vector-db container)
```

### Example 3: Local Container Communication
```
Xray container (veth102)
  → vmbr0 OpenFlow
  → warp0 (same bridge, no mesh needed)
  → Internet
```

## Key Benefits

1. **Transparent cross-node communication** - Containers don't know if peer is local or remote
2. **NAT traversal** - Netmaker defeats firewall/router restrictions
3. **Dynamic scaling** - Add containers/nodes without manual flow updates
4. **Service discovery** - Query API to find any service in the mesh
5. **Isolation** - Mesh traffic separated from socket network, controlled via OpenFlow
6. **Privacy** - All traffic can exit via WARP with Cloudflare IPs

## Next Steps

To implement smart OVS routing:
1. Extend `StateManager` with service registry
2. Implement `DynamicRoutingPlugin`
3. Add D-Bus methods for container creation and service queries
4. Create container templates for common services
5. Test cross-node communication
