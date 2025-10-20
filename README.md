# op-dbus - Operation D-Bus

Declarative system state management via native protocols.

## Deployment Modes

op-dbus supports three deployment modes:

1. **Full (Proxmox)** - Default: D-Bus + Blockchain + LXC/Proxmox + Netmaker
2. **Standalone (Enterprise)** - `--no-proxmox`: D-Bus + Blockchain (no containers)
3. **Agent Only** - `--agent-only`: D-Bus plugins only (minimal)

See **[ENTERPRISE-DEPLOYMENT.md](ENTERPRISE-DEPLOYMENT.md)** for detailed enterprise deployment guide.

## Quick Start

### Installation

**Full Installation (Proxmox Mode):**
```bash
cargo build --release
sudo ./install.sh
```

**Enterprise Standalone (No Proxmox/Containers):**
```bash
cargo build --release
sudo ./install.sh --no-proxmox
```

**Minimal Agent (D-Bus Only):**
```bash
cargo build --release
sudo ./install.sh --agent-only
```

The install script will:
1. Install the binary to `/usr/local/bin/op-dbus`
2. **Automatically detect** your current OVS bridges, IP addresses, and gateway
3. Generate `/etc/op-dbus/state.json` with your detected configuration
4. Create blockchain storage (unless `--agent-only`)
5. Create `mesh` bridge for netmaker containers (Proxmox mode only)
6. **Auto-detect and add** netmaker interfaces to mesh bridge (Proxmox mode only)
7. Create and configure the systemd service

**Test what will be detected:**
```bash
sudo ./test-introspection.sh
```

**Netmaker Mesh Networking (Optional):**

If you want automatic mesh networking for containers:

1. Install netclient:
```bash
curl -sL https://apt.netmaker.org/gpg.key | sudo apt-key add -
curl -sL https://apt.netmaker.org/debian.deb.txt | sudo tee /etc/apt/sources.list.d/netmaker.list
sudo apt update && sudo apt install netclient
```

2. Add your enrollment token to `/etc/op-dbus/netmaker.env`:
```bash
echo "NETMAKER_TOKEN=your-token-here" | sudo tee /etc/op-dbus/netmaker.env
```

3. Run install.sh - it will automatically:
   - Join the host to netmaker
   - Detect netmaker interfaces (nm-*)
   - Add them to the mesh bridge

Or manually sync netmaker interfaces anytime:
```bash
sudo ./sync-netmaker-mesh.sh
```

### Manual Build
```bash
cargo build --release
# Binary: target/release/op-dbus
```

### Commands

**Query current state:**
```bash
op-dbus query                    # All plugins
op-dbus query --plugin net       # Specific plugin
```

**Show diff:**
```bash
op-dbus diff example-state.json
```

**Apply state:**
```bash
op-dbus apply example-state.json
```

**Run daemon:**
```bash
op-dbus run --state-file /etc/op-dbus/state.json
```

## Architecture

### Native Protocols
- **OVSDB**: Direct JSON-RPC to `/var/run/openvswitch/db.sock`
- **Netlink**: rtnetlink for IP/routes (no `ip` command wrapper)
- **D-Bus**: zbus for system services (systemd, NetworkManager, etc.)

### Plugins
- **net**: Network state (OVS bridges, IP addresses, routes)
- **systemd**: Systemd units (start/stop/enable/disable)
- **Extensible**: Any D-Bus service can become a plugin

### Features
- Declarative JSON state files
- SHA-256 cryptographic footprints
- Immutable blockchain audit log
- Per-plugin architecture
- Rollback support

## State File Format

```json
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [{
        "name": "vmbr0",
        "type": "ovs-bridge",
        "ports": ["ens1"],
        "ipv4": {
          "enabled": true,
          "dhcp": false,
          "address": [{"ip": "80.209.240.244", "prefix": 25}],
          "gateway": "80.209.240.129"
        }
      }]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "active_state": "active",
          "enabled": true
        }
      }
    }
  }
}
```

## Universal D-Bus System

op-dbus treats the entire D-Bus tree as a database schema. Each D-Bus service can become a plugin:
- **systemd** (org.freedesktop.systemd1)
- **UDisks2** (org.freedesktop.UDisks2) - storage
- **login1** (org.freedesktop.login1) - sessions
- **NetworkManager** (org.freedesktop.NetworkManager)
- **UPower** (org.freedesktop.UPower) - power
- Any D-Bus service

This creates a universal declarative interface to the entire Linux D-Bus ecosystem.
