# Correct Socket Network Architecture

## Overview

The op-dbus deployment implements a **distributed socket network** that spans multiple Proxmox hosts using Netmaker (WireGuard) as the underlay mesh.

The socket network supports TWO main use cases:
1. **Privacy router** - External users → VPN/proxy → WARP exit
2. **Distributed services** - Containers across multiple hosts (MCP, Vector DB, etc.)

## Network Architecture

### Per-Host Design

```
┌─────────────────────────────────────────────────────────────┐
│  Host (NixOS/Proxmox)                                       │
│                                                             │
│  Netmaker WireGuard (on host):                             │
│    └─ nm-privacy (tun device)                              │
│       - Joins Netmaker mesh                                │
│       - Connects to other hosts                            │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  mesh bridge (OVS) - Isolation Layer                  │ │
│  │                                                        │ │
│  │  ├─ nm-privacy (enslaved tun device)                  │ │
│  │  └─ to-socket (inter-bridge veth → vmbr0)            │ │
│  │                                                        │ │
│  │  OpenFlow Rules:                                       │ │
│  │    - nm-privacy ↔ to-socket (controlled forwarding)   │ │
│  │    - Default: DROP (isolation)                         │ │
│  └───────────────────────────────────────────────────────┘ │
│                          ↓ (inter-bridge veth pair)        │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  vmbr0 - Socket Network Bridge (OVS)                  │ │
│  │                                                        │ │
│  │  Ports:                                                │ │
│  │  ├─ from-mesh (inter-bridge veth from mesh bridge)   │ │
│  │  ├─ warp0 (WARP tunnel, added via wg-quick PostUp)   │ │
│  │  ├─ veth100 (Gateway container - privacy router)      │ │
│  │  ├─ veth102 (Xray container - privacy router)         │ │
│  │  ├─ veth200 (MCP server - distributed service)        │ │
│  │  └─ veth201 (op-dbus API - distributed service)       │ │
│  │                                                        │ │
│  │  Traffic Types:                                        │ │
│  │  1. Privacy Router:                                    │ │
│  │     External → from-mesh → veth100/102 → warp0 → exit │ │
│  │  2. Distributed Services:                              │ │
│  │     veth200 ↔ from-mesh ↔ containers on other hosts  │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Full Network Topology

```
                     ┌──────────────────────┐
                     │  DO Droplet          │
                     │  - Netmaker Server   │
                     │  - SOCKS5 Ingress    │
                     └──────────┬───────────┘
                                │
                     Netmaker WireGuard Mesh
                     (nm-privacy on all hosts)
                                │
               ┌────────────────┼────────────────┐
               │                                 │
     ┌─────────▼──────────┐         ┌──────────▼─────────┐
     │  oo1424oo          │         │  Proxmox Node 2    │
     │                    │         │                    │
     │  ┌──────────────┐  │         │  ┌──────────────┐  │
     │  │ mesh bridge  │  │         │  │ mesh bridge  │  │
     │  │ - nm-privacy │  │         │  │ - nm-privacy │  │
     │  └──────┬───────┘  │         │  └──────┬───────┘  │
     │         │           │         │         │           │
     │  ┌──────▼───────┐  │         │  ┌──────▼───────┐  │
     │  │ vmbr0        │  │         │  │ vmbr0        │  │
     │  │              │  │         │  │              │  │
     │  │ Privacy:     │  │         │  │ Services:    │  │
     │  │ - warp0      │  │         │  │ - veth103    │  │
     │  │ - veth100    │  │         │  │   (Vector DB)│  │
     │  │ - veth102    │  │         │  │ - veth104    │  │
     │  │              │  │         │  │   (Redis)    │  │
     │  │ Services:    │  │         │  │              │  │
     │  │ - veth200    │  │         │  │              │  │
     │  │ - veth201    │  │         │  │              │  │
     │  └──────────────┘  │         │  └──────────────┘  │
     └────────────────────┘         └────────────────────┘

     Socket Network: All vmbr0 bridges connected via Netmaker mesh
