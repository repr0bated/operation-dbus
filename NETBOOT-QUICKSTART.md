# operation-dbus Netboot Quick Start

**One-page reference for netboot.xyz → local disk deployment**

## Prerequisites

- ✅ netboot.xyz running (you have this)
- ✅ Local disk on target machines (you have this)
- 🔲 HTTP server to host images
- 🔲 Build machine with Nix

## Complete Workflow (3 Steps)

### Step 1: Build Installer Image

```bash
# On build machine with Nix
cd operation-dbus
./nixos/netboot/build-installer.sh

# Output: /tmp/opdbus-installer/{bzImage,initrd}
# Size: ~150MB total
```

### Step 2: Host on HTTP Server

```bash
# Copy to web server
scp /tmp/opdbus-installer/* user@webserver:/var/www/netboot/installer/

# OR test locally
cd /tmp/opdbus-installer && python3 -m http.server 8080
```

### Step 3: Add to netboot.xyz

```ipxe
# Edit your netboot.xyz custom.ipxe
:opdbus-installer
kernel http://YOUR-SERVER-IP/netboot/installer/bzImage init=/nix/store/.../init
initrd http://YOUR-SERVER-IP/netboot/installer/initrd
boot
```

## Installation on Target Machine

```bash
# 1. PXE boot → netboot.xyz → Custom → opdbus-installer
# 2. Wait 1-2 minutes for boot
# 3. SSH in (or use console)
ssh root@<installer-ip>  # Password: nixos

# 4. Run installer
lsblk  # Check disk names
sudo /etc/opdbus-install.sh /dev/sda opdbus-node-01

# 5. Reboot (remove netboot from boot order)
sudo reboot

# 6. SSH into new system
ssh root@<new-ip>

# 7. Verify installation
df -h | grep btrfs
systemctl status operation-dbus
```

## What Gets Installed

```
Disk: /dev/sda
├── /dev/sda1  512MB   EFI boot partition (FAT32)
└── /dev/sda2  Rest    BTRFS with subvolumes:
    ├── @root          /                      (system root)
    ├── @cache         /var/lib/op-dbus/cache (ML embeddings)
    ├── @timing        /var/lib/op-dbus/timing (blockchain DB)
    ├── @vectors       /var/lib/op-dbus/vectors (vector index)
    ├── @state         /var/lib/op-dbus/state (current state)
    └── @snapshots     /var/lib/op-dbus/snapshots (backups)
```

## Architecture Benefits

| Feature | Pure Netboot | netboot → Disk | Why Better |
|---------|-------------|----------------|------------|
| Boot time | Slow (downloads) | Fast (local) | No network wait |
| ML cache | Lost on reboot | Persistent | Embeddings persist |
| BTRFS snapshots | Can't use | Full history | Plugin versioning |
| State storage | RAM only | Disk backed | Survives reboot |
| NUMA optimization | Limited | Full support | Memory locality |

## Configuration Files

### Build Machine

```
operation-dbus/
├── nixos/netboot/
│   ├── build-installer.sh          ← Build script (run this)
│   └── configs/
│       ├── installer.nix           ← Installer config
│       ├── proxmox-host.nix        ← Production config
│       └── workstation.nix         ← Simple config
├── nixos/modules/
│   └── operation-dbus.nix          ← Main NixOS module
├── NETBOOT-TO-DISK-INSTALL.md      ← Detailed guide
└── NETBOOT-QUICKSTART.md           ← This file
```

### Target Machine (after install)

```
/etc/nixos/
├── configuration.nix               ← Main config (edit this)
├── hardware-configuration.nix      ← Auto-generated
└── operation-dbus/                 ← Cloned repo (optional)
    └── nixos/modules/operation-dbus.nix
```

## Post-Install: Enable operation-dbus

```bash
# SSH into installed system
ssh root@<new-ip>

# Edit configuration
vim /etc/nixos/configuration.nix

# Uncomment operation-dbus section:
services.operation-dbus = {
  enable = true;

  numa.enable = true;        # For multi-socket systems
  btrfs.enable = true;       # Already configured
  ml.executionProvider = "cpu";  # Or "cuda" for GPU

  defaultState = {
    version = "1.0";
    plugins = {};
  };
};

# Apply configuration
nixos-rebuild switch

# Verify service
systemctl status operation-dbus
op-dbus doctor
```

## NUMA Optimization (Multi-Socket Systems)

```bash
# Check NUMA topology
numactl --hardware

# Configure in /etc/nixos/configuration.nix
services.operation-dbus.numa = {
  enable = true;
  node = 0;           # Pin to socket 0
  cpuList = "0-7";    # First 8 cores
};

# Verify after rebuild
numastat -p $(pgrep op-dbus)
# Should show most memory on node 0
```

## GPU Acceleration (Optional)

```nix
# In /etc/nixos/configuration.nix
services.operation-dbus.ml = {
  enable = true;
  executionProvider = "cuda";  # NVIDIA GPU
  gpuDeviceId = 0;
};

# For CPU-only (default)
services.operation-dbus.ml = {
  enable = true;
  executionProvider = "cpu";
  numThreads = 8;
};
```

## BTRFS Verification

