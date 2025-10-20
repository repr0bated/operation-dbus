# op-dbus - Operation D-Bus

Declarative system state management via native protocols.

## Quick Start

### Build
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
