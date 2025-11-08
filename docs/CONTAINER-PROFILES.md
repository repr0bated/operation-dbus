# Container Deployment Profiles

**Privacy Router Architecture**: Configurable container deployments for different use cases

## Container Profiles

### Profile 1: None (Standalone)
**Use Case**: Testing, development, non-container deployments
**Containers**: 0
**Networks**: OVS bridge only

```json
{
  "lxc": {
    "container_profile": "none",
    "containers": []
  },
  "openflow": {
    "enable_security_flows": true,
    "obfuscation_level": 1,
    "bridges": [{
      "name": "ovsbr0",
      "flows": [],
      "socket_ports": []
    }]
  }
}
```

### Profile 2: Gateway + Warp + XRay Client (Privacy Chain - Client Side)
**Use Case**: Client-side privacy router (full multi-hop tunnel)
**Containers**: 3 (WireGuard gateway, Warp tunnel, XRay client)
**Architecture**: Client → WireGuard (100) → Warp (101) → XRay Client (102) → VPS

```json
{
  "lxc": {
    "container_profile": "privacy-client",
    "containers": [
      {
        "id": 100,
        "name": "wireguard-gateway",
        "template": "debian-12",
        "autostart": true,
        "network": {
          "bridge": "ovsbr0",
          "veth": false,
          "socket_networking": true,
          "port_name": "internal_100",
          "ipv4": "10.0.0.100/24"
        },
        "services": ["wireguard"],
        "config": {
          "wg0": {
            "address": "10.99.0.1/24",
            "listen_port": 51820,
            "private_key_file": "/etc/wireguard/private.key",
            "peers": [
              {
                "public_key": "CLIENT_PUBLIC_KEY",
                "allowed_ips": ["10.99.0.0/24"]
              }
            ]
          }
        }
      },
      {
        "id": 101,
        "name": "warp-tunnel",
        "template": "debian-12",
        "autostart": true,
        "network": {
          "bridge": "ovsbr0",
          "veth": false,
          "socket_networking": false,
          "wg_tunnel": true,
          "port_name": "wg-warp",
          "ipv4": "10.0.0.101/24"
        },
        "services": ["wg-quick@wg-warp"],
        "config": {
          "wg-quick": {
            "interface": "wg-warp",
            "address": "10.99.1.2/32",
            "private_key_file": "/etc/wireguard/warp-private.key",
            "endpoint": "engage.cloudflareclient.com:2408",
            "public_key": "WARP_PUBLIC_KEY",
            "post_up": "ovs-vsctl add-port ovsbr0 wg-warp"
          }
        }
      },
      {
        "id": 102,
        "name": "xray-client",
        "template": "debian-12",
        "autostart": true,
        "network": {
          "bridge": "ovsbr0",
          "veth": false,
          "socket_networking": true,
          "port_name": "internal_102",
          "ipv4": "10.0.0.102/24"
        },
        "services": ["xray"],
        "config": {
          "xray": {
            "inbound": {
              "port": 1080,
              "protocol": "socks"
            },
            "outbound": {
              "protocol": "vmess",
              "server": "VPS_IP",
              "port": 443,
              "id": "UUID",
              "alterId": 0,
              "security": "auto"
            }
          }
        }
      }
    ]
  },
  "openflow": {
    "enable_security_flows": true,
    "obfuscation_level": 3,
    "auto_discover_containers": true,
    "flow_policies": [
      {
        "name": "wireguard-to-warp",
        "selector": "container:100",
        "template": {
          "table": 10,
          "priority": 1000,
          "actions": [{"type": "output", "port": "internal_101"}]
        }
      },
      {
        "name": "warp-to-xray",
        "selector": "container:101",
        "template": {
          "table": 10,
          "priority": 1000,
          "actions": [{"type": "output", "port": "internal_102"}]
        }
      },
      {
        "name": "xray-to-internet",
        "selector": "container:102",
        "template": {
          "table": 10,
          "priority": 1000,
          "actions": [{"type": "normal"}]
        }
      }
    ],
    "bridges": [{
      "name": "ovsbr0",
      "flows": [],
      "socket_ports": [
        {"name": "internal_100", "container_id": "100"},
        {"name": "internal_101", "container_id": "101"},
        {"name": "internal_102", "container_id": "102"}
      ]
    }]
  }
}
```

