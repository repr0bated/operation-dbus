# Dual Socket Network Architecture

## Overview

The op-dbus deployment uses **TWO separate socket networks** with strict isolation:

1. **Privacy Socket Network** - User traffic through privacy tunnel (Gateway → WARP → Internet)
2. **Management Socket Network** - Control plane (MCP servers, op-dbus API, monitoring)

Both networks span multiple hosts via Netmaker mesh, but remain logically isolated.

## Network Separation

```
┌─────────────────────────────────────────────────────────────────┐
│                    Netmaker Mesh Layer                          │
│  - Physical WireGuard overlay connecting all nodes             │
│  - Multiple logical networks on top                             │
└─────────────────────────────────────────────────────────────────┘
                          ↓           ↓
         ┌────────────────┴─────┬─────┴────────────────┐
         ↓                      ↓                       ↓
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Privacy Network │    │ Management Net  │    │  Other Services │
│   (vmbr0)       │    │   (vmbr1)       │    │   (vmbr2...)    │
│                 │    │                 │    │                 │
│ - Gateway       │    │ - MCP servers   │    │ - Vector DB     │
│ - WARP tunnel   │    │ - op-dbus API   │    │ - Redis         │
│ - Xray proxy    │    │ - Monitoring    │    │ - Custom apps   │
│ - User traffic  │    │ - Provisioning  │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Per-Node Architecture

### oo1424oo (Proxmox Host)

```
┌──────────────────────────────────────────────────────────────┐
│  Host (NixOS on Proxmox)                                     │
│                                                              │
│  Netmaker Interfaces:                                        │
│    ├─ nm-privacy (on mesh-privacy bridge)                   │
│    └─ nm-mgmt (on mesh-mgmt bridge)                         │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Privacy Socket Network (vmbr0)                         │ │
│  │                                                        │ │
│  │  ├─ warp0 (WARP tunnel, added via wg-quick PostUp)   │ │
│  │  ├─ veth100 (Gateway container)                       │ │
│  │  ├─ veth102 (Xray container)                          │ │
│  │  └─ from-privacy (inter-bridge veth from mesh)        │ │
│  │                                                        │ │
│  │  OpenFlow Rules:                                       │ │
│  │    - User traffic → WARP exit                          │ │
│  │    - Proxy traffic → WARP exit                         │ │
│  │    - NO access to management network                   │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Management Socket Network (vmbr1)                      │ │
│  │                                                        │ │
│  │  ├─ veth200 (MCP server container)                    │ │
│  │  ├─ veth201 (op-dbus API container)                   │ │
│  │  ├─ veth202 (Monitoring container)                    │ │
│  │  └─ from-mgmt (inter-bridge veth from mesh)           │ │
│  │                                                        │ │
│  │  OpenFlow Rules:                                       │ │
│  │    - MCP traffic → op-dbus API                         │ │
│  │    - Management traffic → internal only                │ │
│  │    - NO access to privacy network                      │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ Mesh Bridges (Isolation Layer)                         │ │
│  │                                                        │ │
│  │  mesh-privacy:                                         │ │
│  │    ├─ nm-privacy (Netmaker interface)                 │ │
│  │    └─ to-privacy (veth to vmbr0)                      │ │
│  │                                                        │ │
│  │  mesh-mgmt:                                            │ │
│  │    ├─ nm-mgmt (Netmaker interface)                    │ │
│  │    └─ to-mgmt (veth to vmbr1)                         │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## NixOS Configuration

### oo1424oo with Dual Networks

