# Optimized Socket Network Architecture

## Resource Allocation

**VPS** (DigitalOcean - Limited resources, Public IP):
- Privacy router function
- Lightweight: Xray ingress + WARP exit
- Solves NAT traversal problem (direct public access)

**oo1424oo** (Home server - 32GB RAM, Multi-core):
- op-dbus management function
- Heavy computation: MCP, API, Vector DB, distributed services
- Can scale to many containers

## Architecture Diagram

### VPS (Privacy Router - Lightweight)

```
┌─────────────────────────────────────────────────────────────┐
│  VPS (Public IP - Lightweight)                              │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  socket - Socket Network                            │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ nm-server (Netmaker server)                     │   │
│  │  ├─ warp0 (WARP tunnel via wg-quick PostUp)        │   │
│  │  └─ veth102 (Xray server - ingress/egress)         │   │
│  │                                                      │   │
│  │  Function: Privacy router (user traffic)            │   │
│  │  - Receives user connections (public IP)            │   │
│  │  - Exits via WARP tunnel                            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### oo1424oo (Management - Heavy Computation)

```
┌─────────────────────────────────────────────────────────────┐
│  oo1424oo (Behind NAT - Powerful: 32GB RAM, Multi-core)    │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  socket - Socket Network                            │   │
│  │                                                      │   │
│  │  Ports:                                              │   │
│  │  ├─ nm-privacy (Netmaker client)                    │   │
│  │  ├─ veth200 (MCP server)                           │   │
│  │  ├─ veth201 (op-dbus API)                          │   │
│  │  ├─ veth202 (Vector DB)                            │   │
│  │  ├─ veth203 (Redis)                                │   │
│  │  └─ veth204+ (distributed services...)             │   │
│  │                                                      │   │
│  │  Function: Management + distributed services        │   │
│  │  - Heavy computational workloads                    │   │
│  │  - Can scale to many containers                     │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Network Topology

```
                      Internet
                         │
                    ┌────▼─────────────────┐
                    │  VPS (Public IP)     │
                    │                      │
                    │  User connects here  │
                    │  ↓                   │
                    │  Xray server         │
                    │  ↓                   │
                    │  WARP tunnel exit    │
                    │  ↓                   │
                    │  Internet            │
                    │                      │
                    │  Netmaker server     │
                    └──────────┬───────────┘
                               │
                    Netmaker Mesh (L3)
                               │
                    ┌──────────▼───────────┐
                    │  oo1424oo            │
                    │  (Behind Verizon NAT)│
                    │                      │
                    │  op-dbus:            │
                    │  - MCP server        │
                    │  - API server        │
                    │  - Vector DB         │
                    │  - Redis             │
                    │  - etc...            │
                    └──────────────────────┘

Privacy router on VPS (solves NAT problem)
Management on oo1424oo (uses powerful hardware)
```

## Traffic Flows

### Privacy Tunnel Traffic (Optimized - All on VPS)

```
External User
  ↓
VPS public IP (Xray server on veth102)
  ↓
VPS socket network
  ↓
OpenFlow: route to veth102
  ↓
Xray container processes traffic
  ↓
OpenFlow: veth102 → warp0
  ↓
warp0 (WARP tunnel on VPS)
  ↓
Cloudflare exit
  ↓
Internet
```

**Key: All privacy traffic handled on VPS, no NAT traversal needed!**

### Management Traffic (Admin → op-dbus API on oo1424oo)

```
Admin laptop
  ↓
VPS public IP (SSH tunnel or direct)
  ↓
VPS socket network
  ↓
nm-server (Netmaker)
  ↓
Netmaker mesh (WireGuard tunnel)
  ↓
oo1424oo nm-privacy
  ↓
oo1424oo socket network
  ↓
OpenFlow: route to veth201 (API container)
  ↓
op-dbus API container
```

### Distributed Service Communication (MCP → Vector DB)

