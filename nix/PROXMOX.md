# Proxmox Installation with NixOS

Guide for deploying op-dbus in **Full (Proxmox) Mode** using NixOS configuration.

## Overview

Proxmox mode includes the complete op-dbus stack:
- D-Bus plugin system
- Blockchain audit logging
- OVS bridge management (ovsbr0 + mesh)
- LXC/Proxmox container integration
- Netmaker mesh networking support
- OpenFlow policy management

## Prerequisites

### System Requirements
- NixOS (or NixOS container on Proxmox)
- Proxmox VE 7.0+ (if using LXC features)
- Root/sudo access
- Network interfaces for bridge creation

### Required Software
All dependencies are handled automatically by the NixOS module:
- OpenVSwitch
- D-Bus
- systemd
- Rust toolchain (for building)

### Optional
- netclient (Netmaker mesh networking)
- pct (Proxmox container management)

## Quick Start

### 1. Enable Nix Flakes

```bash
# In installer or live environment
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# On installed NixOS system, add to configuration.nix:
nix.settings.experimental-features = [ "nix-command" "flakes" ];
```

### 2. Download op-dbus

```bash
# Download latest from branch
curl -L -o operation-dbus.tar.gz https://github.com/repr0bated/operation-dbus/archive/refs/heads/claude/install-script-gap-dev-011CUupgDV45F7ABCw7aMNhx.tar.gz

# Extract
tar xzf operation-dbus.tar.gz
cd operation-dbus-*
```

### 3. Configure NixOS for Proxmox Mode

Add to your `/etc/nixos/configuration.nix`:

```nix
{ config, pkgs, ... }:

{
  # Import op-dbus module
  imports = [
    ./hardware-configuration.nix
    /path/to/operation-dbus/nix/module.nix
  ];

  # Enable op-dbus in Full (Proxmox) mode
  services.op-dbus = {
    enable = true;
    mode = "full";

    # Proxmox-specific configuration
    stateConfig = {
      # Network configuration with mesh bridge
      net = {
        interfaces = [
          # Main OVS bridge
          {
            name = "ovsbr0";
            type = "ovs-bridge";
            ports = [ "eth0" ];
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "192.168.1.10"; prefix = 24; }
              ];
              gateway = "192.168.1.1";
            };
          }
          # Mesh bridge for container networking
          {
            name = "mesh";
            type = "ovs-bridge";
            ports = [];
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "10.0.0.1"; prefix = 24; }
              ];
            };
          }
        ];
      };

      # LXC container configuration
      lxc = {
        containers = [
          {
            id = "101";
            name = "gateway";
            veth = "vi101";
            bridge = "mesh";
            running = true;
            template = "debian-12";
          }
          {
            id = "102";
            name = "warp";
            veth = "vi102";
            bridge = "mesh";
            running = true;
            template = "debian-12";
          }
          {
            id = "103";
            name = "xray-client";
            veth = "vi103";
            bridge = "mesh";
            running = true;
            template = "debian-12";
          }
        ];
      };

      # Systemd services
      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };
    };

    # Enable blockchain audit logging
    enableBlockchain = true;

    # Enable caching
    enableCache = true;
  };

  # Additional Proxmox-specific packages
  environment.systemPackages = with pkgs; [
    openvswitch
    lxc
    # Add netmaker client if using mesh networking
  ];

  # Enable required services
  systemd.services.openvswitch.enable = true;
  virtualisation.lxc.enable = true;
}
```

### 4. Apply Configuration

```bash
# Test the configuration
sudo nixos-rebuild test

# Apply permanently
sudo nixos-rebuild switch
```

### 5. Verify Installation

```bash
# Check service status
systemctl status op-dbus.service

# Query current state
op-dbus query

# View logs
journalctl -fu op-dbus
```

## Container Deployment Options

### Option 1: Gateway + WARP + Xray Client

Full mesh networking stack with Cloudflare WARP and Xray proxy:

```nix
lxc.containers = [
  {
    id = "101";
    name = "gateway";
    veth = "vi101";
    bridge = "mesh";
    running = true;
  }
  {
    id = "102";
    name = "warp";
    veth = "vi102";
    bridge = "mesh";
    running = true;
  }
  {
    id = "103";
    name = "xray-client";
    veth = "vi103";
    bridge = "mesh";
    running = true;
  }
];
```

### Option 2: Xray Server Only

Minimal proxy server configuration:

```nix
lxc.containers = [
  {
    id = "104";
    name = "xray-server";
    veth = "vi104";
    bridge = "mesh";
    running = true;
  }
];
```

### Option 3: No Containers

Use Proxmox GUI to create containers manually (recommended for existing setups):

```nix
lxc.containers = [];
```

The Proxmox schema and container structure remain compatible with manually created containers.

## Network Bridge Configuration

### Standard Setup (ovsbr0 + mesh)

```nix
net.interfaces = [
  # Physical interface bridge
  {
    name = "ovsbr0";
    type = "ovs-bridge";
    ports = [ "eth0" ];  # Your physical interface
    ipv4 = {
      enabled = true;
      dhcp = false;
      address = [{ ip = "192.168.1.10"; prefix = 24; }];
      gateway = "192.168.1.1";
    };
  }
  # Container mesh network
  {
    name = "mesh";
    type = "ovs-bridge";
    ports = [];  # Container veths added dynamically
    ipv4 = {
      enabled = true;
      dhcp = false;
      address = [{ ip = "10.0.0.1"; prefix = 24; }];
    };
  }
];
```

### Netmaker Integration

