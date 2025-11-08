# NixOS Integration for op-dbus

Complete NixOS packaging and module for op-dbus.

## Overview

This directory contains NixOS-specific packaging:
- `package.nix` - Nix derivation to build op-dbus
- `module.nix` - NixOS module for declarative configuration
- `flake.nix` - Nix flake for modern Nix workflows

## Quick Start (Flakes)

### 1. Add to your flake.nix

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    op-dbus.url = "github:ghostbridge/op-dbus";
  };

  outputs = { self, nixpkgs, op-dbus }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        op-dbus.nixosModules.default
        {
          services.op-dbus = {
            enable = true;
            mode = "standalone";
          };
        }
      ];
    };
  };
}
```

### 2. Configure op-dbus

Add to your NixOS configuration:

```nix
services.op-dbus = {
  enable = true;
  mode = "standalone";  # or "full" or "agent"

  # Declarative state configuration
  stateConfig = {
    net = {
      interfaces = [
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
      ];
    };

    systemd = {
      units = {
        "openvswitch.service" = {
          enabled = true;
          active_state = "active";
        };
      };
    };
  };
};
```

### 3. Rebuild

```bash
sudo nixos-rebuild switch
```

That's it! op-dbus is now running and managing your system state declaratively.

## Deployment Modes

### Full Mode (Proxmox)

For container-based deployments with mesh networking:

```nix
services.op-dbus = {
  enable = true;
  mode = "full";

  stateConfig = {
    net.interfaces = [
      { name = "ovsbr0"; type = "ovs-bridge"; ... }
      { name = "mesh"; type = "ovs-bridge"; ... }
    ];
    lxc.containers = [
      { id = "101"; veth = "vi101"; bridge = "mesh"; }
    ];
  };
};
```

### Standalone Mode

For enterprise deployments without containers:

```nix
services.op-dbus = {
  enable = true;
  mode = "standalone";

  stateConfig = {
    net.interfaces = [
      { name = "ovsbr0"; type = "ovs-bridge"; ... }
    ];
    systemd.units = { ... };
  };
};
```

### Agent-Only Mode

Minimal deployment with D-Bus plugins only:

```nix
services.op-dbus = {
  enable = true;
  mode = "agent";

  stateConfig = {
    systemd.units = {};
  };
};
```

## Configuration Options

### Complete Reference

```nix
services.op-dbus = {
  # Enable the service
  enable = true;

  # Package to use (override if needed)
  package = pkgs.op-dbus;

  # Deployment mode
  mode = "standalone";  # "full" | "standalone" | "agent"

  # Declarative state configuration
  stateConfig = {
    # Network plugin
    net = {
      interfaces = [
        {
          name = "ovsbr0";
          type = "ovs-bridge";
          ports = [ "eth0" ];
          ipv4 = {
            enabled = true;
            dhcp = false;
            address = [{ ip = "..."; prefix = 24; }];
            gateway = "...";
          };
        }
      ];
    };

    # Systemd plugin
    systemd = {
      units = {
        "service-name.service" = {
          enabled = true;
          active_state = "active";
        };
      };
    };

    # LXC plugin (full mode only)
    lxc = {
      containers = [
        {
          id = "101";
          veth = "vi101";
          bridge = "mesh";
          running = true;
        }
      ];
    };
  };

  # Data directory
  dataDir = "/var/lib/op-dbus";

  # Enable blockchain audit logging
  enableBlockchain = true;

  # Enable caching
  enableCache = true;

  # TODO: NUMA configuration
  # numaCpuAffinity = "0-3";
  # numaPolicy = "bind";
};
```

## Building from Source

### Development Shell

```bash
nix develop github:ghostbridge/op-dbus
```

This provides:
- Rust toolchain
- All build dependencies
- Development tools (rust-analyzer, clippy, rustfmt)

### Building the Package

```bash
nix build github:ghostbridge/op-dbus
```

Result will be in `./result/bin/op-dbus`

### Building Locally

```bash
cd /path/to/op-dbus
nix build .#op-dbus
```

## Advanced Usage

### Custom Package Build

Build with specific features:

```nix
services.op-dbus = {
  enable = true;
  package = pkgs.op-dbus.override {
    buildFeatures = [ "mcp" "ml" ];
  };
};
```

### Multiple Instances

Run different modes on different hosts:

```nix
# Host 1: Full mode (Proxmox)
services.op-dbus = {
  enable = true;
  mode = "full";
};

# Host 2: Standalone mode
services.op-dbus = {
  enable = true;
  mode = "standalone";
};
```

### State Introspection

Generate initial state from current system:

```bash
# On NixOS
op-dbus init --introspect --output /tmp/state.json

# Convert to Nix expression
nix-instantiate --eval --json /tmp/state.json
```

## Comparison: NixOS vs Bash Scripts

| Feature | NixOS Module | Bash Scripts |
|---------|-------------|--------------|
| Installation | Declarative | Imperative |
| Dependencies | Automatic | Manual (install-dependencies.sh) |
| Configuration | /etc/nixos/configuration.nix | /etc/op-dbus/state.json |
| Updates | nixos-rebuild | Manual reinstall |
| Rollback | Built-in | Manual uninstall |
| State Management | Nix + op-dbus | op-dbus only |

## Troubleshooting

### Check Service Status

```bash
systemctl status op-dbus.service
```

### View Logs

```bash
journalctl -fu op-dbus
```

### Verify Configuration

```bash
op-dbus query
op-dbus diff /etc/op-dbus/state.json
```

### Rebuild with Debugging

```bash
sudo nixos-rebuild switch --show-trace
```

## Migration

### From Traditional Installation

If you installed via bash scripts, migration is straightforward:

1. Extract your current state:
   ```bash
   op-dbus query > /tmp/current-state.json
   ```

2. Convert to NixOS configuration:
   ```nix
   services.op-dbus = {
     enable = true;
     stateConfig = {
       # Copy from current-state.json
     };
   };
   ```

3. Uninstall bash version:
   ```bash
   sudo ./uninstall.sh
   ```

4. Activate NixOS configuration:
   ```bash
   sudo nixos-rebuild switch
   ```

## Development

### Testing Changes

```bash
# Build locally
nix build .#op-dbus

# Run in VM
nixos-rebuild build-vm --flake .#
./result/bin/run-*-vm
```

### Module Development

Edit `module.nix` and test:

```bash
sudo nixos-rebuild test --flake .#
```

## Support

- **NixOS-specific issues**: Open issue on GitHub
- **General op-dbus**: See main README.md
- **Documentation**: INSTALLATION.md, ENTERPRISE-DEPLOYMENT.md

## TODO

- [ ] BTRFS subvolume creation for cache
- [ ] NUMA CPU pinning configuration
- [ ] MCP binary installation when built with features
- [ ] Automated tests in NixOS VM
- [ ] Home Manager integration
- [ ] NixOS option generation from JSON schema
