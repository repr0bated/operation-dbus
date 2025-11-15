# Declarative Full-Server Installation Roadmap

## Vision

**Goal**: Install and configure a complete Proxmox VE server with operation-dbus using a single declarative state.json file.

```bash
# The dream:
sudo op-dbus bootstrap /dev/sda complete-server.json

# Result: Fully configured Proxmox VE 9 + operation-dbus
#  - Packages installed via PackageKit
#  - Network configured (bridges, VLANs)
#  - Storage configured (BTRFS subvolumes)
#  - Services enabled (systemd)
#  - Users created
#  - Firewall configured
#  - Containers deployed
```

---

## Current State ✅

### What We Have Built

1. **✅ PackageKit Plugin** (`src/plugins/packagekit.rs`)
   - Install packages via D-Bus
   - Declarative package management
   - Integrated with op-dbus

2. **✅ Proxmox Extractor Toolkit** (`tools/proxmox-extractor/`)
   - Extract package lists from Proxmox ISO
   - Parse Debian packages with dependencies
   - Generate PackageKit manifest
   - Install via PackageKit D-Bus (zbus)

3. **✅ Network Plugin** (`src/plugins/net.rs`)
   - Basic network interface management
   - Uses rtnetlink for native kernel networking
   - Needs enhancement for Proxmox bridges

4. **✅ LXC Plugin** (`src/plugins/lxc.rs`)
   - Basic LXC container management
   - Needs enhancement for declarative deployment

5. **✅ Systemd Integration** (Used throughout)
   - Service management via D-Bus
   - Needs dedicated plugin

---

## What We Need to Build ❌

### Priority 1: Core Bootstrap Plugins

#### **1. Storage Plugin** ❌ Critical

**Purpose**: Partition disks, create filesystems, manage BTRFS subvolumes

**State Format**:
```json
{
  "storage": {
    "devices": {
      "/dev/sda": {
        "partition_table": "gpt",
        "partitions": [
          {"number": 1, "size": "512M", "type": "efi"},
          {"number": 2, "size": "remaining", "type": "linux"}
        ]
      }
    },
    "filesystems": {
      "/dev/sda2": {"type": "btrfs", "label": "proxmox"}
    },
    "btrfs_subvolumes": {
      "/dev/sda2": [
        {"name": "@", "mount": "/"},
        {"name": "@blockchain", "mount": "/var/lib/blockchain"}
      ]
    },
    "mounts": {
      "/": {"device": "/dev/sda2", "subvol": "@", "options": "compress=zstd:3"},
      "/boot/efi": {"device": "/dev/sda1", "fstype": "vfat"}
    }
  }
}
```

**Implementation**:
- Use `org.freedesktop.UDisks2` D-Bus for disk operations
- Direct BTRFS syscalls for subvolumes
- Location: `src/plugins/storage.rs`

**Estimated Complexity**: Medium (3-4 days)

---

#### **2. Enhanced Network Plugin** ❌ High Priority

**Purpose**: Configure OVS bridges, VLANs, bonds for Proxmox

**State Format**:
```json
{
  "network": {
    "interfaces": {
      "vmbr0": {
        "type": "bridge",
        "bridge_ports": ["ens18"],
        "address": "192.168.1.100/24",
        "gateway": "192.168.1.1"
      },
      "vmbr1": {
        "type": "bridge",
        "address": "10.0.0.1/24",
        "comment": "Internal network"
      }
    },
    "bonds": {
      "bond0": {
        "slaves": ["eth0", "eth1"],
        "mode": "802.3ad"
      }
    }
  }
}
```

**Implementation**:
- Enhance existing `src/plugins/net.rs`
- Add OVS bridge support (vs standard Linux bridges)
- Use rtnetlink for configuration
- Location: `src/plugins/network.rs` (enhanced)

**Estimated Complexity**: Medium (2-3 days)

---

#### **3. Systemd Plugin** ❌ Medium Priority

**Purpose**: Enable/disable services, create custom units

**State Format**:
```json
{
  "systemd": {
    "services": {
      "pvedaemon": {"state": "enabled"},
      "pveproxy": {"state": "enabled"},
      "op-dbus": {"state": "enabled"}
    },
    "timers": {
      "backup.timer": {
        "on_calendar": "daily",
        "unit": "backup.service"
      }
    },
    "custom_units": {
      "my-app.service": {
        "content": "[Unit]\nDescription=My App\n[Service]\nExecStart=/usr/local/bin/my-app"
      }
    }
  }
}
```