For Netmaker mesh networking, add the netmaker interface to the mesh bridge:

```nix
{
  name = "mesh";
  type = "ovs-bridge";
  ports = [ "nm-gateway" ];  # Netmaker interface
  ipv4 = {
    enabled = true;
    dhcp = false;
    address = [{ ip = "10.0.0.1"; prefix = 24; }];
  };
}
```

## Building from Source

### Development Environment

```bash
# Enter development shell with all dependencies
nix develop

# Available tools:
# - Rust toolchain (rustc, cargo)
# - Development tools (rust-analyzer, clippy, rustfmt)
# - Node.js and npm (for web UI)
# - All system dependencies (OVS, D-Bus, etc.)
```

### Build the Package

```bash
# Build locally
nix build .#op-dbus

# Result
ls -la result/bin/op-dbus

# Verify nix folder is included
ls -la result/share/op-dbus/nix/
```

### Custom Build with Features

```nix
services.op-dbus = {
  enable = true;
  mode = "full";
  package = pkgs.op-dbus.override {
    buildFeatures = [ "mcp" "ml" "web" ];
  };
};
```

## State Introspection

Generate declarative configuration from existing Proxmox setup:

```bash
# Introspect current system state
op-dbus init --introspect --output /tmp/state.json

# View captured state
cat /tmp/state.json

# The introspection captures:
# - Network interfaces and OVS bridges
# - LXC containers (if pct is available)
# - Systemd units
# - Login sessions
# - DNS resolver configuration
```

**Note**: Introspection queries existing plugin state, not raw hardware. In a fresh installer, many fields will be empty - this is expected.

## Proxmox-Specific Features

### Container Socket Access

Containers can access the host D-Bus socket for CLI commands:

```bash
# Inside container
op-dbus query --socket /run/op-dbus.sock
```

### BTRFS Subvolumes

op-dbus cache can use BTRFS subvolumes (Proxmox uses BTRFS for storage):

```nix
services.op-dbus = {
  enable = true;
  dataDir = "/var/lib/op-dbus";
  enableCache = true;
};
```

The module will automatically create BTRFS subvolumes if the filesystem supports it.

### Proxmox GUI Integration

You can still use the Proxmox web GUI to create containers. op-dbus will detect and manage containers created through either method:

- NixOS declarative configuration
- Proxmox web GUI
- `pct` command line

All containers follow the Proxmox LXC schema.

## Comparison: NixOS vs Bash Install

| Feature | NixOS Module | Bash Scripts |
|---------|-------------|--------------|
| Installation | Declarative | Imperative |
| Dependencies | Automatic | Manual (`install-dependencies.sh`) |
| Configuration | `/etc/nixos/configuration.nix` | `/etc/op-dbus/state.json` |
| Mode Selection | `mode = "full"` | `./install.sh --full` |
| Container Config | Declarative in Nix | Runtime detection |
| Updates | `nixos-rebuild switch` | Manual reinstall |
| Rollback | `nixos-rebuild --rollback` | Manual uninstall |
| Proxmox Integration | Native | Native |

## Troubleshooting

### Container Creation Fails

```bash
# Check Proxmox/LXC is available
which pct

# Verify container templates
pveam update
pveam list

# Check permissions
ls -la /var/lib/lxc/
```

### Bridge Not Created

```bash
# Check OVS is running
systemctl status openvswitch

# Manually verify bridge
ovs-vsctl show

# Check op-dbus logs
journalctl -fu op-dbus | grep -i bridge
```

### Netmaker Interface Not Added

```bash
# Verify netmaker interface exists
ip addr show nm-gateway

# Check if added to bridge
ovs-vsctl list-ports mesh

# Manually add if needed
ovs-vsctl add-port mesh nm-gateway
```

## Migration from Bash Install

If you installed via bash scripts, migrate to NixOS:

### 1. Extract Current State

```bash
op-dbus query > /tmp/current-state.json
```

### 2. Convert to NixOS Configuration

Convert the JSON state to NixOS configuration format (see examples above).

### 3. Uninstall Bash Version

```bash
sudo ./uninstall.sh
```

### 4. Apply NixOS Configuration

```bash
sudo nixos-rebuild switch
```

## Advanced Configuration

### NUMA Optimization

```nix
services.op-dbus = {
  enable = true;
  # TODO: NUMA configuration (planned)
  # numaCpuAffinity = "0-3";
  # numaPolicy = "bind";
};
```

### Custom Data Directory

```nix
services.op-dbus = {
  enable = true;
  dataDir = "/mnt/storage/op-dbus";
  enableCache = true;
};
```

### Multiple Bridge Modes

```nix
# Single bridge (standalone networking)
net.interfaces = [
  { name = "ovsbr0"; type = "ovs-bridge"; ... }
];

# Dual bridge (container + external)
net.interfaces = [
  { name = "ovsbr0"; type = "ovs-bridge"; ... }
  { name = "mesh"; type = "ovs-bridge"; ... }
];
```

## Next Steps

- Configure Netmaker mesh networking
- Set up container templates
- Deploy WARP/Xray services
- Enable blockchain audit logging
- Configure OpenFlow policies

## Support

- **NixOS module issues**: Open issue on GitHub
- **Proxmox integration**: See ENTERPRISE-DEPLOYMENT.md
- **General installation**: See INSTALLATION.md
- **Container setup**: See main README.md

## TODO

- [ ] Automated container template creation
- [ ] Proxmox cluster support
- [ ] HA configuration
- [ ] Backup/restore workflows
- [ ] Integration with Proxmox Backup Server