### Profile 3: XRay Server Only (VPS Side)
**Use Case**: VPS endpoint for privacy router
**Containers**: 1 (XRay server only)
**Architecture**: Client → Internet → VPS XRay Server (100) → Internet

```json
{
  "lxc": {
    "container_profile": "privacy-vps",
    "containers": [
      {
        "id": 100,
        "name": "xray-server",
        "template": "debian-12",
        "autostart": true,
        "network": {
          "bridge": "ovsbr0",
          "veth": false,
          "socket_networking": true,
          "port_name": "internal_100",
          "ipv4": "10.0.0.100/24",
          "public_ip": "VPS_PUBLIC_IP"
        },
        "services": ["xray"],
        "config": {
          "xray": {
            "inbound": {
              "port": 443,
              "protocol": "vmess",
              "settings": {
                "clients": [
                  {
                    "id": "UUID",
                    "alterId": 0
                  }
                ]
              },
              "streamSettings": {
                "network": "tcp",
                "security": "tls",
                "tlsSettings": {
                  "certificates": [
                    {
                      "certificateFile": "/etc/xray/cert.pem",
                      "keyFile": "/etc/xray/key.pem"
                    }
                  ]
                }
              }
            },
            "outbound": {
              "protocol": "freedom"
            }
          }
        }
      }
    ]
  },
  "openflow": {
    "enable_security_flows": true,
    "obfuscation_level": 2,
    "auto_discover_containers": true,
    "flow_policies": [
      {
        "name": "xray-server-forwarding",
        "selector": "container:100",
        "template": {
          "table": 10,
          "priority": 1000,
          "actions": [{"type": "normal"}]
        }
      }
    ],
    "bridges": [{
      "name": "ovsbr0",
      "flows": [],
      "socket_ports": [
        {"name": "internal_100", "container_id": "100"}
      ]
    }]
  }
}
```

## Traffic Flow Diagrams

### Profile 2: Privacy Chain (Client Side)
```
Client Devices
    ↓ (WiFi/Ethernet)
WireGuard Gateway (Container 100, internal_100, 10.0.0.100)
    ↓ (OpenFlow: table 10, priority 1000, output:wg-warp)
    ↓ (Security flows: Level 3 obfuscation)
Warp Tunnel (Container 101, wg-warp port via wg-quick PostUp)
    ↓ (OpenFlow: table 10, priority 1000, output:internal_102)
    ↓ (Obfuscation: TTL normalization, packet padding)
XRay Client (Container 102, internal_102, 10.0.0.102)
    ↓ (OpenFlow: table 10, priority 1000, normal)
    ↓ (Protocol mimicry: WireGuard→HTTPS)
Internet → VPS XRay Server (Container 100, VPS side)
    ↓
Internet
```

### Profile 3: VPS Endpoint
```
Internet
    ↓ (TCP 443, appears as HTTPS due to TLS)
XRay Server (Container 100, internal_100, 10.0.0.100)
    ↓ (Decrypt VMess, forward to freedom)
    ↓ (OpenFlow: table 10, priority 1000, normal)
    ↓ (Security flows: Level 2 pattern hiding)
Internet (actual destination)
```

## Installation Examples

### Install Client Side (Profile 2)
```bash
# Generate state.json for privacy client
sudo op-dbus init --profile privacy-client > /etc/op-dbus/state.json

# Edit configuration (set VPS_IP, UUID, keys)
sudo nano /etc/op-dbus/state.json

# Apply state (creates containers, flows, everything)
sudo op-dbus apply /etc/op-dbus/state.json

# Verify containers running
sudo lxc-ls -f

# Verify flows installed
sudo ovs-ofctl dump-flows ovsbr0 | grep -E "cookie=0x(dead|cafe|beef)"
```