**Implementation**:
- Use `org.freedesktop.systemd1.Manager` D-Bus
- Already partially used in codebase
- Location: `src/plugins/systemd.rs`

**Estimated Complexity**: Low (1-2 days)

---

### Priority 2: User & Security

#### **4. Users Plugin** ❌

**Purpose**: Create users, groups, SSH keys

**State Format**:
```json
{
  "users": {
    "accounts": {
      "admin": {
        "uid": 1000,
        "gid": 1000,
        "shell": "/bin/bash",
        "groups": ["sudo", "docker"],
        "ssh_authorized_keys": ["ssh-ed25519 AAAA..."]
      }
    },
    "groups": {
      "operators": {"gid": 2000}
    }
  }
}
```

**Implementation**:
- Use `org.freedesktop.Accounts` D-Bus
- Direct `useradd`/`groupadd` as fallback
- SSH key management
- Location: `src/plugins/users.rs`

**Estimated Complexity**: Low (1-2 days)

---

#### **5. Firewall Plugin** ❌

**Purpose**: Configure nftables/iptables

**State Format**:
```json
{
  "firewall": {
    "rules": [
      {"port": 22, "proto": "tcp", "action": "accept", "comment": "SSH"},
      {"port": 8006, "proto": "tcp", "action": "accept", "comment": "Proxmox Web UI"}
    ],
    "default_policy": {
      "input": "drop",
      "forward": "accept",
      "output": "accept"
    }
  }
}
```

**Implementation**:
- Direct nftables commands (no D-Bus needed)
- Generate nftables.conf
- Location: `src/plugins/firewall.rs`

**Estimated Complexity**: Medium (2-3 days)

---

### Priority 3: Virtualization

#### **6. Enhanced LXC Plugin** ❌

**Purpose**: Declarative LXC container deployment

**State Format**:
```json
{
  "lxc": {
    "containers": {
      "web-server": {
        "template": "debian-12",
        "storage": "local-lvm",
        "config": {
          "memory": "2G",
          "cpus": 2,
          "rootfs": "8G",
          "network": {
            "bridge": "vmbr0",
            "ip": "192.168.1.101/24",
            "gateway": "192.168.1.1"
          }
        },
        "autostart": true,
        "state": "started"
      }
    }
  }
}
```

**Implementation**:
- Enhance existing `src/plugins/lxc.rs`
- Add declarative container creation
- Network configuration per container
- Location: `src/plugins/lxc.rs` (enhanced)

**Estimated Complexity**: Medium (3-4 days)

---

#### **7. KVM/QEMU Plugin** ❌ Lower Priority

**Purpose**: Declarative VM management

**State Format**:
```json
{
  "kvm": {
    "vms": {
      "vm-100": {
        "name": "test-vm",
        "os_type": "l26",
        "cores": 2,
        "memory": 4096,
        "disks": [
          {"storage": "local-lvm", "size": "32G"}
        ],
        "network": [
          {"bridge": "vmbr0", "model": "virtio"}
        ],
        "autostart": false
      }
    }
  }
}
```

**Implementation**:
- Use libvirt D-Bus API
- Or direct `qm` commands (Proxmox CLI)
- Location: `src/plugins/kvm.rs`

**Estimated Complexity**: High (5-7 days)

---

### Priority 4: Bootstrap System

#### **8. Bootstrap Command** ❌ Critical for First-Stage Install

**Purpose**: New `op-dbus bootstrap` command that:
1. Partitions disk
2. Creates filesystems
3. Mounts target
4. debootstrap minimal Debian
5. Chroot and apply state.json
6. Install bootloader
7. Unmount and reboot

**Command**:
```bash
sudo op-dbus bootstrap /dev/sda state.json
```

**Implementation**:
- Location: `src/commands/bootstrap.rs`
- Calls all plugins in correct order
- Handles chroot environment

**Estimated Complexity**: High (5-7 days)

---

#### **9. Bootable Installer ISO** ❌ Nice to Have

**Purpose**: Bootable ISO containing op-dbus + state.json

