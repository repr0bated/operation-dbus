# Nix Installation Guide

Complete guide for installing and using op-dbus with Nix and NixOS.

## Table of Contents

- [Quick Start](#quick-start)
- [Installation Methods](#installation-methods)
- [NixOS Module](#nixos-module)
- [Development](#development)
- [Advanced Configuration](#advanced-configuration)

## Quick Start

### With Flakes (Recommended)

```bash
# Try op-dbus without installing
nix run github:repr0bated/operation-dbus -- --help

# Query system state
nix run github:repr0bated/operation-dbus -- query

# Run diagnostics
nix run github:repr0bated/operation-dbus -- doctor

# Install to user profile
nix profile install github:repr0bated/operation-dbus
```

### Without Flakes (Traditional)

```bash
# Clone repository
git clone https://github.com/repr0bated/operation-dbus.git
cd operation-dbus

# Build
nix-build

# Install to user profile
nix-env -i -f default.nix
```

## Installation Methods

### Method 1: Flake-based Installation

**Prerequisites:**
- Nix with flakes enabled
- Add to `/etc/nix/nix.conf` or `~/.config/nix/nix.conf`:
  ```
  experimental-features = nix-command flakes
  ```

**Install system-wide (NixOS):**

Add to `/etc/nixos/configuration.nix`:

```nix
{
  inputs.op-dbus.url = "github:repr0bated/operation-dbus";

  # In your configuration
  environment.systemPackages = [
    inputs.op-dbus.packages.${system}.default
  ];
}
```

**Install to user profile:**

```bash
nix profile install github:repr0bated/operation-dbus
```

### Method 2: Traditional Nix

**Build from source:**

```bash
git clone https://github.com/repr0bated/operation-dbus.git
cd operation-dbus
nix-build
```

Result link: `./result/bin/op-dbus`

**Install to user profile:**

```bash
nix-env -i -f default.nix
```

### Method 3: NixOS System Configuration

See [NixOS Module](#nixos-module) section below.

## NixOS Module

`★ Insight ─────────────────────────────────────`
**Declarative on Declarative**

NixOS manages packages/services declaratively via Nix.
op-dbus manages runtime state declaratively via JSON.

Together: Full infrastructure-as-code from packages to
runtime D-Bus state!
`─────────────────────────────────────────────────`

### Basic Configuration

Add to `/etc/nixos/configuration.nix`:

```nix
{
  imports = [
    # Import op-dbus module
    (builtins.fetchGit {
      url = "https://github.com/repr0bated/operation-dbus";
      ref = "main";
    } + "/nixos-module.nix")
  ];

  services.op-dbus = {
    enable = true;

    # Declarative state configuration
    state = {
      version = 1;
      plugins = {
        systemd = {
          units = {
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };
    };
  };
}
```

### With Network Management (OpenVSwitch)

```nix
{
  services.op-dbus = {
    enable = true;

    state = {
      version = 1;
      plugins = {
        net = {
          interfaces = [{
            name = "br0";
            type = "ovs-bridge";
            ports = [ "eth0" ];
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [{
                ip = "192.168.1.10";
                prefix = 24;
              }];
              gateway = "192.168.1.1";
            };
          }];
        };
        systemd = {
          units = {
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };
    };
  };

  # OpenVSwitch automatically enabled when net plugin is used
  # virtualisation.openvswitch.enable = true;  # Auto-enabled
}
```

### Module Options

```nix
services.op-dbus = {
  enable = true;                    # Enable op-dbus service

  package = pkgs.op-dbus;           # Package to use

  state = { ... };                  # Declarative state (see above)

  autoDiscovery = true;             # Auto-discover D-Bus services

  enabledPlugins = [                # Built-in plugins to enable
    "systemd"
    "login1"
    "net"
  ];

  stateFile = "/etc/op-dbus/state.json";  # State file path

  dataDir = "/var/lib/op-dbus";     # Blockchain storage

  environmentFiles = [              # Secret env files
    "/run/secrets/netmaker-token"
  ];
};
```

### With Secrets (agenix/sops-nix)

Using agenix for secrets:

```nix
{
  age.secrets.netmaker-token = {
    file = ./secrets/netmaker-token.age;
    owner = "root";
    mode = "0400";
  };

  services.op-dbus = {
    enable = true;
    environmentFiles = [
      config.age.secrets.netmaker-token.path
    ];
    state = {
      version = 1;
      plugins = {
        lxc = {
          containers = [{
            id = "100";
            veth = "vi100";
            bridge = "mesh";
            properties = {
              network_type = "netmaker";
            };
          }];
        };
      };
    };
  };
}
```

## Development

### Development Shell with Flakes

```bash
# Enter development environment
nix develop

# Or specify from URL
nix develop github:repr0bated/operation-dbus

# Build and test
cargo build --release
cargo test
cargo run -- query
```

### Development Shell without Flakes

```bash
# Enter development environment
nix-shell

# Build and test
cargo build --release
cargo test
```

### Development Workflow

```bash
# 1. Clone repository
git clone https://github.com/repr0bated/operation-dbus.git
cd operation-dbus

# 2. Enter dev shell
nix develop  # or nix-shell

# 3. Make changes
vim src/main.rs

# 4. Build and test
cargo build
cargo test

# 5. Run locally
sudo ./target/debug/op-dbus query

# 6. Auto-rebuild on changes
cargo watch -x check
```

## Advanced Configuration

### Overlay for Custom Builds

Create an overlay to customize op-dbus:

```nix
# overlay.nix
final: prev: {
  op-dbus = prev.op-dbus.override {
    # Custom features
    features = [ "ml" "mcp" ];
  };
}
```

Use in configuration:

```nix
{
  nixpkgs.overlays = [
    (import ./overlay.nix)
  ];

  services.op-dbus.enable = true;
}
```

### Multiple Instances

Run multiple op-dbus instances with different configs:

```nix
{
  systemd.services.op-dbus-prod = {
    description = "op-dbus (Production)";
    serviceConfig.ExecStart = "${pkgs.op-dbus}/bin/op-dbus run --state-file /etc/op-dbus/prod.json";
    # ... other config
  };

  systemd.services.op-dbus-dev = {
    description = "op-dbus (Development)";
    serviceConfig.ExecStart = "${pkgs.op-dbus}/bin/op-dbus run --state-file /etc/op-dbus/dev.json";
    # ... other config
  };
}
```

### Integration with Home Manager

Use op-dbus in user-level configuration:

```nix
# home.nix
{ pkgs, ... }:
{
  home.packages = [ pkgs.op-dbus ];

  # User-level state management
  home.file.".config/op-dbus/state.json".text = builtins.toJSON {
    version = 1;
    plugins = {
      systemd = {
        units = {
          # User units
        };
      };
    };
  };
}
```

## Platform Support

### Supported Platforms

- ✅ **x86_64-linux** - Fully supported
- ✅ **aarch64-linux** - Fully supported (ARM64)
- ⚠️ **i686-linux** - Should work (32-bit)
- ❌ **Darwin** (macOS) - Not supported (requires Linux D-Bus)

### Tested On

- NixOS 23.11
- NixOS unstable
- Nix on Ubuntu 22.04
- Nix on Debian 12

## Troubleshooting

### Flakes Not Enabled

**Error:** `error: experimental Nix feature 'flakes' is disabled`

**Solution:**
```bash
# Enable flakes
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

### Build Fails

**Error:** `error: hash mismatch in fixed-output derivation`

**Solution:**
```bash
# Update Cargo.lock hash
nix flake update
```

### Permission Denied

**Error:** `Permission denied` when querying D-Bus

**Solution:**
```bash
# Run as root
sudo op-dbus query

# Or add user to appropriate groups
sudo usermod -aG systemd-journal $USER
```

### OVS Not Found

**Error:** `⊗ Skipping plugin: net - OpenVSwitch not found`

**Solution:**
```nix
# Enable OVS in NixOS configuration
{
  virtualisation.openvswitch.enable = true;

  # Or let op-dbus module enable it automatically
  services.op-dbus.enabledPlugins = [ "net" ];
}
```

## Examples

### Complete NixOS Configuration

```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    (builtins.fetchGit {
      url = "https://github.com/repr0bated/operation-dbus";
      ref = "main";
    } + "/nixos-module.nix")
  ];

  # System configuration
  networking.hostName = "nixos-server";

  # op-dbus configuration
  services.op-dbus = {
    enable = true;
    autoDiscovery = true;

    state = {
      version = 1;
      plugins = {
        # Manage OVS networking
        net = {
          interfaces = [{
            name = "br0";
            type = "ovs-bridge";
            ports = [ "ens3" ];
            ipv4 = {
              enabled = true;
              address = [{ ip = "192.168.1.10"; prefix = 24; }];
              gateway = "192.168.1.1";
            };
          }];
        };

        # Manage systemd services
        systemd = {
          units = {
            "nginx.service" = {
              active_state = "active";
              enabled = true;
            };
            "postgresql.service" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };
    };
  };

  # Other system configuration...
  system.stateVersion = "23.11";
}
```

## Resources

- **Source Code:** https://github.com/repr0bated/operation-dbus
- **Documentation:** [README.md](README.md), [INSTALL.md](INSTALL.md)
- **NixOS Manual:** https://nixos.org/manual/nixos/stable/
- **Nix Flakes:** https://nixos.wiki/wiki/Flakes

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on:
- Adding new Nix features
- Improving the NixOS module
- Testing on different platforms
- Submitting pull requests