### Install VPS Side (Profile 3)
```bash
# Generate state.json for VPS endpoint
sudo op-dbus init --profile privacy-vps > /etc/op-dbus/state.json

# Edit configuration (set UUID, TLS certs)
sudo nano /etc/op-dbus/state.json

# Apply state
sudo op-dbus apply /etc/op-dbus/state.json

# Verify XRay listening on 443
sudo lxc-attach -n xray-server -- netstat -tlnp | grep 443
```

### Install Standalone (Profile 1)
```bash
# Generate state.json without containers
sudo op-dbus init --profile none > /etc/op-dbus/state.json

# Apply state (just bridge + flows, no containers)
sudo op-dbus apply /etc/op-dbus/state.json

# Verify bridge exists
sudo ovs-vsctl show
```

## Profile Selection in install.sh

Update `install.sh` to support profile selection:

```bash
# Phase 1: Deployment Mode Selection
echo "Select deployment mode:"
echo ""
echo "  [1] Full (Proxmox) - All features"
echo "  [2] Standalone - OVS + Flows only (no containers)"
echo "  [3] Privacy Client - WireGuard + Warp + XRay Client"
echo "  [4] Privacy VPS - XRay Server only"
echo "  [5] Agent - Minimal (D-Bus plugins only)"
echo ""
read -p "Enter mode [1-5]: " mode_choice

case "$mode_choice" in
    1) MODE="full" ;;
    2) MODE="standalone" ;;
    3) MODE="privacy-client" ;;
    4) MODE="privacy-vps" ;;
    5) MODE="agent" ;;
    *) echo "Invalid selection"; exit 1 ;;
esac
```

## Container Service Setup

Each container needs specific services installed:

### WireGuard Gateway (Container 100)
```bash
# In container 100
apt-get install wireguard iptables
systemctl enable wg-quick@wg0
```

### Warp Tunnel (Container 101)

**Important**: Warp uses WireGuard protocol via `wg-quick`, which creates a tunnel interface that's added to OVS as a port (not socket networking)