**Features**:
- Minimal Debian live environment
- op-dbus binary pre-installed
- Web UI for editing state.json
- One-click install

**Build Script**:
```bash
./build-installer-iso.sh \
    --state examples/complete-server.json \
    --output op-dbus-installer.iso
```

**Estimated Complexity**: High (7-10 days)

---

## Complete Example state.json

```json
{
  "version": 1,
  "metadata": {
    "name": "proxmox-complete",
    "description": "Complete Proxmox VE 9 with operation-dbus",
    "author": "ops-team"
  },
  "bootstrap": {
    "device": "/dev/sda",
    "hostname": "pve-node1",
    "domain": "example.com",
    "timezone": "America/New_York",
    "locale": "en_US.UTF-8"
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
      "btrfs_subvolumes": {
        "/dev/sda2": [
          {"name": "@", "mount": "/"},
          {"name": "@home", "mount": "/home"},
          {"name": "@blockchain", "mount": "/var/lib/blockchain"}
        ]
      }
    },
    "packagekit": {
      "manifest": "/root/proxmox-ve-9-manifest.json",
      "additional_packages": ["vim", "tmux", "htop"]
    },
    "network": {
      "interfaces": {
        "vmbr0": {
          "type": "bridge",
          "bridge_ports": ["ens18"],
          "address": "192.168.1.100/24",
          "gateway": "192.168.1.1",
          "dns": ["8.8.8.8", "1.1.1.1"]
        },
        "vmbr1": {
          "type": "bridge",
          "address": "10.0.0.1/24",
          "comment": "Internal network"
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
          "shell": "/bin/bash",
          "ssh_authorized_keys": ["ssh-ed25519 AAAA..."]
        }
      }
    },
    "firewall": {
      "rules": [
        {"port": 22, "proto": "tcp", "action": "accept", "comment": "SSH"},
        {"port": 8006, "proto": "tcp", "action": "accept", "comment": "Proxmox Web UI"},
        {"port": 8096, "proto": "tcp", "action": "accept", "comment": "op-dbus Web UI"}
      ],
      "default_policy": {
        "input": "drop",
        "forward": "accept",
        "output": "accept"
      }
    },
    "lxc": {
      "containers": {
        "app-server": {
          "template": "debian-12",
          "config": {
            "memory": "2G",
            "cpus": 2,
            "rootfs": "8G",
            "network": {
              "bridge": "vmbr0",
              "ip": "192.168.1.101/24"
            }
          },
          "autostart": true,
          "state": "started"
        }
      }
    }
  }
}
```

---

## Implementation Timeline

### Week 1-2: Core Plugins
- [ ] Storage plugin (disk, filesystem, BTRFS)
- [ ] Enhanced network plugin (OVS bridges)
- [ ] Systemd plugin

### Week 3-4: User & Security
- [ ] Users plugin
- [ ] Firewall plugin

### Week 5-6: Virtualization
- [ ] Enhanced LXC plugin
- [ ] KVM plugin (basic)

### Week 7-8: Bootstrap
- [ ] Bootstrap command
- [ ] Integration testing
- [ ] Documentation

### Week 9-10: Polish
- [ ] Installer ISO
- [ ] Web UI for state.json editing
- [ ] Example states

---

## Benefits of This Approach

✅ **Reproducibility**: Same state.json = identical servers
✅ **Version Control**: Track server config in git
✅ **Testing**: Test in VMs before production
✅ **Disaster Recovery**: Reinstall from state.json
✅ **Auditability**: All changes logged via D-Bus
✅ **No Ansible/Chef/Puppet**: Pure D-Bus + native protocols
✅ **Type Safe**: Rust prevents configuration errors
✅ **Fast**: Direct D-Bus calls, no SSH overhead

---

## Next Steps

1. **Immediate**: Build storage plugin (highest priority)
2. **This week**: Enhance network plugin for Proxmox
3. **Next week**: Systemd plugin
4. **Following**: Bootstrap command

---

See Also:
- [Bootstrap Design](./docs/BOOTSTRAP-DESIGN.md)
- [Proxmox Extractor](./tools/proxmox-extractor/README.md)
- [Plugin Development Guide](./PLUGIN-DEVELOPMENT-GUIDE.md)
