# Simple Socket Network Architecture

## Key Principles

1. **Each server has ONE OVS bridge** (mesh - socket network)
2. **oo1424oo**: Hosts privacy tunnel containers
3. **VPS (DO droplet)**: Hosts op-dbus management containers
4. **Netmaker** connects both socket networks

## Architecture Diagram

### oo1424oo (Privacy Tunnel Host)

```
┌─────────────────────────────────────────────────────────────┐
│  oo1424oo Host                                              │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  mesh - Socket Network                              │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ nm-privacy (Netmaker tun)                       │   │
│  │  ├─ warp0 (WARP tunnel via wg-quick PostUp)        │   │
│  │  ├─ veth100 (Gateway container)                     │   │
│  │  └─ veth102 (Xray container)                        │   │
│  │                                                      │   │
│  │  Function: Privacy tunnel for user traffic          │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### VPS / DO Droplet (Management Host)

```
┌─────────────────────────────────────────────────────────────┐
│  VPS / DO Droplet                                           │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  mesh - Socket Network                              │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ nm-server (Netmaker server or client)          │   │
│  │  ├─ veth200 (MCP server container)                 │   │
│  │  ├─ veth201 (op-dbus API container)                │   │
│  │  ├─ veth202 (Xray server for privacy ingress)      │   │
│  │  └─ veth203+ (other distributed services)          │   │
│  │                                                      │   │
│  │  Function: Management plane + ingress               │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Network Topology

```
                         Internet
                             │
                    ┌────────▼────────┐
                    │  VPS (Public)   │
                    │                 │
                    │  ┌───────────┐  │
                    │  │ mesh      │  │
                    │  │ - MCP     │  │
                    │  │ - API     │  │
                    │  │ - Xray    │  │
                    │  │ - nm      │  │
                    │  └─────┬─────┘  │
                    └────────┼────────┘
                             │
                      Netmaker Mesh
                      (WireGuard)
                             │
                    ┌────────▼────────┐
                    │  oo1424oo       │
                    │  (Behind NAT)   │
                    │                 │
                    │  ┌───────────┐  │
                    │  │ mesh      │  │
                    │  │ - Gateway │  │
                    │  │ - Xray    │  │
                    │  │ - WARP    │  │
                    │  │ - nm      │  │
                    │  └───────────┘  │
                    └─────────────────┘

Socket networks connected via Netmaker mesh
Each host: ONE OVS bridge (mesh)
```

## Traffic Flows

### Privacy Tunnel Traffic (External User → WARP Exit)

```
External User
  ↓
VPS public IP (Xray/SOCKS5 ingress on veth202)
  ↓
VPS mesh socket network
  ↓
nm-server (Netmaker)
  ↓
Netmaker mesh (WireGuard tunnel)
  ↓
oo1424oo nm-privacy
  ↓
oo1424oo mesh socket network
  ↓
OpenFlow: route to veth102 (Xray container)
  ↓
Xray container processes
  ↓
OpenFlow: veth102 → warp0
  ↓
warp0 (WARP tunnel)
  ↓
Cloudflare exit
  ↓
Internet
```

### Management Traffic (Admin → op-dbus API)

```
Admin laptop
  ↓
VPS public IP
  ↓
VPS mesh socket network
  ↓
OpenFlow: route to veth201 (op-dbus API)
  ↓
op-dbus API container
```

### Cross-Socket Communication (MCP on VPS → Service on oo1424oo)

```
MCP container (veth200 on VPS mesh)
  ↓
VPS mesh socket network
  ↓
nm-server (Netmaker)
  ↓
Netmaker mesh
  ↓
oo1424oo nm-privacy
  ↓
oo1424oo mesh socket network
  ↓
Target container (if exists on oo1424oo)
```

## OpenFlow Rules

### oo1424oo mesh (Privacy Tunnel Socket Network)

```bash
# Privacy traffic from netmaker → Xray client
priority=100,in_port=nm-privacy,actions=output:veth102

# Xray client → WARP exit
priority=100,in_port=veth102,actions=output:warp0

# WARP return → Xray
priority=100,in_port=warp0,actions=output:veth102

# Gateway → WARP (if VPN traffic needs routing)
priority=90,in_port=veth100,actions=output:warp0

# Response back to netmaker
priority=80,in_port=veth102,actions=output:nm-privacy
priority=80,in_port=warp0,actions=output:nm-privacy

# Default: normal switching
priority=10,actions=normal
```

### VPS mesh (Management Socket Network)

```bash
# Ingress traffic to Xray server (privacy tunnel ingress)
priority=100,tcp,tp_dst=443,actions=output:veth202
priority=100,tcp,tp_dst=8443,actions=output:veth202

# Xray server → netmaker (forward to oo1424oo)
priority=100,in_port=veth202,actions=output:nm-server

# MCP traffic
priority=90,tcp,tp_dst=9573,actions=output:veth200

# op-dbus API traffic
priority=90,tcp,tp_dst=9574,actions=output:veth201

# Response from containers
priority=80,in_port=veth200,actions=normal
priority=80,in_port=veth201,actions=normal
priority=80,in_port=veth202,actions=normal

# Traffic from netmaker (from oo1424oo)
priority=80,in_port=nm-server,actions=normal

# Default: normal switching
priority=10,actions=normal
```

## NixOS Configurations

### oo1424oo Configuration