```nix
# nix/oo1424oo-dual-network.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  # Network
  networking = {
    hostName = "oo1424oo";
    firewall = {
      enable = true;
      allowedUDPPorts = [ 51820 51821 51822 ];  # Gateway, Netmaker privacy, Netmaker mgmt
      allowedTCPPorts = [ 22 8006 9573 9574 ];  # SSH, Proxmox, MCP

      # Trust management network only
      trustedInterfaces = [ "vmbr1" "mesh-mgmt" ];

      # Privacy network is NOT trusted by host
      # (traffic must go through OpenFlow rules)
    };
  };

  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker - TWO separate networks
      netmaker = {
        networks = [
          {
            name = "privacy-mesh";
            mode = "client";
            interface = "nm-privacy";
            bridge = "mesh-privacy";
            server = "https://netmaker-gateway:8081";
          }
          {
            name = "management-mesh";
            mode = "client";
            interface = "nm-mgmt";
            bridge = "mesh-mgmt";
            server = "https://netmaker-gateway:8082";
          }
        ];
      };

      # Inter-bridge veth pairs
      networking = {
        veth_pairs = {
          privacy_mesh_to_socket = {
            peer1 = { name = "to-privacy"; bridge = "mesh-privacy"; };
            peer2 = { name = "from-privacy"; bridge = "vmbr0"; };
          };
          mgmt_mesh_to_socket = {
            peer1 = { name = "to-mgmt"; bridge = "mesh-mgmt"; };
            peer2 = { name = "from-mgmt"; bridge = "vmbr1"; };
          };
        };
      };

      # WARP tunnel on host
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          post_up = "ovs-vsctl add-port vmbr0 warp0";
          pre_down = "ovs-vsctl del-port vmbr0 warp0";
        };

        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
        };
      };

      # OpenFlow - TWO SEPARATE bridge networks
      openflow = {
        bridges = {
          # Privacy mesh isolation
          "mesh-privacy" = {
            flows = [
              "priority=100,in_port=nm-privacy,actions=output:to-privacy"
              "priority=100,in_port=to-privacy,actions=output:nm-privacy"
              "priority=1,actions=drop"  # Default DROP
            ];
          };

          # Management mesh isolation
          "mesh-mgmt" = {
            flows = [
              "priority=100,in_port=nm-mgmt,actions=output:to-mgmt"
              "priority=100,in_port=to-mgmt,actions=output:nm-mgmt"
              "priority=1,actions=drop"  # Default DROP
            ];
          };

          # Privacy socket network (vmbr0)
          vmbr0 = {
            flows = [
              # VPN clients → Gateway
              "priority=100,udp,tp_dst=51820,actions=output:veth100"

              # Gateway → WARP
              "priority=100,in_port=veth100,actions=output:warp0"

              # Xray → WARP
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP → return traffic (learned flows)
              "priority=100,in_port=warp0,actions=learn(table=1,hard_timeout=300,priority=110,NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],output:NXM_OF_IN_PORT[]),output:normal"

              # Cross-node privacy traffic via mesh
              "priority=90,in_port=from-privacy,actions=output:warp0"

              # NO access to management network (isolation enforced)

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };

          # Management socket network (vmbr1)
          vmbr1 = {
            flows = [
              # MCP → op-dbus API
              "priority=100,tcp,tp_dst=9573,actions=output:veth201"
              "priority=100,tcp,tp_dst=9574,actions=output:veth201"

              # Cross-node management traffic via mesh
              "priority=90,in_port=from-mgmt,actions=normal"

              # NO access to privacy network (isolation enforced)

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on SEPARATE networks
      lxc = {
        containers = [
          # Privacy Network Containers (vmbr0)
          {
            id = "100";
            veth = "veth100";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "gateway";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              network = "privacy";  # Tag for network assignment
            };
          }
          {
            id = "102";
            veth = "veth102";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "xray";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              network = "privacy";
            };
          }

          # Management Network Containers (vmbr1)
          {
            id = "200";
            veth = "veth200";
            bridge = "vmbr1";
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.1.0.200/24";
              gateway = "10.1.0.1";
              network = "management";  # Tag for network assignment
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; exposed = true; }
              ];
            };
          }
          {
            id = "201";
            veth = "veth201";
            bridge = "vmbr1";
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.1.0.201/24";
              gateway = "10.1.0.1";
              network = "management";
              services = [
                { name = "api"; protocol = "tcp"; port = 9573; exposed = true; }
                { name = "api-alt"; protocol = "tcp"; port = 9574; exposed = true; }
              ];
            };
          }
        ];
      };
    };
  };

  system.stateVersion = "25.05";
}
```

## Traffic Flow Examples

### Privacy Traffic: SOCKS5 → WARP → Internet
```
User (anywhere)
  → DO droplet SOCKS5:1080
  → Netmaker privacy-mesh (nm-privacy on DO)
  → [WireGuard tunnel]
  → oo1424oo nm-privacy (mesh-privacy bridge)
  → Inter-bridge veth (to-privacy → from-privacy)
  → vmbr0 (privacy socket network)
  → OpenFlow: from-privacy → warp0
  → WARP tunnel
  → Cloudflare exit
  → Internet
```

