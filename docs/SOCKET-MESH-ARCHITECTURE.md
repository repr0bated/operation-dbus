# Socket Network Over WireGuard Mesh - Correct Architecture

## Simplified Architecture

The socket network is a **distributed overlay network** that spans multiple hosts using WireGuard (Netmaker) as the underlay transport.

### Key Principles

1. **WireGuard runs on HOST** (Proxmox), not in containers
2. **ONE socket network bridge** (vmbr0) per host
3. **Socket network extends over WireGuard mesh** naturally
4. **Containers attach via veth pairs** to their local bridge
5. **OpenFlow routes** traffic locally or via mesh automatically

## Per-Host Architecture

### oo1424oo (Proxmox Host)

```
┌─────────────────────────────────────────────────────────────┐
│  Host (NixOS/Proxmox)                                       │
│                                                             │
│  Underlay: Netmaker WireGuard Interface                    │
│    └─ nm-privacy (tun device, NOT veth)                    │
│       - Connects to DO droplet                             │
│       - Connects to other Proxmox nodes                    │
│       - Provides L3 mesh connectivity                      │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  vmbr0 - Socket Network Bridge (OVS)                  │ │
│  │                                                        │ │
│  │  ├─ warp0 (WARP tunnel, added via wg-quick PostUp)   │ │
│  │  ├─ veth100 (Gateway container)                       │ │
│  │  ├─ veth102 (Xray container)                          │ │
│  │  ├─ veth200 (MCP server container)                    │ │
│  │  └─ veth201 (op-dbus API container)                   │ │
│  │                                                        │ │
│  │  OpenFlow Rules:                                       │ │
│  │    - Local routing: veth102 → warp0                   │ │
│  │    - Cross-node: veth200 → (via nm-privacy mesh)      │ │
│  │    - Service discovery: dynamic flows                  │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  Routing: vmbr0 traffic routes through nm-privacy for      │
│           cross-node communication automatically           │
└─────────────────────────────────────────────────────────────┘
```

### DO Droplet (netmaker-gateway)

```
┌─────────────────────────────────────────────────────────────┐
│  Host (DigitalOcean Droplet)                                │
│                                                             │
│  Underlay: Netmaker Server                                 │
│    └─ nm-server (manages mesh network)                     │
│       - Public endpoint for clients                        │
│       - Routes traffic between mesh nodes                  │
│                                                             │
│  Services:                                                  │
│    ├─ Netmaker API (port 8081)                            │
│    ├─ Netmaker UI (port 8443)                             │
│    └─ SOCKS5 proxy (port 1080)                            │
│       - Routes through mesh to oo1424oo → WARP            │
└─────────────────────────────────────────────────────────────┘
```

### Additional Proxmox Node (if added)

```
┌─────────────────────────────────────────────────────────────┐
│  Host (Another Proxmox Server)                              │
│                                                             │
│  Underlay: Netmaker WireGuard Interface                    │
│    └─ nm-privacy (tun device)                              │
│       - Joins same mesh as oo1424oo                        │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  vmbr0 - Socket Network Bridge (OVS)                  │ │
│  │                                                        │ │
│  │  ├─ veth103 (Vector DB container)                     │ │
│  │  ├─ veth104 (Redis container)                         │ │
│  │  └─ veth105 (Custom service)                          │ │
│  │                                                        │ │
│  │  OpenFlow Rules:                                       │ │
│  │    - Local routing between local containers            │ │
│  │    - Cross-node routing via nm-privacy mesh            │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Network Topology

```
                    ┌─────────────────────┐
                    │  DO Droplet         │
                    │  - Netmaker Server  │
                    │  - SOCKS5 Proxy     │
                    │  nm-server          │
                    └──────────┬──────────┘
                               │
                    WireGuard Mesh (Netmaker)
                    (L3 connectivity)
                               │
              ┌────────────────┼────────────────┐
              │                                 │
    ┌─────────▼──────────┐         ┌──────────▼─────────┐
    │  oo1424oo          │         │  Proxmox Node 2    │
    │  nm-privacy        │◄────────┤  nm-privacy        │
    │                    │  Mesh   │                    │
    │  ┌──────────────┐  │         │  ┌──────────────┐  │
    │  │ vmbr0        │  │         │  │ vmbr0        │  │
    │  │ - warp0      │  │         │  │ - veth103    │  │
    │  │ - veth100    │  │         │  │ - veth104    │  │
    │  │ - veth102    │  │         │  │              │  │
    │  │ - veth200    │  │         │  │              │  │
    │  │ - veth201    │  │         │  │              │  │
    │  └──────────────┘  │         │  └──────────────┘  │
    └────────────────────┘         └────────────────────┘

    Socket Network: All vmbr0 bridges logically connected via nm-privacy mesh
```

## Traffic Flow Examples

### Local Container Communication (Same Host)
```
Xray container (veth102)
  → vmbr0 bridge
  → OpenFlow: "in_port=veth102,actions=output:warp0"
  → warp0 (WARP tunnel)
  → Internet

NO mesh traversal needed - stays on local bridge
```

### Cross-Node Container Communication
```
MCP server (oo1424oo, veth200)
  → vmbr0 bridge (oo1424oo)
  → OpenFlow: "tcp,tp_dst=6333 → route via mesh"
  → Host routing table: destination is on node2's network
  → nm-privacy (Netmaker WireGuard interface)
  → [WireGuard mesh tunnel]
  → nm-privacy on node2
  → vmbr0 bridge (node2)
  → OpenFlow: "tcp,tp_dst=6333,actions=output:veth103"
  → veth103 (Vector DB container on node2)