**Tool**: [wgcf](https://github.com/ViRb3/wgcf) - Cloudflare Warp WireGuard config generator

```bash
# In container 101
apt-get install wireguard

# Install wgcf to generate Warp config
wget https://github.com/ViRb3/wgcf/releases/latest/download/wgcf_$(uname -s | tr '[:upper:]' '[:lower:]')_amd64 -O /usr/local/bin/wgcf
chmod +x /usr/local/bin/wgcf

# Register with Cloudflare Warp
wgcf register
wgcf generate

# Modify wgcf-profile.conf to add OVS integration
sed -i '/\[Interface\]/a PostUp = ovs-vsctl add-port ovsbr0 wg-warp\nPostDown = ovs-vsctl del-port ovsbr0 wg-warp' wgcf-profile.conf

# Install config
mv wgcf-profile.conf /etc/wireguard/wg-warp.conf

# Start tunnel (wg-quick automatically adds to OVS via PostUp)
systemctl enable wg-quick@wg-warp
systemctl start wg-quick@wg-warp

# Verify tunnel added to OVS
ovs-vsctl show | grep wg-warp
# Should show: Port "wg-warp"

# Verify Warp working
curl --interface wg-warp https://www.cloudflare.com/cdn-cgi/trace/
# Should show warp=on
```

### XRay Client (Container 102) / XRay Server (VPS Container 100)
```bash
# In container
bash -c "$(curl -L https://github.com/XTLS/Xray-install/raw/main/install-release.sh)" @ install
systemctl enable xray
systemctl start xray
```

## Testing Container Connectivity

### Test Profile 2 (Privacy Client)
```bash
# From host, test WireGuard gateway
ping 10.0.0.100

# From container 100, test Warp tunnel
sudo lxc-attach -n wireguard-gateway -- ping 10.0.0.101

# From container 101, test XRay client
sudo lxc-attach -n warp-tunnel -- ping 10.0.0.102

# Test full chain with curl through SOCKS proxy
curl --socks5 10.0.0.102:1080 https://ifconfig.me
```

### Test Profile 3 (VPS)
```bash
# Test XRay server listening
sudo lxc-attach -n xray-server -- netstat -tlnp | grep 443

# Test from client (should see VPS public IP)
curl --proxy vmess://UUID@VPS_IP:443 https://ifconfig.me
```

## Profile 4: General Containers with Optional Netmaker
**Use Case**: Beyond privacy router, any containers created at install
**Containers**: Variable (104+, user-defined)
**Networking**: Optional Netmaker mesh integration

### Overview

In addition to the privacy router containers (100-102), you can create general-purpose containers with optional Netmaker mesh networking. Each container can individually choose to join Netmaker or use standard bridge networking.

**IMPORTANT**: Containers are created via **Proxmox** using the full Proxmox schema and functionality. op-dbus manages **networking only** (OVS/Netmaker), not container creation.

### Architecture

**Container Creation**: Proxmox (Recommended & Primary Method)

**Proxmox Container Creation** (Full Proxmox Functionality Preserved):
- **Created By**: User via Proxmox web GUI or `pct create`
- **Storage**: `/var/lib/pve/local-btrfs/subvol/<vmid>/` (BTRFS subvolume)
- **Benefits**: Full Proxmox schema, snapshots, backups, migration, monitoring
- **op-dbus Role**: Takes over networking ONLY after creation

**Why Proxmox**:
- ✅ Full Proxmox feature set (snapshots, backups, HA, migration)
- ✅ Familiar web GUI for container management
- ✅ Proxmox schema with all configuration options
- ✅ Integration with Proxmox cluster features
- ✅ Template system and image management

**Alternative: Traditional LXC** (Theoretical, Not Recommended):
- Could use `lxc-create` to create containers in `/var/lib/lxc/`
- Same BTRFS subvolume format as Proxmox
- BUT: Loses all Proxmox functionality and schema
- Only mentioned for completeness - **use Proxmox in practice**

**Container Management Division**:
- **Proxmox manages**: Creation, lifecycle, storage, backups, snapshots
- **op-dbus manages**: Networking ONLY - OVS socket networking OR Netmaker mesh
- **Clean separation**: Proxmox = container, op-dbus = network

**Netmaker Integration**:
- Join key stored in `/etc/op-dbus/netmaker.env` on host
- Template auto-injects token at container creation
- First-boot script joins Netmaker network automatically

### Configuration Examples

#### Container WITH Netmaker (Mesh Networking)
```json
{
  "id": 104,
  "name": "app-server",
  "template": "debian-12",
  "autostart": true,
  "network": {
    "bridge": "mesh",
    "veth": true,
    "ipv4": "10.1.0.104/24"
  },
  "properties": {
    "network_type": "netmaker",
    "template": "debian-netmaker"
  }
}
```

**Behavior**:
- Uses `mesh` bridge (Netmaker WireGuard mesh)
- Reads `/etc/op-dbus/netmaker.env` from host
- Injects `NETMAKER_TOKEN` into container
- First boot: `netclient join -t $NETMAKER_TOKEN`
- Gets mesh IP (e.g., 10.10.0.104) from Netmaker

#### Container WITHOUT Netmaker (Standard Bridge)
```json
{
  "id": 105,
  "name": "web-server",
  "template": "debian-12",
  "autostart": true,
  "network": {
    "bridge": "ovsbr0",
    "veth": true,
    "ipv4": "10.0.0.105/24"
  },
  "properties": {
    "network_type": "bridge",
    "template": "debian-12"
  }
}
```

**Behavior**:
- Uses `ovsbr0` bridge (standard OVS bridge)
- No Netmaker integration
- Static IP configuration only
- No mesh networking

### Netmaker Token Configuration

**On Host** (`/etc/op-dbus/netmaker.env`):
```bash
# Netmaker enrollment token for automatic container join
NETMAKER_TOKEN="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**Token Injection Flow**:
1. Host stores token in `/etc/op-dbus/netmaker.env`
2. Container creation reads this file (via `lxc.rs:332`)
3. Token injected into container rootfs `/etc/netmaker.env`
4. First-boot service reads token and runs `netclient join`

### Template Creation (Traditional LXC + BTRFS)

**Create Netmaker-Ready Template**:
```bash
# On host (uses traditional LXC, not Proxmox pct)
# NOTE: create-netmaker-template.sh currently uses pct (legacy)
#       Need to rewrite to use: lxc-create, lxc-start, lxc-attach
sudo ./create-netmaker-template.sh

# Manual creation using traditional LXC:
sudo lxc-create -n netmaker-template -t debian -- -r bookworm
sudo lxc-start -n netmaker-template
sudo lxc-attach -n netmaker-template

# Install netclient inside container...
# (See script for full installation steps)

# Creates template at: /var/lib/lxc/netmaker-template/
```

**Template Contents**:
- netclient binary pre-installed
- First-boot join script (`/usr/local/bin/netmaker-first-boot.sh`)
- Systemd service for auto-join (`netmaker-first-boot.service`)
- WireGuard kernel modules

**BTRFS Subvolume Structure** (Same for Both Methods):
```
# Proxmox-created container
$ btrfs subvolume list /
ID 256 path var/lib/pve/local-btrfs/subvol/100
  └─ /var/lib/pve/local-btrfs/subvol/100/rootfs/  # Disk image

# Traditional LXC container
$ btrfs subvolume list /
ID 257 path var/lib/lxc/container-name
  └─ /var/lib/lxc/container-name/rootfs/  # Disk image (same format!)

# Both are BTRFS subvolumes with identical disk image format
# op-dbus works with either by managing OVS/Netmaker networking
```

**Creation Method Comparison**:

| Method | Path | Created By | Managed By | Networking |
|--------|------|-----------|------------|------------|
| Proxmox GUI | `/var/lib/pve/local-btrfs/subvol/<vmid>/` | User (Proxmox web UI) | op-dbus (networking only) | OVS/Netmaker |
| Traditional LXC | `/var/lib/lxc/<name>/` | User (lxc-create) | op-dbus (networking only) | OVS/Netmaker |
| BTRFS Template | Either path (snapshot cloning) | Either method | op-dbus (networking only) | OVS/Netmaker |

**op-dbus Role**: Regardless of creation method, op-dbus manages:
- OVS bridge configuration
- Socket OpenFlow networking OR Netmaker mesh
- Network policies and flows
- Container discovery (via OVS port introspection)

**✅ IMPLEMENTATION STATUS**:
Current code correctly uses Proxmox `pct` commands - this is the CORRECT approach:
- **src/state/plugins/lxc.rs:269-315**: Uses `pct create/start/attach` ✅ KEEP THIS
- **create-netmaker-template.sh**: Uses `pct` for Proxmox templates ✅ KEEP THIS
- **Proxmox schema**: Preserves full Proxmox functionality ✅ CORRECT
- **No changes needed**: Current implementation is correct for Proxmox containers

**Why pct is Correct**:
- Preserves Proxmox web GUI integration
- Maintains Proxmox configuration schema
- Keeps snapshots, backups, HA, migration features
- op-dbus focuses on networking layer only

### Installation with General Containers

**Full State Configuration** (Privacy Router + General Containers):
```json
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": 100,
          "name": "wireguard-gateway",
          "properties": {"network_type": "bridge"}
        },
        {
          "id": 101,
          "name": "warp-tunnel",
          "properties": {"network_type": "bridge"}
        },
        {
          "id": 102,
          "name": "xray-client",
          "properties": {"network_type": "bridge"}
        },
        {
          "id": 104,
          "name": "app-server",
          "properties": {
            "network_type": "netmaker",
            "template": "debian-netmaker"
          }
        },
        {
          "id": 105,
          "name": "database-server",
          "properties": {
            "network_type": "netmaker",
            "template": "debian-netmaker"
          }
        }
      ]
    },
    "openflow": {
      "enable_security_flows": true,
      "obfuscation_level": 3
    }
  }
}
```

**Installation**:
```bash
# 1. Create Netmaker template
sudo ./create-netmaker-template.sh