```

## Bridge Architecture Details

### mesh bridge (Isolation Layer)

**Purpose**: Provide controlled access to Netmaker mesh, enforce security isolation

**Ports**:
- `nm-privacy`: Netmaker WireGuard interface (tun device) enslaved to bridge
- `to-socket`: Inter-bridge veth peer (connects to vmbr0's `from-mesh`)

**OpenFlow Rules**:
```bash
# Allow bidirectional traffic between mesh and socket network
priority=100,in_port=nm-privacy,actions=output:to-socket
priority=100,in_port=to-socket,actions=output:nm-privacy

# Default: DROP (strict isolation)
priority=1,actions=drop
```

**Why separate bridge?**
- Security boundary between external mesh and internal socket network
- All mesh traffic must pass through OpenFlow rules
- Prevents unauthorized access to socket network
- Enables traffic shaping and monitoring at mesh entry point

### vmbr0 (Socket Network)

**Purpose**: Container networking with local and cross-host routing

**Ports**:
- `from-mesh`: Inter-bridge veth peer (receives traffic from mesh bridge)
- `warp0`: WARP tunnel interface (privacy exit)
- `veth100-199`: Privacy router containers
- `veth200-299`: Distributed service containers

**OpenFlow Rules**:
```bash
# Privacy router traffic → WARP exit
priority=100,in_port=veth100,actions=output:warp0
priority=100,in_port=veth102,actions=output:warp0

# WARP return traffic (MAC learning)
priority=100,in_port=warp0,actions=learn(...),output:normal

# Cross-host traffic via mesh
priority=90,in_port=from-mesh,actions=normal

# Distributed services can communicate locally or via mesh
priority=80,in_port=veth200,actions=normal
priority=80,in_port=veth201,actions=normal

# Default: normal switching
priority=10,actions=normal
```

## Traffic Flow Examples

### 1. Privacy Router: External User → WARP Exit

```
External User
  ↓
DO Droplet SOCKS5 (1080)
  ↓
Netmaker Server
  ↓
[WireGuard Tunnel via nm-privacy]
  ↓
oo1424oo nm-privacy (tun device)
  ↓
mesh bridge (enslaves nm-privacy)
  ↓
OpenFlow: nm-privacy → to-socket
  ↓
Inter-bridge veth pair
  ↓
vmbr0 (from-mesh port)
  ↓
OpenFlow: from-mesh → warp0 (for privacy traffic)
  ↓
warp0 (WARP tunnel)
  ↓
Cloudflare Exit
  ↓
Internet
```

**This is privacy router traffic - comes in from external, forwarded to vmbr0 socket network**

### 2. Distributed Service: MCP on oo1424oo → Vector DB on node2

```
MCP container (veth200 on oo1424oo)
  ↓
vmbr0 on oo1424oo
  ↓
OpenFlow: veth200 → normal (routing table decides)
  ↓
Host routing: destination 10.1.0.103 is on node2
  ↓
from-mesh port (vmbr0)
  ↓
Inter-bridge veth
  ↓
mesh bridge → to-socket port
  ↓
OpenFlow: to-socket → nm-privacy
  ↓
nm-privacy (Netmaker WireGuard)
  ↓
[WireGuard Tunnel]
  ↓
node2 nm-privacy
  ↓
node2 mesh bridge
  ↓
node2 inter-bridge veth
  ↓
node2 vmbr0
  ↓
OpenFlow: routes to veth103
  ↓
Vector DB container (node2)
```

**This is multi-server socket network traffic**

### 3. Local Container Communication (Same Host)

```
Xray container (veth102)
  ↓
vmbr0
  ↓
OpenFlow: in_port=veth102 → warp0
  ↓
warp0 (WARP tunnel)
  ↓
