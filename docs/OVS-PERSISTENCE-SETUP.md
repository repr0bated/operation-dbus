# OVS Persistence Setup - OVSDB Database Configuration

## Overview

This document explains how to ensure OVS bridges persist across reboots by properly configuring the OVSDB database storage.

## The Persistence Problem

**Without proper configuration:**
- OVS bridges are created in-memory only
- JSON-RPC transactions don't write to persistent storage
- Database file gets wiped on reboot
- All bridge configurations disappear after reboot

**With proper configuration:**
- OVSDB writes to persistent database file
- Bridge configurations survive reboots
- No need to recreate bridges after each boot

---

## Critical Configuration Requirements

### 1. OVSDB Database File Location

The OVSDB database must be stored at a persistent location:

**Recommended paths:**
- `/etc/openvswitch/conf.db` (Debian/Ubuntu standard)
- `/var/lib/openvswitch/conf.db` (Alternative)

**Create directory if it doesn't exist:**
```bash
sudo mkdir -p /etc/openvswitch
sudo chown openvswitch:openvswitch /etc/openvswitch
```

### 2. Systemd Service Configuration

#### ovsdb-server.service

The ovsdb-server must be configured to use the persistent database file:

**File:** `/etc/systemd/system/ovsdb-server.service.d/override.conf`

```ini
[Unit]
Description=Open vSwitch Database Server
After=network-pre.target
Before=network.target

[Service]
Type=forking
PIDFile=/var/run/openvswitch/ovsdb-server.pid

# CRITICAL: Use persistent database file
ExecStart=/usr/sbin/ovsdb-server \
    --remote=punix:/var/run/openvswitch/db.sock \
    --remote=db:Open_vSwitch,Open_vSwitch,manager_options \
    --pidfile=/var/run/openvswitch/ovsdb-server.pid \
    --detach \
    --log-file=/var/log/openvswitch/ovsdb-server.log \
    /etc/openvswitch/conf.db

# Create database if it doesn't exist
ExecStartPre=/bin/sh -c 'test -e /etc/openvswitch/conf.db || /usr/bin/ovsdb-tool create /etc/openvswitch/conf.db /usr/share/openvswitch/vswitch.ovsschema'

# Ensure directory exists and has correct permissions
ExecStartPre=/bin/mkdir -p /etc/openvswitch
ExecStartPre=/bin/chown -R openvswitch:openvswitch /etc/openvswitch
ExecStartPre=/bin/mkdir -p /var/run/openvswitch
ExecStartPre=/bin/chown -R openvswitch:openvswitch /var/run/openvswitch

Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

**Key parameters:**
- `/etc/openvswitch/conf.db` - Database file path (MUST persist across reboots)
- `--remote=punix:/var/run/openvswitch/db.sock` - Unix socket for JSON-RPC
- `ExecStartPre` - Creates database file if missing using schema

#### ovs-vswitchd.service

The OVS switch daemon must start AFTER the database server:

**File:** `/etc/systemd/system/ovs-vswitchd.service.d/override.conf`

```ini
[Unit]
Description=Open vSwitch Forwarding Unit
After=ovsdb-server.service
Requires=ovsdb-server.service
BindsTo=ovsdb-server.service

[Service]
Type=forking
PIDFile=/var/run/openvswitch/ovs-vswitchd.pid

ExecStart=/usr/sbin/ovs-vswitchd \
    --pidfile=/var/run/openvswitch/ovs-vswitchd.pid \
    --detach \
    --log-file=/var/log/openvswitch/ovs-vswitchd.log \
    unix:/var/run/openvswitch/db.sock

Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

**Critical ordering:**
```
ovsdb-server.service (database with persistent storage)
    ↓
ovs-vswitchd.service (switch daemon, reads database)
    ↓
network.target (networking is ready)
    ↓
op-dbus apply state.json (creates bridges via JSON-RPC)
```

### 3. Verify Database Persistence

After configuring the services, verify persistence works:

```bash
# Reload systemd configuration
sudo systemctl daemon-reload

# Restart OVS services
sudo systemctl restart ovsdb-server.service
sudo systemctl restart ovs-vswitchd.service

# Check database file exists
ls -lh /etc/openvswitch/conf.db

# Create a test bridge via OVSDB JSON-RPC
sudo op-dbus apply state.json

# Verify bridge was created
sudo ovs-vsctl show

# Check database file was modified (mtime should be recent)
ls -lh /etc/openvswitch/conf.db

# REBOOT THE SYSTEM
sudo reboot

# After reboot, verify bridges still exist
sudo ovs-vsctl show

# If bridges persist, configuration is correct! ✓
```

---

## state.json Network Configuration

Example configuration for persistent OVS bridges:

```json
{
  "version": 1,
  "plugins": {
    "network": {
      "bridges": [
        {
          "name": "vmbr0",
          "datapath_type": "system",
          "ports": ["eth0"],
          "internal_ports": ["vmbr0-if"],
          "address": "10.0.0.1/24",
          "openflow": {
            "auto_apply_defaults": true,
            "default_rules": [
              "priority=100,dl_dst=ff:ff:ff:ff:ff:ff,actions=drop",
              "priority=50,actions=normal"
            ]
          }
        },
        {
          "name": "vmbr1",
          "datapath_type": "system",
          "internal_ports": ["vmbr1-if"],
          "address": "10.0.1.1/24",
          "dhcp": false
        }
      ],
      "ovsdb": {
        "socket_path": "/var/run/openvswitch/db.sock",
        "database_path": "/etc/openvswitch/conf.db",
        "timeout_seconds": 30,
        "persist": true
      }
    }
  }
}
```

**Key configuration parameters:**