```bash
# Check subvolumes
btrfs subvolume list /

# Check compression ratio
compsize /var/lib/op-dbus/cache

# Check mounts
df -h | grep op-dbus

# Manual snapshot (automatic via hourly timer)
btrfs subvolume snapshot /var/lib/op-dbus/cache /var/lib/op-dbus/snapshots/cache-$(date +%Y%m%d-%H%M)
```

## Troubleshooting

### Installer won't boot
```bash
# Check HTTP server
curl -I http://your-server/netboot/installer/bzImage
# Should return: 200 OK

# Check netboot.xyz menu
# Verify URL is correct in custom.ipxe
```

### Can't SSH into installer
```bash
# Check IP address on console
# Or scan network
nmap -sP 192.168.1.0/24
```

### Installation fails
```bash
# Check disk
lsblk
# Verify disk name is correct

# Check space
df -h
# Need at least 10GB free

# View detailed errors
nixos-install --show-trace
```

### operation-dbus won't start
```bash
# Check logs
journalctl -u operation-dbus -xe

# Common fixes:
systemctl status dbus           # D-Bus must be running
ls /etc/operation-dbus/state.json  # State file must exist
df -h | grep op-dbus           # Subvolumes must be mounted
```

## Multi-Machine Deployment

```bash
# Create inventory
MACHINES=(
  "192.168.1.10:node-01:/dev/sda"
  "192.168.1.11:node-02:/dev/nvme0n1"
  "192.168.1.12:node-03:/dev/sda"
)

# Deploy to all
for m in "${MACHINES[@]}"; do
  IFS=: read -r ip hostname disk <<< "$m"

  # Boot via IPMI (if available)
  ipmitool -H "$ip-mgmt" chassis bootdev pxe
  ipmitool -H "$ip-mgmt" power reset

  # Wait for installer
  until ssh -o ConnectTimeout=5 root@"$ip" true 2>/dev/null; do
    sleep 10
  done

  # Install
  ssh root@"$ip" "/etc/opdbus-install.sh $disk $hostname"
done
```

## Performance Tips

### SSD/NVMe Optimization
```nix
# Add to filesystem mount options
fileSystems."/var/lib/op-dbus/cache".options = [
  "subvol=@cache"
  "compress=zstd:3"
  "noatime"
  "ssd"              # Enable SSD optimizations
  "discard=async"    # Async TRIM
];

# Enable fstrim
services.fstrim.enable = true;
```

### ML Cache Warmup
```bash
# After first boot
op-dbus cache warmup

# Monitor progress
journalctl -u operation-dbus -f | grep embedding

# Check size
du -sh /var/lib/op-dbus/cache
```

## Quick Commands Reference

```bash
# Build installer
./nixos/netboot/build-installer.sh

# Host on HTTP
python3 -m http.server 8080

# Install to disk
/etc/opdbus-install.sh /dev/sda hostname

# Enable operation-dbus
vim /etc/nixos/configuration.nix
nixos-rebuild switch

# Check status
systemctl status operation-dbus
op-dbus doctor

# NUMA stats
numactl --hardware
numastat -p $(pgrep op-dbus)

# BTRFS info
btrfs subvolume list /
compsize /var/lib/op-dbus/cache

# Logs
journalctl -u operation-dbus -f
```

## File Checklist

Before deployment, ensure you have:

- ✅ Built installer image (`/tmp/opdbus-installer/`)
- ✅ Hosted on HTTP server (accessible from target network)
- ✅ Added to netboot.xyz custom menu
- ✅ SSH key configured (or know root password: nixos)
- ✅ Target disk identified (`lsblk` to check)
- ✅ Network access to target machines

## Success Criteria

After installation, verify:

- ✅ System boots from local disk (not netboot)
- ✅ BTRFS subvolumes mounted: `df -h | grep op-dbus`
- ✅ operation-dbus service running: `systemctl status operation-dbus`
- ✅ NUMA configured (multi-socket): `numastat -p $(pgrep op-dbus)`
- ✅ ML provider loaded: `journalctl -u operation-dbus | grep ML_PROVIDER`
- ✅ Compression active: `compsize /var/lib/op-dbus/cache`

## Documentation References

- **Detailed guide**: `NETBOOT-TO-DISK-INSTALL.md` (8000+ words)
- **NixOS setup**: `NIXOS-SETUP-GUIDE.md`
- **netboot.xyz integration**: `NETBOOT-XYZ-INTEGRATION.md`
- **Architecture**: `HYBRID-BTRFS-ARCHITECTURE.md`
- **Module options**: `nixos/modules/operation-dbus.nix`

## Support

If you need help:

1. Check logs: `journalctl -u operation-dbus -xe`
2. Run diagnostics: `op-dbus doctor`
3. Review documentation in repository
4. Check GitHub issues

---

**TL;DR**:
```bash
# Build
./nixos/netboot/build-installer.sh

# Host
scp /tmp/opdbus-installer/* user@webserver:/var/www/netboot/installer/

# Add to netboot.xyz custom.ipxe
# kernel http://server/netboot/installer/bzImage
# initrd http://server/netboot/installer/initrd

# Boot target machine → netboot.xyz → Custom → opdbus-installer
# SSH: root@<ip> password: nixos
# Install: /etc/opdbus-install.sh /dev/sda hostname
# Reboot → Done!
```