Internet
```

**No mesh traversal - stays local on vmbr0**

## NixOS Configuration

### oo1424oo with Correct Architecture

```nix
# nix/oo1424oo-socket-mesh-correct.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  networking = {
    hostName = "oo1424oo";
    firewall = {
      enable = true;
      allowedUDPPorts = [ 51820 51821 ];  # VPN, Netmaker
      allowedTCPPorts = [ 22 8006 9573 9574 ];
      trustedInterfaces = [ "vmbr0" "mesh" ];
    };
  };

  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker client on HOST
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";  # tun device
        server = "https://netmaker-gateway:8081";
        # Will be enslaved by mesh bridge
        bridge = "mesh";
      };

      # Inter-bridge veth pair
      networking = {
        veth_pairs = {
          mesh_to_socket = {
            peer1 = { name = "to-socket"; bridge = "mesh"; };
            peer2 = { name = "from-mesh"; bridge = "vmbr0"; };
          };
        };
      };

      # WARP tunnel on host
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds warp0 to socket network
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

      # OpenFlow - TWO bridges
      openflow = {
        bridges = {
          # Mesh bridge - Isolation layer
          mesh = {
            flows = [
              # Netmaker ↔ socket network
              "priority=100,in_port=nm-privacy,actions=output:to-socket"
              "priority=100,in_port=to-socket,actions=output:nm-privacy"

              # Default: DROP
              "priority=1,actions=drop"
            ];
          };

          # Socket network - Privacy router + distributed services
          vmbr0 = {
            flows = [
              # Privacy router: external traffic → WARP
              "priority=100,in_port=from-mesh,actions=output:warp0"

              # Privacy containers → WARP
              "priority=100,in_port=veth100,actions=output:warp0"
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return (MAC learning)
              "priority=100,in_port=warp0,actions=learn(table=1,hard_timeout=300,priority=110,NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],output:NXM_OF_IN_PORT[]),output:normal"

              # Distributed services (local or cross-host)
              "priority=80,in_port=veth200,actions=normal"
              "priority=80,in_port=veth201,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on vmbr0
      lxc = {
        containers = [
          # Privacy Router Containers
          {
            id = "100";
            veth = "veth100";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "gateway";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
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
            };
          }

          # Distributed Service Containers
          {
            id = "200";
            veth = "veth200";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.0.0.200/24";
              gateway = "10.0.0.1";
            };
          }
          {
            id = "201";
            veth = "veth201";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.0.0.201/24";
              gateway = "10.0.0.1";
            };
          }
        ];
      };

      systemd = {
        units = {
          "openvswitch.service" = { enabled = true; active_state = "active"; };
          "wg-quick@wg0.service" = { enabled = true; active_state = "active"; };
          "wg-quick@warp0.service" = { enabled = true; active_state = "active"; };
        };
      };

      packagekit = {
        packages = {
          "lxc" = { ensure = "installed"; };
          "wireguard-tools" = { ensure = "installed"; };
          "netclient" = { ensure = "installed"; };
        };
      };
    };
  };

  system.stateVersion = "25.05";
}
```

## Key Points

1. **TWO OVS bridges**:
   - `mesh`: Isolation layer with nm-privacy enslaved
   - `vmbr0`: Socket network with containers

2. **ONE Netmaker interface** (nm-privacy):
   - Created on host as tun device
   - Enslaved by mesh OVS bridge
   - NOT a veth pair - it's a WireGuard tunnel interface

3. **Inter-bridge veth pair**:
   - Connects mesh bridge to vmbr0
   - Controlled via OpenFlow rules
   - Security boundary

4. **Socket network supports TWO traffic types**:
   - **Privacy router**: External ingress → forwarded to vmbr0 → WARP exit
   - **Distributed services**: Cross-host container communication via mesh

5. **All traffic on vmbr0**:
   - Privacy router containers (100-199)
   - Distributed service containers (200-299)
   - Both use same bridge, different routing via OpenFlow