### Management Traffic: MCP Query → op-dbus API
```
Admin laptop
  → DO droplet (management entry point)
  → Netmaker management-mesh (nm-mgmt on DO)
  → [WireGuard tunnel]
  → oo1424oo nm-mgmt (mesh-mgmt bridge)
  → Inter-bridge veth (to-mgmt → from-mgmt)
  → vmbr1 (management socket network)
  → OpenFlow: tcp,tp_dst=9573 → veth201
  → op-dbus API container
  → Process request, return response
```

### Isolation Test: Privacy → Management (BLOCKED)
```
Xray container (vmbr0)
  → Attempts connection to 10.1.0.201:9573
  → vmbr0 OpenFlow rules
  → NO FLOW MATCHES (privacy network has no route to management)
  → Default flow: normal switching (stays on vmbr0)
  → ARP lookup fails (10.1.0.0/24 not on vmbr0)
  → Connection refused
```

### Isolation Test: Management → Privacy (BLOCKED)
```
MCP container (vmbr1)
  → Attempts connection to 10.0.0.102:443 (Xray)
  → vmbr1 OpenFlow rules
  → NO FLOW MATCHES (management network has no route to privacy)
  → Default flow: normal switching (stays on vmbr1)
  → ARP lookup fails (10.0.0.0/24 not on vmbr1)
  → Connection refused
```

## Security Boundaries

### Network Isolation
- **Privacy network** (vmbr0): Handles user traffic, MUST NOT access management plane
- **Management network** (vmbr1): Controls infrastructure, MUST NOT access user traffic
- **Mesh bridges**: Enforce isolation via default DROP rules
- **OpenFlow**: No cross-network flows defined

### Why Two Networks?

1. **Security**: Management plane compromise doesn't expose user traffic
2. **Privacy**: User traffic compromise doesn't expose control plane
3. **Compliance**: Separation of concerns for audit trails
4. **Performance**: Traffic isolation prevents resource contention
5. **Flexibility**: Can scale each network independently

## Dynamic Container Assignment

When creating a new container, specify which network:

```bash
# Create container on privacy network
curl -X POST http://localhost:9573/api/containers/create \
  -H "Content-Type: application/json" \
  -d '{
    "name": "tor-relay",
    "network": "privacy",
    "services": [
      { "name": "tor-relay", "protocol": "tcp", "port": 9001 }
    ]
  }'

# Create container on management network
curl -X POST http://localhost:9573/api/containers/create \
  -H "Content-Type: application/json" \
  -d '{
    "name": "prometheus",
    "network": "management",
    "services": [
      { "name": "prometheus", "protocol": "tcp", "port": 9090 }
    ]
  }'
```

op-dbus will automatically:
1. Assign next available container ID on correct bridge (100-199 for privacy, 200-299 for management)
2. Create veth pair attached to correct bridge
3. Add OpenFlow rules for the container's services
4. Register service in appropriate network's service discovery

## Cross-Network Communication (When Needed)

In rare cases where legitimate cross-network communication is needed (e.g., management needs to monitor privacy network health):

```nix
# Add specific flow with explicit approval
openflow = {
  bridges = {
    vmbr1 = {
      flows = [
        # Allow management → privacy health checks ONLY
        "priority=150,in_port=veth202,tcp,tp_dst=8080,nw_dst=10.0.0.102,actions=output:from-mgmt"

        # ... other management flows
      ];
    };

    vmbr0 = {
      flows = [
        # Allow return traffic for health checks
        "priority=150,in_port=from-privacy,tcp,tp_src=8080,nw_src=10.0.0.102,actions=output:to-privacy"

        # ... other privacy flows
      ];
    };
  };
};
```

## Deployment Checklist

- [ ] Deploy Netmaker server on DO droplet (two networks: privacy-mesh, management-mesh)
- [ ] Join oo1424oo to both Netmaker networks
- [ ] Create mesh-privacy and mesh-mgmt bridges
- [ ] Create vmbr0 (privacy) and vmbr1 (management) socket networks
- [ ] Configure inter-bridge veth pairs
- [ ] Deploy OpenFlow isolation rules
- [ ] Deploy privacy containers (Gateway, WARP, Xray) on vmbr0
- [ ] Deploy management containers (MCP, API, monitoring) on vmbr1
- [ ] Test isolation: verify privacy → management is blocked
- [ ] Test isolation: verify management → privacy is blocked
- [ ] Test functionality: SOCKS5 proxy works through privacy network
- [ ] Test functionality: MCP queries work through management network
