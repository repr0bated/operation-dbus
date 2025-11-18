# OVS Bridge State Templates

## Overview

These templates demonstrate how to configure Open vSwitch (OVS) bridges using native OVSDB JSON-RPC protocol via the `op-dbus` state management system.

**Key Benefits:**
- ✅ **Native Protocol**: Uses OVSDB JSON-RPC (not CLI tools)
- ✅ **Persistence**: Configuration survives reboots (written to OVSDB + `/etc/network/interfaces`)
- ✅ **Kernel Visibility**: Bridges appear in kernel once ports are attached
- ✅ **Idempotent**: Safe to apply multiple times - only applies changes
- ✅ **Atomic**: Changes are transactional via OVSDB

## Prerequisites

```bash
# Ensure OVS is installed and running
sudo apt install openvswitch-switch
sudo systemctl enable openvswitch-switch
sudo systemctl start openvswitch-switch

# Verify OVSDB socket exists
ls -la /var/run/openvswitch/db.sock
```

## Available Templates

Use: `op-dbus apply templates/<template-name>.json`

- **ovs-bridge-basic.json** - Simple bridge with one port and static IP
- **ovs-bridge-multiple-ports.json** - Bridge with multiple ports (bonding/trunking)  
- **ovs-bridge-with-internal-port.json** - Bridge with internal management port
- **ovs-add-port-to-existing-bridge.json** - Add port to existing bridge
- **ovs-bridge-with-dhcp.json** - Bridge with DHCP configuration

## Native OVSDB JSON-RPC vs CLI

### Adding a Port

**❌ CLI (Forbidden)**:
```bash
ovs-vsctl add-port ovsbr0 eth0
```

**✅ Native OVSDB JSON-RPC (Automatic via net plugin)**:
The `net` plugin automatically generates OVSDB transactions to add ports atomically.

## Persistence

Configuration persists across reboots via:
1. **OVSDB** (`/var/lib/openvswitch/conf.db`)
2. **/etc/network/interfaces** (IP configuration)
3. **systemd** (`openvswitch-switch.service`)

Bridges appear in kernel once ports are attached.