- `datapath_type: "system"` - **CRITICAL!** Uses kernel-based datapath with persistence
  - Alternative: `"netdev"` (userspace, faster but less persistent)
  - Always use `"system"` for Proxmox/production

- `ovsdb.database_path` - Where OVSDB stores persistent data
  - Default: `/etc/openvswitch/conf.db`
  - Must match systemd service configuration

- `ovsdb.persist: true` - Verify persistence is enabled
  - Plugin will warn if disabled

---

## How It Works

### 1. OVSDB JSON-RPC Transactions

When `op-dbus apply state.json` runs, the network plugin:

```rust
// Create OVSDB client (connects to unix socket)
let client = OvsdbClient::new();

// Send JSON-RPC transaction to OVSDB
let operations = json!([{
    "op": "insert",
    "table": "Bridge",
    "row": {
        "name": "vmbr0",
        "datapath_type": "system",  // ← CRITICAL for persistence
        "ports": ["set", [...]]
    }
}]);

// This writes to /etc/openvswitch/conf.db (persistent storage)
client.transact(operations).await?;
```

### 2. Database Persistence Flow

```
1. op-dbus sends JSON-RPC to /var/run/openvswitch/db.sock
      ↓
2. ovsdb-server receives transaction
      ↓
3. ovsdb-server validates against schema
      ↓
4. ovsdb-server writes to /etc/openvswitch/conf.db  ← PERSISTENCE!
      ↓
5. ovs-vswitchd reads database changes
      ↓
6. ovs-vswitchd creates kernel interfaces
      ↓
7. Bridges exist in kernel and database
```

### 3. After Reboot

```
1. System boots
      ↓
2. ovsdb-server starts, reads /etc/openvswitch/conf.db
      ↓
3. Database contains all bridge configurations
      ↓
4. ovs-vswitchd starts, reads database
      ↓
5. ovs-vswitchd recreates bridges from database
      ↓
6. All bridges automatically restored! ✓
```

---

## Troubleshooting

### Problem: Bridges disappear after reboot

**Cause 1: Database file not persistent**
```bash
# Check database file location
sudo grep -r "conf.db" /etc/systemd/system/

# Should see: /etc/openvswitch/conf.db in ovsdb-server.service
```

**Cause 2: Database file wiped on reboot**
```bash
# Check if directory is tmpfs (RAM-based, non-persistent)
df -h /etc/openvswitch

# Should NOT be tmpfs
# If tmpfs, move database to /var/lib/openvswitch
```

**Cause 3: Using netdev datapath instead of system**
```bash
# Check bridge datapath type
sudo ovs-vsctl get Bridge vmbr0 datapath_type

# Should be: "system"
# If "netdev", recreate with datapath_type="system" in state.json
```

### Problem: OVSDB connection timeout

**Cause: ovsdb-server not running**
```bash
# Check service status
sudo systemctl status ovsdb-server.service

# Check logs
sudo journalctl -u ovsdb-server.service -n 50

# Start service if not running
sudo systemctl start ovsdb-server.service
```

### Problem: Permission denied writing to database

**Cause: Incorrect file permissions**
```bash
# Fix permissions
sudo chown -R openvswitch:openvswitch /etc/openvswitch
sudo chmod 750 /etc/openvswitch
sudo chmod 640 /etc/openvswitch/conf.db
```

---

## Migration from Shell Scripts

### Before (Non-Persistent)

```bash
# Old way: ovs-vsctl commands (may not persist properly)
ovs-vsctl --may-exist add-br vmbr0
ovs-vsctl --may-exist add-port vmbr0 eth0
```

**Problems:**
- May create bridges in-memory only
- Depends on systemd service calling ovs-vsctl on each boot
- Brittle, hard to debug

### After (Persistent via OVSDB)

```json
// state.json
{
  "plugins": {
    "network": {
      "bridges": [
        {
          "name": "vmbr0",
          "datapath_type": "system",
          "ports": ["eth0"]
        }
      ]
    }
  }
}
```

```bash
# Run once (or on each state change)
sudo op-dbus apply state.json
```

**Benefits:**
- Direct OVSDB JSON-RPC writes to database
- Explicit `datapath_type: "system"` ensures kernel persistence
- Idempotent (safe to run multiple times)
- Declarative (state.json defines desired state)
- Automatic persistence (no manual systemd service needed)

---

## Summary

`★ Insight ─────────────────────────────────────`
**OVS Persistence Architecture:**

1. **OVSDB Database File** - Physical storage on disk
   - Location: `/etc/openvswitch/conf.db`
   - Contains all bridge/port/interface configurations
   - Must persist across reboots (not tmpfs!)

2. **ovsdb-server** - Database server daemon
   - Listens on `/var/run/openvswitch/db.sock`
   - Receives JSON-RPC transactions
   - Writes to database file (persistence layer)

3. **ovs-vswitchd** - Virtual switch daemon
   - Reads database on startup
   - Creates kernel network interfaces
   - Depends on ovsdb-server

4. **op-dbus network plugin** - Configuration manager
   - Sends JSON-RPC to ovsdb-server
   - Uses `datapath_type: "system"` for kernel persistence
   - Idempotent operations (safe to re-apply state)

**Key to persistence:**
- Database file must be on persistent storage
- Systemd services must reference correct database path
- Use `datapath_type: "system"` for kernel-based bridges
- JSON-RPC transactions automatically write to database
`─────────────────────────────────────────────────`

---

## See Also

- [HOW-APPLY-STATE-WORKS.md](./HOW-APPLY-STATE-WORKS.md) - Apply command flow
- [OVSDB-MIGRATION-SUMMARY.md](../OVSDB-MIGRATION-SUMMARY.md) - Migration from shell scripts
- [Network Plugin Source](../src/plugins/network.rs) - Implementation details
