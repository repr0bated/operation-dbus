# op-dbus Quick Start

## Build & Install

```bash
cd /git/op-dbus

# 1. Build
cargo build --release

# 2. Install (as root)
sudo ./install.sh

# 3. Configure
sudo nano /etc/op-dbus/state.json
```

## Test Safely (Read-Only)

```bash
# Run safe tests
sudo ./test-safe.sh

# Or manually:
op-dbus query                           # All plugins
op-dbus query --plugin net              # Network only
op-dbus diff /etc/op-dbus/state.json    # Show what would change
```

## Apply State (CAREFUL - Makes Changes!)

```bash
# Test apply manually first (DO NOT enable service yet)
sudo op-dbus apply /etc/op-dbus/state.json

# If successful, enable service
sudo systemctl enable op-dbus
sudo systemctl start op-dbus
sudo systemctl status op-dbus

# Watch logs
sudo journalctl -u op-dbus -f
```

## Uninstall

```bash
sudo ./uninstall.sh
```

## Example State File

```json
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [{
        "name": "ovsbr0",
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

## Troubleshooting

**Build fails:**
```bash
# Check dependencies
cargo --version
rustc --version

# Clean build
cargo clean
cargo build --release
```

**Permission errors:**
```bash
# Check capabilities
sudo getcap /usr/local/bin/op-dbus

# D-Bus access
ls -la /var/run/dbus/system_bus_socket

# OVSDB access
ls -la /var/run/openvswitch/db.sock
```

**Service won't start:**
```bash
# Check logs
sudo journalctl -u op-dbus -n 50

# Test manually
sudo /usr/local/bin/op-dbus run --state-file /etc/op-dbus/state.json
```