Socket network spans the mesh transparently
```

### SOCKS5 Proxy Through Mesh
```
External client
  → DO droplet SOCKS5:1080
  → Routing decision: exit via oo1424oo
  → nm-server (Netmaker)
  → [WireGuard mesh tunnel]
  → oo1424oo nm-privacy
  → Host routing → vmbr0 bridge
  → OpenFlow: "from external → warp0"
  → warp0 (WARP tunnel)
  → Cloudflare exit
  → Internet
```

## NixOS Configuration (Corrected)

### oo1424oo Configuration

```nix
# nix/oo1424oo-socket-mesh.nix
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
      allowedUDPPorts = [ 51820 51821 ];  # Gateway VPN, Netmaker
      allowedTCPPorts = [ 22 8006 9573 9574 ];  # SSH, Proxmox, MCP, API

      # Trust Netmaker interface and bridge
      trustedInterfaces = [ "vmbr0" "nm-privacy" ];
    };
  };

  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker client on HOST (not in container)
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";  # tun device, NOT veth
        server = "https://netmaker-gateway:8081";
        # NO bridge assignment - WireGuard runs on host directly
      };

      # WARP tunnel on host
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds warp0 to socket network bridge
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

      # Socket network routing
      networking = {
        # Enable IP forwarding for mesh routing
        ip_forward = true;

        # Routes for cross-node communication
        routes = {
          # Other nodes' container networks route via nm-privacy
          "10.1.0.0/24" = {  # node2's container network
            via = "nm-privacy";
            metric = 50;
          };
        };
      };

      # OpenFlow - ONE bridge, smart routing
      openflow = {
        dynamic_routing = {
          enabled = true;
          service_discovery = true;
          auto_flows = true;
        };

        bridges = {
          # Socket network bridge (ONE bridge for everything)
          vmbr0 = {
            flows = [
              # VPN clients → Gateway container
              "priority=100,udp,tp_dst=51820,actions=output:veth100"

              # Gateway → WARP
              "priority=100,in_port=veth100,actions=output:warp0"

              # Xray (privacy) → WARP
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP → return traffic (MAC learning)
              "priority=100,in_port=warp0,actions=learn(table=1,hard_timeout=300,priority=110,NXM_OF_ETH_DST[]=NXM_OF_ETH_SRC[],output:NXM_OF_IN_PORT[]),output:normal"

              # MCP/API containers → local or via mesh (handled by routing table)
              "priority=90,in_port=veth200,actions=normal"
              "priority=90,in_port=veth201,actions=normal"

              # Dynamic flows added by op-dbus for new containers

              # Default: normal switching
              # (routing table decides if local or via nm-privacy mesh)
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers - ALL on same bridge (vmbr0)
      lxc = {
        containers = [
          # Gateway
          {
            id = "100";
            veth = "veth100";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "gateway";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              template = "ubuntu-22.04";
            };
          }

          # Xray (privacy tunnel)
          {
            id = "102";
            veth = "veth102";
            bridge = "vmbr0";
            running = true;
            properties = {
              name = "xray";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              services = [
                { name = "xray-vless"; protocol = "tcp"; port = 443; }
              ];
            };
          }

          # MCP server (management)
          {
            id = "200";
            veth = "veth200";
            bridge = "vmbr0";  # SAME bridge!
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.0.0.200/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; }
              ];
            };
          }

          # op-dbus API (management)
          {
            id = "201";
            veth = "veth201";
            bridge = "vmbr0";  # SAME bridge!
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.0.0.201/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              services = [
                { name = "api"; protocol = "tcp"; port = 9573; }
              ];
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
          "netclient" = { ensure = "installed"; };  # Netmaker client
        };
      };
    };
  };

  system.stateVersion = "25.05";
}
```

## Key Differences from Incorrect Architecture

### ❌ WRONG (What I Had Before)
- Multiple mesh bridges (mesh-privacy, mesh-mgmt)
- Inter-bridge veth pairs (to-socket, from-mesh)
- Complex OpenFlow routing between bridges
- WireGuard in containers

### ✅ CORRECT (Simplified)
- ONE socket network bridge (vmbr0)
- WireGuard (Netmaker) on HOST as tun device
- Socket network naturally extends over WireGuard mesh via routing table
- OpenFlow handles local routing, routing table handles cross-node

## Network Isolation (If Needed)

If you want to isolate privacy traffic from management traffic, you do it with **OpenFlow rules and firewall rules**, NOT separate bridges:

```nix
openflow = {
  bridges = {
    vmbr0 = {
      flows = [
        # Privacy containers can ONLY talk to warp0
        "priority=100,in_port=veth100,nw_dst=10.0.0.200/24,actions=drop"  # Block Gateway → MCP
        "priority=100,in_port=veth102,nw_dst=10.0.0.200/24,actions=drop"  # Block Xray → MCP

        # Management containers can ONLY talk locally, not via WARP
        "priority=100,in_port=veth200,actions=local"   # MCP → host services only
        "priority=100,in_port=veth201,actions=local"   # API → host services only

        # ... normal flows
      ];
    };
  };
};
```

## Summary

- **One socket network bridge per host** (vmbr0)
- **WireGuard (Netmaker) on host** as tun device (nm-privacy)
- **Socket network spans mesh** via routing table automatically
- **Containers attach via veth pairs** to their local vmbr0
- **OpenFlow routes locally**, routing table routes across mesh
- **Simple, clean, no inter-bridge complexity**
