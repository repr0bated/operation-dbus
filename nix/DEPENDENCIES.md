# Dependencies for Proxmox Full Mode

## Overview

For **Full (Proxmox) Mode**, op-dbus requires several system components. The NixOS module handles these automatically, but this document explains what's needed and why.

## Required Components

### 1. D-Bus (✅ You have this)

**What it is:** System message bus for inter-process communication
**Why needed:** Core of op-dbus plugin system
**NixOS handles:** Automatically enabled with systemd

```nix
# Already included in NixOS base system
services.dbus.enable = true;
```

### 2. OpenVSwitch (❌ Missing)

**What it is:** Software-defined networking switch
**Why needed:** Creates ovsbr0 and mesh bridges for container networking
**NixOS config:**

```nix
# Add to configuration.nix
virtualisation.vswitch = {
  enable = true;
  resetOnStart = false;
};

# Or just install the package
environment.systemPackages = with pkgs; [
  openvswitch
];

# Enable the service
systemd.services.openvswitch.enable = true;
```

**Verification:**
```bash
ovs-vsctl show
# Should show OVS database
```

### 3. LXC Containers (❌ Missing)

**What it is:** Linux container runtime
**Why needed:** Manages containers for gateway/warp/xray services
**NixOS config:**

```nix
# Enable LXC support
virtualisation.lxc = {
  enable = true;
  lxcfs.enable = true;
};

environment.systemPackages = with pkgs; [
  lxc
  lxcfs
];
```

**Verification:**
```bash
lxc-checkconfig
# Should show all features enabled
```

### 4. Proxmox VE Tools (❌ Optional but recommended)

**What it is:** Proxmox container management tools (pct, pveam)
**Why needed:** Advanced container management with templates
**Note:** Not available in nixpkgs - use LXC directly instead

**Alternative on NixOS:**
```nix
# Use systemd-nspawn instead of Proxmox pct
virtualisation.containers.enable = true;

# Or use LXC commands directly
# lxc-create, lxc-start, lxc-attach
```

### 5. PackageKit (✅ You have this)

**What it is:** Package management abstraction layer
**Why needed:** Used by some D-Bus plugins for system queries
**NixOS handles:** Already in base system

## Complete NixOS Configuration for Full Mode

Here's a complete configuration.nix for Proxmox Full mode:

```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    /path/to/operation-dbus/nix/module.nix
  ];

  # System packages
  environment.systemPackages = with pkgs; [
    openvswitch
    lxc
    vim
    git
    curl
    htop
  ];

  # Enable OpenVSwitch
  virtualisation.vswitch = {
    enable = true;
    resetOnStart = false;
  };

  systemd.services.openvswitch = {
    enable = true;
    wantedBy = [ "multi-user.target" ];
  };

  # Enable LXC containers
  virtualisation.lxc = {
    enable = true;
    lxcfs.enable = true;
  };

  # Enable D-Bus (already default in NixOS)
  services.dbus.enable = true;

  # Enable op-dbus in Full (Proxmox) mode
  services.op-dbus = {
    enable = true;
    mode = "full";

    stateConfig = {
      # Network bridges
      net = {
        interfaces = [
          {
            name = "ovsbr0";
            type = "ovs-bridge";
            ports = [ "eth0" ];  # Your physical interface
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "192.168.1.10"; prefix = 24; }
              ];
              gateway = "192.168.1.1";
            };
          }
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

      # LXC containers
      lxc = {
        containers = [
          {
            id = "100";
            name = "gateway";
            veth = "vi100";
            bridge = "mesh";
            running = true;
          }
          {
            id = "101";
            name = "warp";
            veth = "vi101";
            bridge = "mesh";
            running = true;
          }
          {
            id = "102";
            name = "xray-client";
            veth = "vi102";
            bridge = "mesh";
            running = true;
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

    enableBlockchain = true;
    enableCache = true;
  };

  # Network configuration
  networking = {
    hostName = "ghostbridge";
    useDHCP = false;
    # OpenVSwitch will manage interfaces
  };

  # Firewall
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 9573 ];  # SSH and MCP
    trustedInterfaces = [ "ovsbr0" "mesh" ];
  };

  system.stateVersion = "24.05";
}
```

## Dependency Matrix by Mode

| Component | Agent Mode | Standalone Mode | Full Mode |
|-----------|-----------|-----------------|-----------|
| D-Bus | ✅ Required | ✅ Required | ✅ Required |
| systemd | ✅ Required | ✅ Required | ✅ Required |
| OpenVSwitch | ❌ Not used | ✅ Required | ✅ Required |
| LXC | ❌ Not used | ❌ Not used | ✅ Required |
| Proxmox VE | ❌ Not used | ❌ Not used | ⚠️ Optional |
| PackageKit | ⚠️ Optional | ⚠️ Optional | ⚠️ Optional |
| Netmaker | ❌ Not used | ❌ Not used | ⚠️ Optional |

## Installing Missing Dependencies

### Quick Install (NixOS)

```bash
# Add to configuration.nix:
sudo nano /etc/nixos/configuration.nix

# Add the virtualisation options shown above

# Rebuild system
sudo nixos-rebuild switch

# Verify
ovs-vsctl show
lxc-checkconfig
systemctl status openvswitch
systemctl status op-dbus
```

### Manual Package Install (Debian/Ubuntu basis)

If you're on a traditional system before NixOS install:

```bash
# OpenVSwitch
sudo apt-get install openvswitch-switch

# LXC
sudo apt-get install lxc lxcfs

# Verify
ovs-vsctl show
lxc-checkconfig
```

## Why Each Dependency

### OpenVSwitch (OVS)
- Creates software bridges (`ovsbr0`, `mesh`)
- Supports OpenFlow for traffic routing
- Enables policy-based networking
- Allows container network isolation
- Required for multi-bridge architecture

### LXC
- Lightweight container runtime
- Manages gateway/warp/xray containers
- Provides network namespace isolation
- Integrates with OVS bridges via veth pairs
- Alternative to Docker with better Proxmox compatibility

### D-Bus
- Plugin communication bus
- Service state queries (systemd, login1)
- Network configuration changes
- System events and signals
- Core of op-dbus architecture

### PackageKit
- System package queries
- Update status monitoring
- Used by some D-Bus plugins
- Not strictly required for op-dbus core functionality

## Troubleshooting

### OpenVSwitch not starting

```bash
# Check logs
journalctl -u openvswitch

# Manual start
sudo systemctl start openvswitch
sudo systemctl enable openvswitch

# Verify
ovs-vsctl show
```

### LXC check fails

```bash
# Run diagnostics
lxc-checkconfig

# Common issues:
# - Kernel modules not loaded
# - cgroups v2 not enabled
# - User namespaces disabled

# On NixOS, ensure:
virtualisation.lxc.enable = true;
```

### D-Bus connection failed

```bash
# Check D-Bus is running
systemctl status dbus

# Permissions
sudo usermod -aG dbus $USER

# Test connection
dbus-send --system --print-reply --dest=org.freedesktop.DBus / org.freedesktop.DBus.ListNames
```

## Next Steps

1. **Install OpenVSwitch** - Core requirement for all bridge modes
2. **Enable LXC** - Required for Full (Proxmox) mode containers
3. **Apply NixOS config** - Use the complete example above
4. **Verify services** - Check all components running
5. **Deploy op-dbus** - Run `nixos-rebuild switch`

## See Also

- **nix/PROXMOX.md** - Complete Proxmox deployment guide
- **nix/README.md** - General NixOS integration
- **INSTALLATION.md** - Traditional installation (non-NixOS)