```
MCP container (veth200 on oo1424oo)
  ↓
oo1424oo socket network
  ↓
OpenFlow: veth200 → veth202
  ↓
Vector DB container (veth202 on oo1424oo)
```

**Local communication on powerful home server**

## OpenFlow Rules

### VPS socket (Privacy Router)

```bash
# Ingress to Xray server (privacy tunnel)
priority=100,tcp,tp_dst=443,actions=output:veth102
priority=100,tcp,tp_dst=8443,actions=output:veth102

# Xray → WARP exit
priority=100,in_port=veth102,actions=output:warp0

# WARP return → Xray
priority=100,in_port=warp0,actions=output:veth102

# Netmaker traffic (to oo1424oo management)
priority=80,in_port=nm-server,actions=normal

# Default: normal switching
priority=10,actions=normal
```

### oo1424oo socket (Management)

```bash
# Traffic from netmaker (from VPS or other nodes)
priority=100,in_port=nm-privacy,actions=normal

# MCP traffic
priority=90,tcp,tp_dst=9573,actions=output:veth200

# op-dbus API traffic
priority=90,tcp,tp_dst=9574,actions=output:veth201

# Vector DB traffic
priority=90,tcp,tp_dst=6333,actions=output:veth202

# Redis traffic
priority=90,tcp,tp_dst=6379,actions=output:veth203

# Container responses
priority=80,in_port=veth200,actions=normal
priority=80,in_port=veth201,actions=normal
priority=80,in_port=veth202,actions=normal

# Default: normal switching
priority=10,actions=normal
```

## NixOS Configurations

### VPS Configuration (Privacy Router)