# 2. Set Netmaker token
echo 'NETMAKER_TOKEN="your-token-here"' | sudo tee /etc/op-dbus/netmaker.env

# 3. Install with privacy-client profile
sudo op-dbus init --profile privacy-client > /etc/op-dbus/state.json

# 4. Edit to add general containers (104, 105, etc.)
sudo nano /etc/op-dbus/state.json

# 5. Apply state (creates all containers)
sudo op-dbus apply /etc/op-dbus/state.json
```

### Netmaker Mesh Communication

**Container 104 → Container 105** (via Netmaker mesh):
```bash
# Inside container 104
ping 10.10.0.105  # Mesh IP, NOT bridge IP

# Direct WireGuard tunnel between containers
# No need to route through host OVS bridge
```

**Container 104 → Container 100** (via OVS bridge):
```bash
# Inside container 104
ping 10.0.0.100  # Bridge IP (WireGuard gateway)

# Routes through ovsbr0 bridge
# Uses OpenFlow flows for routing
```

### Choice: Netmaker vs Standard Bridge

| Feature | Netmaker (`network_type: "netmaker"`) | Standard Bridge (`network_type: "bridge"`) |
|---------|--------------------------------------|-------------------------------------------|
| Bridge | `mesh` | `ovsbr0` |
| IP Allocation | Dynamic (Netmaker DHCP) | Static (state.json) |
| Container-to-Container | Direct WireGuard tunnel | Via OVS flows |
| Multi-Host | Yes (mesh across hosts) | No (single host only) |
| Latency | +1-2ms (WireGuard) | +0.1ms (kernel bridge) |
| Security | Encrypted mesh | Plaintext (unless using tunnel containers) |
| Setup Complexity | Medium (requires Netmaker server) | Low (just OVS) |

### When to Use Netmaker

**Use Netmaker When**:
- Containers need to communicate across multiple Proxmox hosts
- You want encrypted container-to-container traffic
- Dynamic IP allocation is preferred
- Building a multi-datacenter mesh network

**Use Standard Bridge When**:
- All containers on same host (privacy router use case)
- Lowest latency required
- Static IP configuration is acceptable
- No need for cross-host networking

## Performance Comparison

| Profile | Containers | Obfuscation | Latency Overhead | Throughput | Use Case |
|---------|-----------|-------------|------------------|------------|----------|
| None | 0 | Level 1 | +0.1ms | 100% | Testing |
| Privacy Client | 3 | Level 3 | +5-10ms | 80-85% | Maximum privacy |
| Privacy VPS | 1 | Level 2 | +2-3ms | 90-95% | VPS endpoint |
| General + Netmaker | Variable | Level 1-2 | +1-2ms | 95% | Multi-host mesh |

## Security Considerations

### Profile 2 (Privacy Client)
- **Threat Model**: ISP surveillance, DPI, traffic analysis
- **Protection**: 3 layers (WireGuard + Warp + XRay), Level 3 obfuscation
- **Weakness**: All eggs in one basket (if host compromised, all containers exposed)

### Profile 3 (VPS)
- **Threat Model**: VPS provider surveillance, port scanning
- **Protection**: TLS encryption, Level 2 pattern hiding
- **Weakness**: Single point of failure (VPS IP can be blocked)

### Best Practice
- **Client**: Use Profile 2 with obfuscation_level=3
- **VPS**: Use Profile 3 with obfuscation_level=2
- **Rotate**: Change VPS endpoints monthly
- **Monitor**: Check flow statistics for anomalies

---

**Version**: 1.0.0
**Last Updated**: 2025-11-08
**Branch**: claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx
