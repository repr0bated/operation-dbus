# NixOS Integration - Quick Reference

## 🚀 Quick Commands

### Installation

```bash
# Run introspection tool (no install)
nix run github:repr0bated/operation-dbus#nix-introspect -- scan

# Install to profile
nix profile install github:repr0bated/operation-dbus#nix-introspect

# Install operation-dbus
nix profile install github:repr0bated/operation-dbus
```

### Introspection

```bash
# Full system scan
sudo nix-introspect scan --output config.nix --include-mcp

# List D-Bus services
nix-introspect list-services --bus both

# Inspect specific service
nix-introspect inspect org.freedesktop.systemd1

# Generate template
nix-introspect template --output template.nix
```

### Deployment

```bash
# Add to flake.nix
inputs.operation-dbus.url = "github:repr0bated/operation-dbus";

# Rebuild system
sudo nixos-rebuild switch --flake .#hostname

# Build VM for testing
nixos-rebuild build-vm --flake .#hostname
```

### Management

```bash
# Check services
systemctl status operation-dbus
systemctl status dbus-mcp-server

# View logs
journalctl -u operation-dbus -f

# Query state
op-dbus query

# Apply state
op-dbus apply state.json

# Verify configuration
op-dbus verify
```

### MCP Interface

```bash
# List tools
curl http://localhost:3000/tools | jq

# Call a tool
curl -X POST http://localhost:3000/call \
  -H "Content-Type: application/json" \
  -d '{"tool": "systemd.list_units", "arguments": {}}'

# Access chat interface
# http://localhost:8080
```

## 📝 Minimal Configuration

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    operation-dbus.url = "github:repr0bated/operation-dbus";
  };

  outputs = { nixpkgs, operation-dbus, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        operation-dbus.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

```nix
# configuration.nix
{ config, ... }:

{
  services.operation-dbus = {
    enable = true;
    plugins.systemd.enable = true;
  };

  services.operation-dbus.mcp = {
    enable = true;
    agents = [ "executor" "systemd" ];
  };
}
```

## 🎯 Common Use Cases

### 1. System State Management

```nix
services.operation-dbus = {
  enable = true;
  plugins.systemd = {
    enable = true;
    config.units = {
      "nginx.service" = {
        active_state = "active";
        enabled = true;
      };
    };
  };
};
```

### 2. Network with OVS

```nix
services.operation-dbus.plugins.network = {
  enable = true;
  config.interfaces = [{
    name = "ovsbr0";
    type = "ovs-bridge";
    ports = [ "eth0" ];
    ipv4.address = [{ ip = "10.0.0.1"; prefix = 24; }];
  }];
};
```

### 3. Container Management

```nix
services.operation-dbus.plugins.lxc = {
  enable = true;
  config.containers = [{
    name = "web-server";
    state = "running";
    config.image = "ubuntu/22.04";
  }];
};
```

### 4. OpenFlow Controller

```nix
services.operation-dbus.plugins.openflow = {
  enable = true;
  config = {
    bridges = [{
      name = "mesh";
      controller = "tcp:127.0.0.1:6653";
    }];
    flows = [];
  };
};
```

### 5. Full MCP Setup

```nix
services.operation-dbus.mcp = {
  enable = true;
  agents = [ "executor" "systemd" "file" "network" "monitor" ];
  enableWebBridge = true;
  serverPort = 3000;
  enableDiscovery = true;
  orchestrator.enable = true;
  chat.enable = true;
};
```

## 🔧 Module Options Reference

### Main Service

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | false | Enable operation-dbus |
| `package` | package | pkgs.operation-dbus | Package to use |
| `stateFile` | path\|null | null | State file path |
| `dataDir` | path | /var/lib/op-dbus | Data directory |
| `cacheDir` | path | /var/cache/op-dbus | Cache directory |
| `logLevel` | enum | "info" | Log level |
| `oneshot` | bool | false | Run in oneshot mode |

### Plugins

| Plugin | Description | Requires |
|--------|-------------|----------|
| `systemd` | Service management | systemd |
| `network` | Network/OVS | openvswitch |
| `lxc` | Containers | lxc |
| `login1` | User sessions | systemd-logind |
| `dnsresolver` | DNS config | systemd-resolved |
| `openflow` | Flow control | openvswitch |

### MCP Server

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enable` | bool | false | Enable MCP server |
| `agents` | list | [] | Agents to enable |
| `enableWebBridge` | bool | false | Enable web bridge |
| `serverPort` | int | 3000 | Server port |
| `enableDiscovery` | bool | true | Enable D-Bus discovery |
| `orchestrator.enable` | bool | true | Enable orchestrator |
| `chat.enable` | bool | false | Enable chat interface |

### Available Agents

- **executor** - Command execution (requires root)
- **systemd** - Service management (requires root)
- **file** - File operations
- **network** - Network management (requires root)
- **monitor** - System monitoring

## 🐛 Troubleshooting

### Service Won't Start

```bash
# Check status
systemctl status operation-dbus

# View logs
journalctl -xeu operation-dbus

# Verify D-Bus
busctl list | grep opdbus
```

### MCP Not Accessible

```bash
# Check service
systemctl status dbus-mcp-server

# Test locally
curl http://localhost:3000/tools

# Check firewall
sudo iptables -L -n | grep 3000
```

### Introspection Fails

```bash
# Run with debug
RUST_LOG=debug nix-introspect scan

# Check D-Bus access
dbus-send --system --print-reply \
  --dest=org.freedesktop.DBus \
  /org/freedesktop/DBus \
  org.freedesktop.DBus.ListNames
```

## 📚 Resources

- **Full Documentation**: [nix/README.md](nix/README.md)
- **Workflow Guide**: [NIXOS_WORKFLOW.md](NIXOS_WORKFLOW.md)
- **Examples**: [nix/examples/](nix/examples/)
- **Main README**: [README.md](README.md)

## 🔗 Links

- GitHub: https://github.com/repr0bated/operation-dbus
- MCP Docs: https://modelcontextprotocol.io
- NixOS Manual: https://nixos.org/manual/nixos/stable/
