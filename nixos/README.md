# NixOS Configuration for op-dbus

This directory contains NixOS module and configurations for declarative op-dbus deployment.

## Quick Start

### Using Flakes (Recommended)

1. **Build the package:**
   ```bash
   nix build
   ```

2. **Test the configuration:**
   ```bash
   ./validate-config.sh
   ```

3. **Apply to your system:**
   ```nix
   # In your /etc/nixos/configuration.nix
   {
     imports = [ /path/to/operation-dbus/nixos/module.nix ];

     services.op-dbus = {
       enable = true;
       mode = "standalone";
       mcp.enable = true;
     };
   }
   ```

4. **Rebuild:**
   ```bash
   sudo nixos-rebuild switch
   ```

### Using Traditional NixOS

1. **Copy module to your system:**
   ```bash
   sudo cp module.nix /etc/nixos/op-dbus.nix
   ```

2. **Import in configuration.nix:**
   ```nix
   {
     imports = [ ./op-dbus.nix ];

     services.op-dbus.enable = true;
   }
   ```

3. **Rebuild:**
   ```bash
   sudo nixos-rebuild switch
   ```

## Configuration Options

### Deployment Modes

```nix
services.op-dbus = {
  mode = "standalone"; # or "full" or "agent-only"
};
```

- **standalone**: D-Bus + OVS (no containers) - Best for enterprise deployments
- **full**: D-Bus + OVS + LXC/Proxmox + Netmaker - Full features
- **agent-only**: D-Bus plugins only - Minimal footprint

### MCP Integration

```nix
services.op-dbus.mcp = {
  enable = true;
  introspection = true;
  hybridScanner = true;

  agents = {
    systemd = true;
    packagekit = true;
    network = true;
    file = true;
  };
};
```

### Network Configuration

```nix
services.op-dbus.network = {
  interfaces = [
    {
      name = "ovsbr0";
      type = "ovs-bridge";
      ports = [ "eth0" ];
      ipv4 = {
        enabled = true;
        dhcp = false;
        addresses = [{
          ip = "192.168.1.100";
          prefix = 24;
        }];
        gateway = "192.168.1.1";
      };
    }
  ];

  ovs = {
    enable = true;
    bridges = [ "ovsbr0" "mesh" ];
  };
};
```

### Package Management

```nix
services.op-dbus.packages = {
  enable = true;
  installed = [ "nginx" "postgresql" ];
  removed = [ "apache2" ];
  autoUpdate = false;
};
```

### Systemd Units

```nix
services.op-dbus.systemd = {
  units = {
    "sshd.service" = {
      active_state = "active";
      enabled = true;
    };
  };
};
```

## File Structure

```
nixos/
├── module.nix           # NixOS module definition
├── configuration.nix    # Example configuration
├── flake.nix            # Nix flake for modern workflows
├── validate-config.sh   # Configuration validation script
└── README.md            # This file
```

## Validation

Before applying, validate your configuration:

```bash
./validate-config.sh
```

This will:
1. Check Nix syntax
2. Evaluate the configuration
3. Validate option types
4. Check for common errors

## Features

### Automatic D-Bus Integration

All D-Bus services are automatically discovered and managed:
- systemd
- NetworkManager
- PackageKit
- login1
- UDisks2
- Custom services

### MCP Tool Generation

The introspection service automatically generates MCP tools from:
- D-Bus interfaces
- Filesystem resources
- Running processes
- Hardware devices
- Network interfaces

### Declarative State

Define your entire system state in NixOS configuration:

```nix
services.op-dbus = {
  network.interfaces = [ /* ... */ ];
  systemd.units = { /* ... */ };
  packages.installed = [ /* ... */ ];
};
```

op-dbus will reconcile current state to match desired state.

## Security

The NixOS module includes security hardening:

- `PrivateTmp = true` - Private /tmp
- `ProtectSystem = "strict"` - Read-only system directories
- `ProtectHome = true` - No access to home directories
- Minimal capabilities (`CAP_NET_ADMIN`, `CAP_SYS_ADMIN`)
- D-Bus policy restrictions

## Debugging

### Check Service Status

```bash
sudo systemctl status op-dbus
sudo journalctl -u op-dbus -f
```

### Test D-Bus Connection

```bash
busctl list | grep opdbus
busctl introspect org.opdbus.HybridSystem /org/opdbus/HybridSystem
```

### Introspect System

```bash
op-dbus introspect-all
op-dbus hybrid-scan
```

## Rollback

If something goes wrong, rollback to previous generation:

```bash
sudo nixos-rebuild switch --rollback
```

## Development

### Enter Development Shell

```bash
nix develop
```

### Build from Source

```bash
nix build .#packages.x86_64-linux.default
```

### Test in VM

```bash
nixos-rebuild build-vm -I nixos-config=./configuration.nix
./result/bin/run-nixos-vm
```

## Examples

See `configuration.nix` for a complete example configuration.

### Minimal Configuration

```nix
{
  services.op-dbus = {
    enable = true;
    mode = "agent-only";
  };
}
```

### Full-Featured Configuration

```nix
{
  services.op-dbus = {
    enable = true;
    mode = "standalone";

    mcp.enable = true;

    network = {
      interfaces = [ /* ... */ ];
      ovs.enable = true;
    };

    packages = {
      enable = true;
      installed = [ "nginx" "postgresql" ];
    };

    systemd.units = {
      "myservice.service".enabled = true;
    };
  };
}
```

## Troubleshooting

### "Failed to connect to D-Bus"

Ensure D-Bus is running:
```bash
sudo systemctl status dbus
```

### "Permission denied" errors

Check PolicyKit rules:
```bash
pkaction --verbose
```

### Configuration doesn't validate

Run validation script:
```bash
./validate-config.sh
```

Check syntax:
```bash
nix-instantiate --parse configuration.nix
```

## Contributing

To contribute improvements to the NixOS module:

1. Test changes in a VM
2. Validate configuration
3. Update documentation
4. Submit pull request

## License

MIT License - See LICENSE file
