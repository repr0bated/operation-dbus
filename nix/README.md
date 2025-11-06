# NixOS Integration for Operation D-Bus

This directory contains NixOS packaging and modules for the Operation D-Bus system.

## 📦 What's Included

### Flake Outputs

- **Packages**
  - `operation-dbus` - All binaries (op-dbus, dbus-mcp, agents, etc.)
  - `nix-introspect` - Standalone system introspection tool

- **NixOS Modules**
  - `operation-dbus` - Main daemon and plugin configuration
  - `mcp-server` - MCP server and agent configuration
  - `default` - Combined module (recommended)

- **Apps**
  - `operation-dbus` - Run the main daemon
  - `nix-introspect` - Run the introspection tool
  - `dbus-mcp` - Run the MCP server

## 🚀 Quick Start

### 1. Add to Your Flake

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    operation-dbus.url = "github:repr0bated/operation-dbus";
  };

  outputs = { self, nixpkgs, operation-dbus }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        operation-dbus.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

### 2. Enable in Your Configuration

```nix
# configuration.nix
{ config, pkgs, ... }:

{
  services.operation-dbus = {
    enable = true;
    plugins = {
      systemd.enable = true;
      network.enable = true;
    };
  };

  services.operation-dbus.mcp = {
    enable = true;
    agents = [ "executor" "systemd" "file" ];
  };
}
```

### 3. Rebuild Your System

```bash
sudo nixos-rebuild switch --flake .#myhost
```

## 🔍 Using the Introspection Tool

The `nix-introspect` tool can scan your running system and generate a NixOS configuration.

### Scan Your System

```bash
# Scan entire system and generate configuration.nix
nix run github:repr0bated/operation-dbus#nix-introspect -- scan \
  --output /tmp/generated-config.nix \
  --include-mcp

# Scan with specific options
nix run github:repr0bated/operation-dbus#nix-introspect -- scan \
  --output config.nix \
  --format nix \
  --include-mcp \
  --include-network \
  --include-systemd \
  --include-containers

# Generate JSON format for inspection
nix run github:repr0bated/operation-dbus#nix-introspect -- scan \
  --output system-state.json \
  --format json
```

### List D-Bus Services

```bash
# List all system bus services
nix run github:repr0bated/operation-dbus#nix-introspect -- list-services \
  --bus system

# List with filtering
nix run github:repr0bated/operation-dbus#nix-introspect -- list-services \
  --bus both \
  --filter "systemd"
```

### Introspect Specific Services

```bash
# Introspect systemd
nix run github:repr0bated/operation-dbus#nix-introspect -- inspect \
  org.freedesktop.systemd1 \
  --path /org/freedesktop/systemd1 \
  --format json

# Introspect NetworkManager
nix run github:repr0bated/operation-dbus#nix-introspect -- inspect \
  org.freedesktop.NetworkManager \
  --bus system \
  --format xml
```

### Generate Template

```bash
# Generate a template configuration
nix run github:repr0bated/operation-dbus#nix-introspect -- template \
  --output template.nix \
  --documented
```

## 🔧 Configuration Options

### Main Service Options

```nix
services.operation-dbus = {
  enable = true;

  # Package to use
  package = pkgs.operation-dbus;

  # State file (null to generate from config)
  stateFile = null;

  # Data directories
  dataDir = "/var/lib/op-dbus";
  cacheDir = "/var/cache/op-dbus";

  # Logging
  logLevel = "info";  # error, warn, info, debug, trace

  # Run mode
  oneshot = false;  # Set to true for apply-and-exit

  # Plugins
  plugins = {
    systemd.enable = true;
    network.enable = false;
    lxc.enable = false;
    login1.enable = false;
    dnsresolver.enable = false;
    openflow.enable = false;
  };

  # Blockchain audit trail
  blockchain = {
    enable = true;
    useML = false;  # Requires ML dependencies
  };

  # Cache configuration
  cache = {
    enable = true;
    maxSize = "10G";
  };
};
```

### MCP Server Options

