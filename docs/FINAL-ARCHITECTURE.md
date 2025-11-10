# Final Correct Socket Network Architecture

## Key Principles

1. **vmbr0 bridge** - Uplink bridge with external connectivity
   - Has uplink interface (traditional internet) enslaved
   - Has nm-privacy (Netmaker) enslaved
   - Optionally: second Netmaker interface (if second public IP available)
   - NO containers directly attached

2. **mesh bridge** - Socket network where ALL containers live
   - Privacy tunnel containers (Xray, WARP, Gateway)
   - op-dbus containers (MCP, API, distributed services)
   - Connects to vmbr0 via inter-bridge veth pair
   - All containers communicate via socket network

3. **Two separate functions** on same socket network:
   - Privacy tunnel: User traffic routing through WARP
   - op-dbus management: MCP, API, distributed services

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  oo1424oo Host                                              │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  vmbr0 - Uplink Bridge (External Connectivity)      │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ eth0 (uplink to internet, enslaved)            │   │
│  │  ├─ nm-privacy (Netmaker tun, enslaved)            │   │
│  │  ├─ nm-mgmt (optional 2nd Netmaker, enslaved)      │   │
│  │  └─ to-mesh (inter-bridge veth → mesh)             │   │
│  │                                                      │   │
│  │  Purpose: Provides connectivity for socket network  │   │
│  └──────────────────┬───────────────────────────────────┘   │
│                     │ Inter-bridge veth                     │
│  ┌──────────────────▼───────────────────────────────────┐   │
│  │  mesh - Socket Network (ALL containers)             │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ from-uplink (inter-bridge veth from vmbr0)     │   │
│  │  ├─ warp0 (WARP tunnel via wg-quick PostUp)        │   │
│  │  │                                                   │   │
│  │  │  Privacy Tunnel Containers:                      │   │
│  │  ├─ veth100 (Gateway - WireGuard VPN server)       │   │
│  │  ├─ veth102 (Xray proxy)                           │   │
│  │  │                                                   │   │
│  │  │  op-dbus Management Containers:                  │   │
│  │  ├─ veth200 (MCP server)                           │   │
│  │  ├─ veth201 (op-dbus API)                          │   │
│  │  └─ veth202+ (Vector DB, Redis, other services)    │   │
│  │                                                      │   │
│  │  ALL containers communicate via socket network      │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Network Topology

```
                      Internet
                         │
                    ┌────▼─────┐
                    │ eth0     │ (uplink)
                    └────┬─────┘
                         │
                    ┌────▼─────────────────────┐
                    │  vmbr0 (uplink bridge)  │
                    │  - eth0 enslaved         │
                    │  - nm-privacy enslaved   │
                    └────┬─────────────────────┘
                         │ (inter-bridge veth)
                    ┌────▼──────────────────────────────┐
                    │  mesh (socket network)           │
                    │                                   │
                    │  Privacy Tunnel:                  │
                    │    veth100 (Gateway)              │
                    │    veth102 (Xray)                 │
                    │    warp0 (WARP exit)              │
                    │                                   │
                    │  op-dbus Management:              │
                    │    veth200 (MCP)                  │
                    │    veth201 (API)                  │
                    │    veth202+ (distributed svcs)    │
                    └───────────────────────────────────┘

All containers on mesh can communicate via socket network
External connectivity via vmbr0 (uplink + netmaker)
```

## Traffic Flows

### 1. Privacy Tunnel Traffic (External User → WARP Exit)

```
External User
  ↓
DO Droplet (SOCKS5/Xray ingress)
  ↓
Netmaker mesh
  ↓
oo1424oo nm-privacy (enslaved by vmbr0)
  ↓
vmbr0 uplink bridge
  ↓
OpenFlow: nm-privacy → to-mesh
  ↓
Inter-bridge veth pair
  ↓
mesh socket network (from-uplink port)
  ↓
OpenFlow: route to veth102 (Xray container)
  ↓
Xray container processes traffic
  ↓
OpenFlow: veth102 → warp0
  ↓
warp0 (WARP tunnel)
  ↓
Cloudflare exit
  ↓
Internet
```

### 2. op-dbus Management Traffic (MCP Query)

