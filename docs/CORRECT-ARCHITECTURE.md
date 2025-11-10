# Correct Socket Network Architecture

## Key Principles

1. **mesh bridge receives ALL traffic** - Has nm-privacy (Netmaker) enslaved
2. **mesh forwards privacy traffic** to isolated vmbr0 bridge
3. **vmbr0 is isolated** - NO netmaker interface, privacy containers only
4. **Privacy traffic uses WARP tunnel** - Traditional internet (non-netmaker) to VPS
5. **Other traffic uses netmaker** - Stays on mesh, routes via netmaker to VPS

## Two Socket Networks

### 1. Privacy Socket Network (vmbr0)
- **Isolated** - No netmaker interface
- **Purpose**: Privacy tunnel for user traffic
- **Exit**: WARP tunnel over traditional internet (cloaked)
- **Containers**: Xray client, Gateway
- **Destination**: VPS Xray server (via WARP-cloaked traditional connection)

### 2. Management/Distributed Socket Network (mesh bridge area)
- **Connected to netmaker** via nm-privacy
- **Purpose**: op-dbus services, MCP, distributed containers
- **Exit**: Via netmaker mesh to VPS
- **Containers**: MCP server, op-dbus API, Vector DB, etc.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│  oo1424oo Host                                              │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  mesh bridge (OVS) - RECEIVES ALL TRAFFIC           │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ nm-privacy (Netmaker tun enslaved)              │   │
│  │  ├─ to-privacy (inter-bridge veth → vmbr0)         │   │
│  │  ├─ veth200 (MCP container)                         │   │
│  │  └─ veth201 (op-dbus API container)                 │   │
│  │                                                      │   │
│  │  Traffic Routing:                                    │   │
│  │  1. Privacy traffic → to-privacy (forward to vmbr0) │   │
│  │  2. Other traffic → nm-privacy (via netmaker)       │   │
│  └──────────────────┬───────────────────────────────────┘   │
│                     │ Inter-bridge veth                     │
│  ┌──────────────────▼───────────────────────────────────┐   │
│  │  vmbr0 (OVS) - ISOLATED PRIVACY NETWORK             │   │
│  │  NO netmaker interface!                              │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ from-mesh (inter-bridge veth from mesh)        │   │
│  │  ├─ warp0 (WARP tunnel via wg-quick PostUp)        │   │
│  │  ├─ veth100 (Gateway container)                     │   │
│  │  └─ veth102 (Xray client container)                 │   │
│  │                                                      │   │
│  │  Traffic Flow:                                       │   │
│  │  from-mesh → xray client → warp0 → traditional      │   │
│  │  internet (non-netmaker) → VPS xray server          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Traffic Flows

### Privacy Traffic Path

```
External User
  ↓
DO Droplet (SOCKS5 or Xray server)
  ↓
Netmaker mesh
  ↓
oo1424oo nm-privacy (enslaved by mesh bridge)
  ↓
mesh bridge (receives traffic, identifies as privacy)
  ↓
OpenFlow: forward to-privacy
  ↓
Inter-bridge veth pair
  ↓
vmbr0 from-mesh port (isolated network)
  ↓
OpenFlow: from-mesh → veth102 (xray client)
  ↓
Xray client container processes
  ↓
OpenFlow: veth102 → warp0
  ↓
warp0 (WARP tunnel) - TRADITIONAL INTERNET (non-netmaker)
  ↓
Cloaked in WARP, exits to internet
  ↓
VPS Xray server (traditional connection)
  ↓
Internet
```

**Key: Privacy traffic exits via WARP over traditional internet, NOT via netmaker**

### Management/Distributed Traffic Path

```
MCP container (veth200 on mesh bridge)
  ↓
mesh bridge
  ↓
OpenFlow: routes to nm-privacy
  ↓
nm-privacy (Netmaker)
  ↓
Netmaker mesh to VPS
  ↓
VPS receives via netmaker
```

**Key: Management traffic uses netmaker for transport**

### Cross-Node Distributed Service

```
MCP container (mesh bridge on oo1424oo)
  ↓
mesh bridge
  ↓
nm-privacy → netmaker mesh
  ↓
node2 nm-privacy
  ↓
node2 mesh bridge
  ↓
Vector DB container (on node2 mesh bridge)
```

**Key: Distributed services communicate via netmaker mesh**

## OpenFlow Rules

### mesh bridge (Receives ALL traffic)

```bash
# Privacy traffic → isolated vmbr0
# Match on source/dest indicating privacy tunnel traffic
priority=100,in_port=nm-privacy,<privacy-match>,actions=output:to-privacy

# Management containers → netmaker
priority=90,in_port=veth200,actions=output:nm-privacy
priority=90,in_port=veth201,actions=output:nm-privacy

# Return traffic from netmaker → management containers
priority=90,in_port=nm-privacy,actions=normal

# Return traffic from privacy network
priority=80,in_port=to-privacy,actions=output:nm-privacy

# Default: normal switching
priority=10,actions=normal
```