```nix
# nix/oo1424oo-simple.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";
  };

  networking = {
    hostName = "oo1424oo";
    firewall = {
      enable = true;
      allowedUDPPorts = [ 51820 51821 ];  # Gateway VPN, Netmaker
      allowedTCPPorts = [ 22 8006 ];  # SSH, Proxmox
      trustedInterfaces = [ "mesh" "nm-privacy" ];
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
      # Netmaker client
      netmaker = {
        mode = "client";
        network = "privacy-mesh";
        interface = "nm-privacy";
        bridge = "mesh";  # ONE bridge
        server = "https://vps-public-ip:8081";
      };

      # WARP tunnel
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
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

      # OpenFlow
      openflow = {
        bridges = {
          mesh = {
            flows = [
              # Privacy traffic from netmaker → Xray
              "priority=100,in_port=nm-privacy,actions=output:veth102"

              # Xray → WARP exit
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return → Xray
              "priority=100,in_port=warp0,actions=output:veth102"

              # Gateway → WARP
              "priority=90,in_port=veth100,actions=output:warp0"

              # Response back to netmaker
              "priority=80,in_port=veth102,actions=output:nm-privacy"
              "priority=80,in_port=warp0,actions=output:nm-privacy"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on mesh
      lxc = {
        containers = [
          {
            id = "100";
            veth = "veth100";
            bridge = "mesh";
            running = true;
            properties = {
              name = "gateway";
              ipv4_address = "10.0.0.100/24";
              template = "ubuntu-22.04";
            };
          }
          {
            id = "102";
            veth = "veth102";
            bridge = "mesh";
            running = true;
            properties = {
              name = "xray-client";
              ipv4_address = "10.0.0.102/24";
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

### VPS Configuration

```nix
# nix/vps-simple.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  boot.loader.grub = {
    enable = true;
    device = "/dev/vda";  # DigitalOcean virtual disk
  };

  networking = {
    hostName = "vps-gateway";
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22        # SSH
        443       # Xray VLESS
        8443      # Xray VMess
        8081      # Netmaker API
        8443      # Netmaker UI
        9573      # MCP
        9574      # op-dbus API
      ];
      allowedUDPPorts = [
        51821     # Netmaker WireGuard
      ];
      trustedInterfaces = [ "mesh" ];
    };
  };

  services.op-dbus = {
    enable = true;
    mode = "standalone";

    stateConfig = {
      # Netmaker server
      netmaker = {
        mode = "server";
        network = "privacy-mesh";
        interface = "nm-server";
        bridge = "mesh";  # ONE bridge
        listen_port = 51821;
        api_endpoint = "https://<vps-public-ip>:8081";
      };

      # OpenFlow
      openflow = {
        bridges = {
          mesh = {
            flows = [
              # Ingress to Xray server
              "priority=100,tcp,tp_dst=443,actions=output:veth202"
              "priority=100,tcp,tp_dst=8443,actions=output:veth202"

              # Xray → netmaker (forward to oo1424oo)
              "priority=100,in_port=veth202,actions=output:nm-server"

              # MCP traffic
              "priority=90,tcp,tp_dst=9573,actions=output:veth200"

              # op-dbus API traffic
              "priority=90,tcp,tp_dst=9574,actions=output:veth201"

              # Container responses
              "priority=80,in_port=veth200,actions=normal"
              "priority=80,in_port=veth201,actions=normal"
              "priority=80,in_port=veth202,actions=normal"

              # Netmaker traffic
              "priority=80,in_port=nm-server,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Containers on mesh
      lxc = {
        containers = [
          {
            id = "200";
            veth = "veth200";
            bridge = "mesh";
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.1.0.200/24";
              template = "alpine-3.19";
              services = [
                { name = "mcp"; protocol = "tcp"; port = 9573; }
              ];
            };
          }
          {
            id = "201";
            veth = "veth201";
            bridge = "mesh";
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.1.0.201/24";
              template = "alpine-3.19";
              services = [
                { name = "api"; protocol = "tcp"; port = 9574; }
              ];
            };
          }
          {
            id = "202";
            veth = "veth202";
            bridge = "mesh";
            running = true;
            properties = {
              name = "xray-server";
              ipv4_address = "10.1.0.202/24";
              template = "alpine-3.19";
              services = [
                { name = "xray-vless"; protocol = "tcp"; port = 443; }
                { name = "xray-vmess"; protocol = "tcp"; port = 8443; }
              ];
            };
          }
        ];
      };

      systemd = {
        units = {
          "openvswitch.service" = { enabled = true; active_state = "active"; };
          "netmaker.service" = { enabled = true; active_state = "active"; };
        };
      };

      packagekit = {
        packages = {
          "wireguard-tools" = { ensure = "installed"; };
          "netmaker" = { ensure = "installed"; };
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
    vim git htop curl wget wireguard-tools netmaker
  ];

  system.stateVersion = "25.05";
}
```

## Summary

**Simple Architecture:**
- **ONE bridge per host** (mesh - socket network)
- **oo1424oo**: Privacy tunnel containers (Gateway, Xray, WARP)
- **VPS**: Management containers (MCP, op-dbus API, Xray server ingress)
- **Netmaker**: Connects both socket networks across NAT

**Key Benefits:**
- Simple: ONE bridge per host
- Clean separation: Privacy tunnel on oo1424oo, management on VPS
- NAT traversal: Netmaker allows oo1424oo to receive traffic despite being behind NAT
- Scalable: Add more nodes with ONE bridge each