```
Admin laptop
  ↓
DO Droplet
  ↓
Netmaker mesh
  ↓
oo1424oo nm-privacy (enslaved by vmbr0)
  ↓
vmbr0 uplink bridge
  ↓
OpenFlow: nm-privacy → to-mesh
  ↓
mesh socket network
  ↓
OpenFlow: route to veth201 (API container)
  ↓
op-dbus API container
  ↓
Response back through same path
```

### 3. Container-to-Container Communication (Same Host)

```
MCP container (veth200 on mesh)
  ↓
mesh socket network
  ↓
OpenFlow: veth200 → veth201
  ↓
op-dbus API container (veth201 on mesh)
```

**Direct socket network communication, no uplink needed**

### 4. Cross-Host Container Communication

```
MCP container (mesh on oo1424oo)
  ↓
mesh socket network
  ↓
OpenFlow: route to from-uplink
  ↓
Inter-bridge veth
  ↓
vmbr0 uplink bridge
  ↓
nm-privacy (Netmaker)
  ↓
Netmaker mesh to node2
  ↓
node2 nm-privacy → vmbr0 → mesh
  ↓
Vector DB container (mesh on node2)
```

## OpenFlow Rules

### vmbr0 (Uplink Bridge)

```bash
# Netmaker traffic → socket network
priority=100,in_port=nm-privacy,actions=output:to-mesh

# Socket network → Netmaker (cross-host)
priority=100,in_port=to-mesh,actions=output:nm-privacy

# Uplink traffic → socket network
priority=90,in_port=eth0,actions=output:to-mesh

# Socket network → uplink (traditional internet)
priority=90,in_port=to-mesh,actions=output:eth0

# Default: normal switching
priority=10,actions=normal
```

### mesh (Socket Network)

```bash
# Privacy tunnel: Xray → WARP exit
priority=100,in_port=veth102,actions=output:warp0

# WARP return → Xray
priority=100,in_port=warp0,actions=output:veth102

# Gateway → WARP (if VPN traffic needs privacy exit)
priority=90,in_port=veth100,actions=output:warp0

# Ingress from uplink → route to appropriate container
priority=80,in_port=from-uplink,<service-match>,actions=output:veth102  # Privacy
priority=80,in_port=from-uplink,<service-match>,actions=output:veth200  # MCP
priority=80,in_port=from-uplink,<service-match>,actions=output:veth201  # API

# Container-to-container (local socket network)
priority=70,in_port=veth200,actions=normal
priority=70,in_port=veth201,actions=normal
priority=70,in_port=veth202,actions=normal

# Container needs external connectivity (cross-host or internet)
priority=60,in_port=veth200,actions=output:from-uplink
priority=60,in_port=veth201,actions=output:from-uplink

# Default: normal switching for socket network
priority=10,actions=normal
```

## Optional: Two Netmaker Interfaces

If you have a second public IP available, you can run TWO Netmaker interfaces:

```nix
netmaker = {
  networks = [
    {
      name = "privacy-mesh";
      interface = "nm-privacy";
      bridge = "vmbr0";  # Enslaved by uplink bridge
      server = "https://netmaker-gateway:8081";
    }
    {
      name = "management-mesh";
      interface = "nm-mgmt";
      bridge = "vmbr0";  # Also enslaved by uplink bridge
      server = "https://netmaker-gateway:8082";
    }
  ];
};
```

Then OpenFlow on vmbr0 can route:
- Privacy traffic via nm-privacy
- Management traffic via nm-mgmt

This provides logical separation at the mesh level while keeping all containers on the same socket network.

## NixOS Configuration