### vmbr0 (Isolated Privacy Network)

```bash
# Traffic from mesh → xray client
priority=100,in_port=from-mesh,actions=output:veth102

# Xray client → WARP tunnel
priority=100,in_port=veth102,actions=output:warp0

# WARP return → xray client
priority=100,in_port=warp0,actions=output:veth102

# Gateway container (if needed)
priority=90,in_port=veth100,actions=output:warp0

# Default: normal switching (but isolated, no mesh access)
priority=10,actions=normal
```

## NixOS Configuration

```nix
# nix/oo1424oo-dual-socket-correct.nix
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
      # Trust mesh bridge (has netmaker)
      trustedInterfaces = [ "mesh" "nm-privacy" ];
      # vmbr0 is NOT trusted (isolated privacy network)
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
      # Netmaker on host, enslaved by mesh bridge
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";  # tun device
        bridge = "mesh";  # Enslaved by mesh bridge
        server = "https://netmaker-gateway:8081";
      };

      # Inter-bridge connection: mesh → vmbr0 (privacy isolation)
      networking = {
        veth_pairs = {
          mesh_to_privacy = {
            peer1 = {
              name = "to-privacy";
              bridge = "mesh";
            };
            peer2 = {
              name = "from-mesh";
              bridge = "vmbr0";
            };
          };
        };
      };

      # WARP tunnel on host (privacy exit)
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          # PostUp adds to ISOLATED vmbr0 (not mesh)
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

      # OpenFlow - TWO bridges with different purposes
      openflow = {
        bridges = {
          # mesh bridge - Receives ALL traffic, routes appropriately
          mesh = {
            flows = [
              # Privacy traffic from netmaker → isolated vmbr0
              # TODO: Add proper matching for privacy traffic
              "priority=100,in_port=nm-privacy,actions=output:to-privacy"

              # Management containers → netmaker
              "priority=90,in_port=veth200,actions=output:nm-privacy"
              "priority=90,in_port=veth201,actions=output:nm-privacy"

              # Return from netmaker → management containers
              "priority=90,in_port=nm-privacy,actions=normal"

              # Return from privacy network → netmaker
              "priority=80,in_port=to-privacy,actions=output:nm-privacy"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };

          # vmbr0 - ISOLATED privacy network (NO netmaker access)
          vmbr0 = {
            flows = [
              # Privacy traffic from mesh → xray client
              "priority=100,in_port=from-mesh,actions=output:veth102"

              # Xray client → WARP tunnel (traditional internet)
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return → xray client
              "priority=100,in_port=warp0,actions=output:veth102"

              # Gateway (if needed) → WARP
              "priority=90,in_port=veth100,actions=output:warp0"

              # Default: normal switching
              # (but isolated, cannot reach mesh or netmaker)
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on DIFFERENT bridges
      lxc = {
        containers = [
          # Privacy network containers (vmbr0 - ISOLATED)
          {
            id = "100";
            veth = "veth100";
            bridge = "vmbr0";  # Isolated bridge
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
            bridge = "vmbr0";  # Isolated bridge
            running = true;
            properties = {
              name = "xray-client";
              ipv4_address = "10.0.0.102/24";
              gateway = "10.0.0.1";
              template = "alpine-3.19";
              services = [
                { name = "xray-client"; protocol = "tcp"; port = 8080; }
              ];
            };
          }

          # Management/distributed containers (mesh bridge - uses netmaker)
          {
            id = "200";
            veth = "veth200";
            bridge = "mesh";  # Connected to netmaker
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.1.0.200/24";
              gateway = "10.1.0.1";
              template = "alpine-3.19";
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; }
              ];
            };
          }
          {
            id = "201";
            veth = "veth201";
            bridge = "mesh";  # Connected to netmaker
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.1.0.201/24";
              gateway = "10.1.0.1";
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

**Two Socket Networks:**

1. **Privacy Network (vmbr0)**:
   - Isolated from netmaker
   - Receives privacy traffic forwarded from mesh bridge
   - Routes via WARP tunnel over traditional internet
   - Exits to VPS xray server (non-netmaker connection)
   - Cloaked in WARP for privacy

2. **Management Network (mesh bridge)**:
   - Has nm-privacy (Netmaker) enslaved
   - Receives ALL ingress traffic
   - Routes privacy traffic to vmbr0
   - Routes other traffic via netmaker
   - Hosts management containers (MCP, API, etc.)

**Key Architecture Decision:**
Privacy traffic is intentionally isolated from netmaker and exits via WARP-cloaked traditional internet connection to avoid netmaker metadata/correlation.