```nix
# nix/vps-privacy-router.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./module.nix
  ];

  boot.loader.grub = {
    enable = true;
    device = "/dev/vda";
  };

  networking = {
    hostName = "vps-privacy-router";
    firewall = {
      enable = true;
      allowedTCPPorts = [
        22        # SSH
        443       # Xray VLESS
        8443      # Xray VMess
        8081      # Netmaker API
      ];
      allowedUDPPorts = [
        51821     # Netmaker WireGuard
      ];
      trustedInterfaces = [ "socket" "nm-server" "warp0" ];
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
        bridge = "socket";
        listen_port = 51821;
        api_endpoint = "https://<vps-public-ip>:8081";
      };

      # WARP tunnel on VPS
      wireguard = {
        warp_tunnel = {
          interface = "warp0";
          enabled = true;
          post_up = "ovs-vsctl add-port socket warp0";
          pre_down = "ovs-vsctl del-port socket warp0";
        };
      };

      # OpenFlow
      openflow = {
        bridges = {
          socket = {
            flows = [
              # Ingress to Xray
              "priority=100,tcp,tp_dst=443,actions=output:veth102"
              "priority=100,tcp,tp_dst=8443,actions=output:veth102"

              # Xray → WARP
              "priority=100,in_port=veth102,actions=output:warp0"

              # WARP return → Xray
              "priority=100,in_port=warp0,actions=output:veth102"

              # Netmaker traffic
              "priority=80,in_port=nm-server,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Privacy router container
      lxc = {
        containers = [
          {
            id = "102";
            veth = "veth102";
            bridge = "socket";
            running = true;
            properties = {
              name = "xray-server";
              network_type = "veth";
              ipv4_address = "10.2.0.102/24";
              gateway = "10.2.0.1";
              template = "alpine-3.19";
              memory = 512;
              swap = 256;
              services = [
                { name = "xray-vless"; protocol = "tcp"; port = 443; exposed = true; }
                { name = "xray-vmess"; protocol = "tcp"; port = 8443; exposed = true; }
              ];
            };
          }
        ];
      };

      systemd = {
        units = {
          "openvswitch.service" = { enabled = true; active_state = "active"; };
          "wg-quick@warp0.service" = { enabled = true; active_state = "active"; };
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

### oo1424oo Configuration (Management)

```nix
# nix/oo1424oo-management.nix
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
      allowedUDPPorts = [ 51821 ];  # Netmaker
      allowedTCPPorts = [ 22 8006 ];  # SSH, Proxmox
      trustedInterfaces = [ "socket" "nm-privacy" ];
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
        bridge = "socket";
        server = "https://<vps-public-ip>:8081";
      };

      # OpenFlow
      openflow = {
        dynamic_routing = {
          enabled = true;
          service_discovery = true;
          auto_flows = true;
        };

        bridges = {
          socket = {
            flows = [
              # Traffic from netmaker
              "priority=100,in_port=nm-privacy,actions=normal"

              # MCP traffic
              "priority=90,tcp,tp_dst=9573,actions=output:veth200"

              # op-dbus API traffic
              "priority=90,tcp,tp_dst=9574,actions=output:veth201"

              # Vector DB traffic
              "priority=90,tcp,tp_dst=6333,actions=output:veth202"

              # Redis traffic
              "priority=90,tcp,tp_dst=6379,actions=output:veth203"

              # Container responses
              "priority=80,actions=normal"

              # Default: normal switching
              "priority=10,actions=normal"
            ];
          };
        };
      };

      # Management containers (scalable on powerful hardware)
      lxc = {
        containers = [
          {
            id = "200";
            veth = "veth200";
            bridge = "socket";
            running = true;
            properties = {
              name = "mcp-server";
              ipv4_address = "10.0.0.200/24";
              template = "alpine-3.19";
              memory = 1024;  # More resources available
              swap = 512;
            };
          }
          {
            id = "201";
            veth = "veth201";
            bridge = "socket";
            running = true;
            properties = {
              name = "op-dbus-api";
              ipv4_address = "10.0.0.201/24";
              template = "alpine-3.19";
              memory = 1024;
              swap = 512;
            };
          }
          {
            id = "202";
            veth = "veth202";
            bridge = "socket";
            running = true;
            properties = {
              name = "vector-db";
              ipv4_address = "10.0.0.202/24";
              template = "debian-12";
              memory = 4096;  # Vector DB needs memory
              swap = 2048;
            };
          }
          {
            id = "203";
            veth = "veth203";
            bridge = "socket";
            running = true;
            properties = {
              name = "redis";
              ipv4_address = "10.0.0.203/24";
              template = "alpine-3.19";
              memory = 2048;
              swap = 1024;
            };
          }
        ];
      };

      systemd = {
        units = {
          "openvswitch.service" = { enabled = true; active_state = "active"; };
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

## Benefits of This Architecture

1. **Solves NAT Problem**: VPS has public IP, receives user traffic directly
2. **Resource Optimization**:
   - VPS: Lightweight proxying (Xray + WARP)
   - oo1424oo: Heavy computation (32GB RAM, multi-core for distributed services)
3. **Cost Efficient**: Small VPS droplet for privacy router, powerful home server for workloads
4. **Scalable**: Can add many containers on oo1424oo without resource constraints
5. **Simple**: Each server has ONE socket network bridge

## Deployment Order

1. **Deploy VPS** (Privacy Router):
   ```bash
   nixos-rebuild switch -I nixos-config=/path/to/vps-privacy-router.nix
   ```

2. **Deploy oo1424oo** (Management):
   ```bash
   nixos-rebuild switch -I nixos-config=/path/to/oo1424oo-management.nix
   ```

3. **Verify Netmaker Mesh**:
   ```bash
   # On VPS
   wg show nm-server

   # On oo1424oo
   wg show nm-privacy
   ```

4. **Test Privacy Tunnel**:
   ```bash
   # Connect to VPS Xray server
   # Traffic should exit via WARP
   curl --socks5 <vps-ip>:1080 ifconfig.me
   ```

5. **Test Management Access**:
   ```bash
   # Access op-dbus API on oo1424oo via Netmaker mesh
   curl http://<oo1424oo-netmaker-ip>:9574/api/status
   ```