```nix
# nix/oo1424oo-socket-uplink.nix
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
      allowedUDPPorts = [ 51820 51821 ];
      allowedTCPPorts = [ 22 8006 9573 9574 ];
      trustedInterfaces = [ "mesh" "vmbr0" ];
    };
  };

  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Netmaker on host, enslaved by uplink bridge
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";
        bridge = "vmbr0";  # Uplink bridge
        server = "https://netmaker-gateway:8081";
      };

      # Optional: Second Netmaker interface
      # netmaker_mgmt = {
      #   mode = "client";
      #   network = "management-mesh";
      #   interface = "nm-mgmt";
      #   bridge = "vmbr0";
      #   server = "https://netmaker-gateway:8082";
      # };

      # Uplink interface enslaved by vmbr0
      networking = {
        # Enslave uplink interface
        enslaved_interfaces = {
          eth0 = {
            bridge = "vmbr0";
          };
        };

        # Inter-bridge connection: uplink → socket network
        veth_pairs = {
          uplink_to_socket = {
            peer1 = {
              name = "to-mesh";
              bridge = "vmbr0";
            };
            peer2 = {
              name = "from-uplink";
              bridge = "mesh";
            };
          };
        };
      };

      # WARP tunnel on host, added to socket network
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds to socket network (mesh)
          post_up = "ovs-vsctl add-port mesh warp0";
          pre_down = "ovs-vsctl del-port mesh warp0";
        };

        vpn_server = {
          interface = "wg0";
          listen_port = 51820;
          ip_pool = "10.8.0.0/24";
          auto_provision = true;
        };
      };

      # OpenFlow rules
      openflow = {
        bridges = {
          # Uplink bridge - External connectivity
          vmbr0 = {
            flows = [
              # Netmaker → socket network
              "priority=100,in_port=nm-privacy,actions=output:to-mesh"

              # Socket network → Netmaker
              "priority=100,in_port=to-mesh,actions=output:nm-privacy"

              # Uplink → socket network
              "priority=90,in_port=eth0,actions=output:to-mesh"

              # Socket network → uplink
              "priority=90,in_port=to-mesh,actions=output:eth0"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };

          # Socket network - ALL containers
          mesh = {
            flows = [
              # Privacy tunnel: Xray → WARP
              "priority=100,in_port=veth102,actions=output:warp0"
              "priority=100,in_port=warp0,actions=output:veth102"

              # Gateway → WARP
              "priority=90,in_port=veth100,actions=output:warp0"

              # Ingress routing (from uplink)
              "priority=80,in_port=from-uplink,actions=normal"

              # Container-to-container (local)
              "priority=70,actions=normal"
            ];
          };
        };
      };

      # ALL containers on mesh (socket network)
      lxc = {
        containers = [
          # Privacy Tunnel Containers
          {
            id = "100";
            veth = "veth100";
            bridge = "mesh";  # Socket network
            running = true;
            properties = {
              name = "gateway";
              ipv4_address = "10.0.0.100/24";
              gateway = "10.0.0.1";
              template = "ubuntu-22.04";
            };
          }
          {
            id = "102";
            veth = "veth102";
            bridge = "mesh";  # Socket network
            running = true;
            properties = {
              name = "xray";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
            };
          }

          # op-dbus Management Containers
          {
            id = "200";
            veth = "veth200";
            bridge = "mesh";  # Socket network
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.0.0.200/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
            };
          }
          {
            id = "201";
            veth = "veth201";
            bridge = "mesh";  # Socket network
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.0.0.201/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
            };
          }
        ];
      };

      systemd = {
        units = {
          "openvswitch.service" = { enabled = true; active_state = "active"; };
          "wg-quick@wg0.service" = { enabled = true; active_state = "active"; };
          "wg-quick@warp0.service" = { enabled = true; active_state = "active"; };
          "netclient.service" = { enabled = true; active_state = "active"; };
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

  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "prohibit-password";
      PasswordAuthentication = false;
    };
  };

  environment.systemPackages = with pkgs; [
    vim git htop tmux curl wget wireguard-tools
  ];

  system.stateVersion = "25.05";
}
```

## Summary

**Two OVS bridges:**
1. **vmbr0** - Uplink bridge
   - eth0 (uplink) enslaved
   - nm-privacy (Netmaker) enslaved
   - Optional: nm-mgmt (2nd Netmaker) enslaved
   - Inter-bridge veth to mesh

2. **mesh** - Socket network
   - ALL containers (privacy + op-dbus)
   - Inter-bridge veth from vmbr0
   - warp0 (WARP tunnel)

**Key insight:** Socket network (mesh) is where ALL containers live and communicate. Uplink bridge (vmbr0) provides external connectivity (internet + netmaker). Privacy tunnel and op-dbus are logically separate functions but share the same socket network infrastructure.
