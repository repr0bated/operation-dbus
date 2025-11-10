# Proxmox VE Installation via op-dbus PackageKit

**Quick Start Guide** - Install Proxmox VE declaratively via D-Bus

---

## Prerequisites

```bash
# 1. Add Proxmox repository
echo "deb [arch=amd64] http://download.proxmox.com/debian/pve bookworm pve-no-subscription" \
  > /etc/apt/sources.list.d/pve.list

# 2. Add GPG key
wget https://enterprise.proxmox.com/debian/proxmox-release-bookworm.gpg \
  -O /etc/apt/trusted.gpg.d/proxmox-release-bookworm.gpg

# 3. Update package lists
apt update
```

---

## Installation Methods

### Method 1: One Command (Full Stack)

```bash
sudo ./target/release/op-dbus apply proxmox-ve-full-install.json
```

This installs all 11 packages:
- ifupdown2, postfix, open-iscsi, chrony
- proxmox-ve, pve-manager, pve-kernel-helper
- qemu-server, lxc-pve, corosync, pve-cluster

### Method 2: Step-by-Step

```bash
# 1. See what will be installed (dry run)
./target/release/op-dbus diff proxmox-ve-full-install.json

# 2. Apply state
sudo ./target/release/op-dbus apply proxmox-ve-full-install.json

# 3. Verify installation
./target/release/op-dbus verify proxmox-ve-full-install.json
```

---

## State File Format

```json
{
  "version": 1,
  "plugins": {
    "packagekit": {
      "packages": {
        "proxmox-ve": {
          "ensure": "installed",
          "provider": "apt"
        }
      }
    }
  }
}
```

---

## Access Proxmox

After installation:

1. **Reboot** (for Proxmox kernel)
2. **Access Web UI**: `https://your-ip:8006`
3. **Login**: root + your password

---

## Commands Reference

```bash
# Query current packages
./target/release/op-dbus query --plugin packagekit

# Show diff (what will change)
./target/release/op-dbus diff <state-file.json>

# Apply state (install/remove packages)
sudo ./target/release/op-dbus apply <state-file.json>

# Verify state matches desired
./target/release/op-dbus verify <state-file.json>
```

---

## Example: Add Additional Packages

Edit `proxmox-ve-full-install.json`:

```json
{
  "version": 1,
  "plugins": {
    "packagekit": {
      "packages": {
        "proxmox-ve": {"ensure": "installed"},
        "pve-manager": {"ensure": "installed"},
        "ceph": {"ensure": "installed"},
        "zfs-dkms": {"ensure": "installed"}
      }
    }
  }
}
```

Then apply: `sudo ./target/release/op-dbus apply proxmox-ve-full-install.json`

---

## Troubleshooting

### PackageKit Not Available

Plugin automatically falls back to direct package managers:
- Debian/Ubuntu: `apt-get`
- Fedora/RHEL: `dnf`
- Arch: `pacman`

### Check Plugin Status

```bash
./target/release/op-dbus query --plugin packagekit
```

Should return: `{"packages": {}, "version": 1}`

### Manual Package Check

```bash
# Debian/Ubuntu
dpkg -l proxmox-ve

# Check services
systemctl status pve-cluster
systemctl status pvedaemon
```

---

**That's it! Declarative Proxmox VE installation via D-Bus!** 🎉
