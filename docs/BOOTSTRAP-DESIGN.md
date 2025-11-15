# Bootstrap Design for Declarative Installation

## Goal
Boot from ISO/USB → Complete Proxmox VE + operation-dbus installation via single `state.json` file.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Phase 0: Boot Environment                  │
│  - Minimal Debian live ISO                  │
│  - Contains: op-dbus binary, state.json     │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  Phase 1: Disk Preparation                  │
│  - Partition disks (via storage plugin)     │
│  - Create filesystems (BTRFS/LVM)           │
│  - Mount target system                      │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  Phase 2: Base System Install               │
│  - debootstrap minimal Debian               │
│  - Install PackageKit                       │
│  - Copy op-dbus binary to target            │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  Phase 3: Chroot & Apply State              │
│  - chroot into target system                │
│  - op-dbus apply state.json                 │
│    ├─ packagekit: Install Proxmox           │
│    ├─ network: Configure bridges            │
│    ├─ storage: Create BTRFS subvolumes      │
│    ├─ systemd: Enable services              │
│    ├─ users: Create accounts                │
│    └─ firewall: Configure nftables          │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  Phase 4: Bootloader & Finalize             │
│  - Install GRUB                             │
│  - Configure fstab                          │
│  - Unmount and reboot                       │
└─────────────────────────────────────────────┘
```

## Required Plugins

### ✅ Already Built
- **packagekit**: Install packages via PackageKit D-Bus

### ❌ Need to Build

#### **storage** Plugin
Manages disks, partitions, filesystems.

```json
{
  "storage": {
    "devices": {
      "/dev/sda": {
        "partition_table": "gpt",
        "partitions": [
          {"number": 1, "size": "512M", "type": "efi", "filesystem": "vfat", "mount": "/boot/efi"},
          {"number": 2, "size": "remaining", "type": "linux", "filesystem": "btrfs", "mount": "/"}
        ]
      }
    },
    "btrfs": {
      "/dev/sda2": {
        "subvolumes": [
          {"name": "@", "mount": "/"},
          {"name": "@home", "mount": "/home"},
          {"name": "@snapshots", "mount": "/.snapshots"},
          {"name": "@blockchain", "mount": "/var/lib/blockchain"}
        ]
      }
    }
  }
}
```

**D-Bus Interfaces Used**:
- `org.freedesktop.UDisks2` - Disk management
- Direct syscalls for BTRFS operations

#### **network** Plugin (Already exists in src/plugins/network.rs!)
Configure network interfaces, bridges, VLANs.

```json
{
  "network": {
    "interfaces": {
      "vmbr0": {
        "type": "bridge",
        "address": "192.168.1.100/24",
        "gateway": "192.168.1.1",
        "bridge_ports": ["ens18"]
      }
    }
  }
}
```

**D-Bus Interfaces Used**:
- `org.freedesktop.NetworkManager` (we already use this!)
- Or direct netlink for Proxmox compatibility

#### **systemd** Plugin
Enable/disable services, create units.

```json
{
  "systemd": {
    "services": {
      "pve-cluster": {"state": "enabled"},
      "pvedaemon": {"state": "enabled"},
      "pveproxy": {"state": "enabled"},
      "op-dbus": {"state": "enabled"}
    },
    "units": {
      "my-custom.service": {
        "content": "[Unit]\nDescription=...\n[Service]\nExecStart=...",
        "state": "enabled"
      }
    }
  }
}
```

**D-Bus Interfaces Used**:
- `org.freedesktop.systemd1.Manager` - We already use this!

#### **users** Plugin
Create users, groups, permissions.

```json
{
  "users": {
    "accounts": {
      "admin": {
        "uid": 1000,
        "groups": ["sudo", "docker"],
        "shell": "/bin/bash",
        "ssh_keys": ["ssh-ed25519 AAAA..."]
      }
    }
  }
}
```

**D-Bus Interfaces Used**:
- `org.freedesktop.Accounts` - User management

#### **firewall** Plugin
Configure nftables/iptables rules.

```json
{
  "firewall": {
    "rules": [
      {"port": 22, "proto": "tcp", "action": "accept", "comment": "SSH"},
      {"port": 8006, "proto": "tcp", "action": "accept", "comment": "Proxmox Web UI"},
      {"port": 8096, "proto": "tcp", "action": "accept", "comment": "op-dbus MCP Web"}
    ],
    "default_policy": "drop"
  }
}
```

**Implementation**: Direct nftables/iptables commands (no D-Bus needed)

#### **lxc** Plugin (Already partially exists!)
We have LXC support in `src/plugins/lxc.rs`!

```json
{
  "lxc": {
    "containers": {
      "app-server": {
        "template": "debian-12",
        "config": {
          "memory": "2G",
          "cpus": 2,
          "network": {"bridge": "vmbr0", "ip": "192.168.1.101/24"}
        },
        "autostart": true
      }
    }
  }
}
```

**D-Bus Interfaces Used**:
- Direct LXC API calls (lxc-create, lxc-start)

---

## Complete state.json Example

```json
{
  "version": 1,
  "metadata": {
    "name": "proxmox-complete-install",
    "description": "Full Proxmox VE 9 + operation-dbus installation",
    "created": "2025-11-15T12:00:00Z"
  },
  "bootstrap": {
    "device": "/dev/sda",
    "hostname": "pve-node1",
    "timezone": "America/New_York"
  },
  "plugins": {
    "storage": {
      "devices": {
        "/dev/sda": {
          "partition_table": "gpt",
          "partitions": [
            {"number": 1, "size": "512M", "type": "efi", "filesystem": "vfat", "mount": "/boot/efi"},
            {"number": 2, "size": "remaining", "type": "linux", "filesystem": "btrfs", "mount": "/"}
          ]
        }
      },
      "btrfs": {
        "/dev/sda2": {
          "subvolumes": [
            {"name": "@", "mount": "/"},
            {"name": "@blockchain", "mount": "/var/lib/blockchain"}
          ]
        }
      }
    },
    "packagekit": {
      "manifest": "/root/proxmox-ve-9-manifest.json"
    },
    "network": {
      "interfaces": {
        "vmbr0": {
          "type": "bridge",
          "address": "192.168.1.100/24",
          "gateway": "192.168.1.1",
          "bridge_ports": ["ens18"]
        }
      }
    },
    "systemd": {
      "services": {
        "pve-cluster": {"state": "enabled"},
        "pvedaemon": {"state": "enabled"},
        "pveproxy": {"state": "enabled"},
        "pvestatd": {"state": "enabled"},
        "op-dbus": {"state": "enabled"}
      }
    },
    "users": {
      "accounts": {
        "admin": {
          "uid": 1000,
          "groups": ["sudo"],
          "shell": "/bin/bash"
        }
      }
    },
    "firewall": {
      "rules": [
        {"port": 22, "proto": "tcp", "action": "accept"},
        {"port": 8006, "proto": "tcp", "action": "accept"}
      ],
      "default_policy": "drop"
    },
    "lxc": {
      "containers": {
        "test-container": {
          "template": "debian-12",
          "config": {
            "memory": "1G",
            "cpus": 1
          }
        }
      }
    }
  }
}
```

---

## Implementation Roadmap

### Phase 1: Core Plugins (Priority 1)
- [x] PackageKit plugin (done!)
- [ ] Storage plugin (disk partitioning, BTRFS)
- [ ] Network plugin (enhance existing)
- [ ] Systemd plugin (enhance existing)

### Phase 2: User Management (Priority 2)
- [ ] Users plugin
- [ ] Firewall plugin

### Phase 3: Advanced (Priority 3)
- [ ] LXC plugin (enhance existing)
- [ ] KVM plugin
- [ ] Backup plugin

### Phase 4: Bootstrap Installer (Priority 4)
- [ ] Bootable ISO with op-dbus
- [ ] Installer script
- [ ] Web-based installer UI

---

## Usage

### Build Bootstrap ISO
```bash
./build-bootstrap-iso.sh \
    --state complete-server.json \
    --output op-dbus-installer.iso
```

### Boot and Install
1. Boot from op-dbus-installer.iso
2. Installer loads state.json
3. Runs: `op-dbus bootstrap /dev/sda state.json`
4. System reboots into fully configured Proxmox VE + op-dbus

### Or Manual Install
```bash
# From live environment
sudo op-dbus bootstrap /dev/sda state.json
```

---

## Benefits

✅ **Reproducibility**: Same state.json → identical systems
✅ **Auditability**: All changes via D-Bus are logged
✅ **Version Control**: state.json can be git-tracked
✅ **Testing**: Test installations in VMs before production
✅ **Disaster Recovery**: Reinstall from state.json backup

---

See also:
- [Storage Plugin Design](./STORAGE-PLUGIN.md)
- [Bootstrap Process](./BOOTSTRAP-PROCESS.md)
- [Example States](../examples/complete-server-states/)