```nix
services.operation-dbus.mcp = {
  enable = true;

  # Agents to enable
  agents = [
    "executor"   # Command execution
    "systemd"    # Service management
    "file"       # File operations
    "network"    # Network management
    "monitor"    # System monitoring
  ];

  # Web bridge
  enableWebBridge = false;
  serverPort = 3000;

  # D-Bus discovery
  enableDiscovery = true;
  discoveryConfig = {
    scan_interval = 300;
    default_format = "xml";
    output_dir = "/var/lib/op-dbus/mcp-discovery";
  };

  # Orchestrator
  orchestrator = {
    enable = true;
    maxConcurrentTasks = 10;
  };

  # Tool limits
  tools = {
    maxToolCount = 100;
    blocklist = [
      "org.freedesktop.DBus"
      "org.freedesktop.secrets"
    ];
  };

  # Chat interface
  chat = {
    enable = false;
    port = 8080;
  };

  logLevel = "info";
};
```

## 📚 Example Configurations

See the `examples/` directory for complete configuration examples:

- **basic-configuration.nix** - Simple setup with systemd and network plugins
- **advanced-configuration.nix** - Full setup with containers, OpenFlow, and all agents

## 🔄 Complete Workflow

### 1. Introspect Existing System

On your current system (doesn't need to be NixOS):

```bash
# Install nix-introspect
nix profile install github:repr0bated/operation-dbus#nix-introspect

# Scan system
nix-introspect scan \
  --output /tmp/discovered-config.nix \
  --include-mcp \
  --include-network \
  --include-systemd

# Review the generated configuration
cat /tmp/discovered-config.nix
```

### 2. Customize Configuration

Edit the generated configuration to fit your needs:

```nix
# configuration.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    # Import generated configuration as a starting point
  ];

  # Customize as needed
  services.operation-dbus = {
    enable = true;

    plugins = {
      systemd.enable = true;
      network.enable = true;
      lxc.enable = true;
    };

    # Add custom state configuration
    plugins.systemd.config = {
      units = {
        "my-service.service" = {
          active_state = "active";
          enabled = true;
        };
      };
    };
  };

  # Enable MCP
  services.operation-dbus.mcp = {
    enable = true;
    agents = [ "executor" "systemd" "file" "network" ];
    enableWebBridge = true;
  };
}
```

### 3. Deploy to NixOS

```bash
# Add the flake to your system
cd /etc/nixos
git init
git add flake.nix configuration.nix

# Build and switch
sudo nixos-rebuild switch --flake .#

# Verify operation-dbus is running
systemctl status operation-dbus
systemctl status dbus-mcp-server

# Check logs
journalctl -u operation-dbus -f
```

### 4. Use the MCP Interface

```bash
# Query system state
op-dbus query

# Apply declarative state
op-dbus apply /path/to/state.json

# Access MCP server (if web bridge is enabled)
curl http://localhost:3000/tools

# Use the chat interface (if enabled)
# Navigate to http://localhost:8080
```

## 🧪 Development

### Build Locally

```bash
# Clone repository
git clone https://github.com/repr0bated/operation-dbus
cd operation-dbus

# Enter development shell
nix develop

# Build specific components
cargo build --bin op-dbus
cargo build --bin nix-introspect --features mcp
cargo build --features mcp,web

# Test
cargo test
```

### Test in NixOS VM

```bash
# Build a VM with your configuration
nixos-rebuild build-vm --flake .#myhost

# Run the VM
./result/bin/run-myhost-vm
```

## 🎯 Features

### Operation D-Bus

- ✅ Declarative system state management
- ✅ Native protocol support (D-Bus, Netlink, OVSDB)
- ✅ Plugin system for extensibility
- ✅ Blockchain audit trail
- ✅ BTRFS-based caching
- ✅ Rollback and checkpoint support

### MCP Integration

- ✅ Model Context Protocol server
- ✅ Multiple specialized agents
- ✅ Automatic D-Bus service discovery
- ✅ Tool generation from introspection
- ✅ Multi-agent orchestration
- ✅ Web/WebSocket bridge

### NixOS Introspection

- ✅ Scan running systems
- ✅ Generate NixOS configurations
- ✅ D-Bus service discovery
- ✅ Systemd unit detection
- ✅ Network interface discovery
- ✅ Container detection (LXC)
- ✅ Plugin auto-detection
- ✅ Multi-format output (Nix, JSON, TOML)

## 📖 Related Documentation

- [Operation D-Bus Main README](../README.md)
- [MCP Documentation](../documentation/mcp.md)
- [Plugin Development Guide](../documentation/plugins.md)
- [State File Format](../documentation/state-format.md)

## 🤝 Contributing

Contributions are welcome! Please see the main project README for contribution guidelines.

## 📝 License

MIT License - see LICENSE file for details
