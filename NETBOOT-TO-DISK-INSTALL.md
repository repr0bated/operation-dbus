# netboot.xyz → Local Disk Installation

Complete guide for installing operation-dbus on local disk via netboot.xyz boot.

## Overview

This workflow gives you the best of both worlds:
- **Boot from netboot.xyz**: No USB drive needed, always latest image
- **Install to local disk**: Full BTRFS caching, persistent ML embeddings, snapshots
- **Automated setup**: One command to configure the entire system

## Prerequisites

- Machine with local disk (HDD/SSD/NVMe)
- Network boot capability (PXE)
- netboot.xyz already configured
- HTTP server hosting NixOS images

## Quick Start

```bash
# 1. Build installer image (on build machine)
cd operation-dbus
./nixos/netboot/build-installer.sh

# 2. Host on HTTP server
cp /tmp/opdbus-installer/* /var/www/netboot/installer/

# 3. Boot target machine via netboot.xyz
# Select: Custom → operation-dbus Installer

# 4. On target machine (auto-runs or manual):
sudo opdbus-install /dev/sda
```

That's it! The system installs NixOS with operation-dbus, reboots, and is ready.

## Architecture: Why This Approach?

### netboot.xyz Boot Phase
```
┌─────────────────┐
│  netboot.xyz    │ ←── Machine PXE boots
│  (existing)     │
└────────┬────────┘
         │ HTTP download
         ↓
┌─────────────────┐
│  NixOS Installer│ ←── Runs in RAM (diskless)
│  + op-dbus      │     Contains installation scripts
└────────┬────────┘
         │
         ↓ Installs to
┌─────────────────┐
│  Local Disk     │ ←── Persistent BTRFS storage
│  /dev/sda       │     Full operation-dbus
│  ┌─────────────┐│
│  │ @root       ││     NixOS root
│  │ @cache      ││ ←── ML embeddings (compressed)
│  │ @timing     ││     Blockchain timing DB
│  │ @vectors    ││     Vector cache
│  │ @snapshots  ││     Plugin snapshots
│  └─────────────┘│
└─────────────────┘
```

### Benefits Over Pure Netboot

| Feature | Pure Netboot (RAM) | netboot → Disk |
|---------|-------------------|----------------|
| BTRFS cache | ❌ Ephemeral | ✅ Persistent |
| ML embeddings | ❌ Lost on reboot | ✅ Cached forever |
| Snapshots | ❌ Can't store | ✅ Full history |
| Boot time | ⚠️ Slow (downloads) | ✅ Fast (local) |
| State storage | ❌ RAM only | ✅ Persistent |
| Plugin updates | ⚠️ Full reboot | ✅ Atomic snapshots |

## Step 1: Build Installer Image

### Create installer configuration

```nix
# nixos/netboot/configs/installer.nix
{ config, pkgs, lib, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/netboot/netboot-minimal.nix")
    ../../modules/operation-dbus.nix
  ];

  # Installer-specific settings
  boot = {
    supportedFilesystems = [ "btrfs" "ext4" "xfs" ];
    kernelParams = [
      "boot.shell_on_fail"
      "console=tty0"
      "console=ttyS0,115200"
    ];
  };

  # Networking for installation
  networking = {
    useDHCP = true;
    firewall.enable = false; # Permissive during install
  };

  # SSH access during installation
  services.openssh = {
    enable = true;
    settings.PermitRootLogin = "yes";
  };

  users.users.root = {
    # CHANGE THIS TO YOUR SSH KEY!
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyChangeThis your-key"
    ];
    # Fallback password: "nixos" (change in production!)
    password = "nixos";
  };

  # Installation tools
  environment.systemPackages = with pkgs; [
    # Partitioning
    parted
    gptfdisk

    # Filesystems
    btrfs-progs
    e2fsprogs
    dosfstools

    # Diagnostics
    smartmontools
    hdparm
    nvme-cli
    lsblk

    # Network
    curl
    wget
    git

    # operation-dbus
    # Note: Full installation happens to disk, not in RAM
  ];

  # Automated installation script
  environment.etc."opdbus-install.sh" = {
    source = pkgs.writeScript "opdbus-install.sh" ''
      #!/usr/bin/env bash
      # operation-dbus automated installation script
      set -e

      DISK="''${1:-/dev/sda}"
      HOSTNAME="''${2:-opdbus-node}"

      echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
      echo "operation-dbus NixOS Installation"
      echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
      echo "Target disk: $DISK"
      echo "Hostname: $HOSTNAME"
      echo ""

      # Confirmation
      read -p "This will ERASE $DISK. Continue? (yes/NO): " confirm
      if [ "$confirm" != "yes" ]; then
        echo "Installation cancelled."
        exit 1
      fi

      echo ""
      echo "[1/7] Partitioning $DISK..."
      parted "$DISK" --script -- \
        mklabel gpt \
        mkpart ESP fat32 1MiB 512MiB \
        set 1 esp on \
        mkpart primary 512MiB 100%

      # Wait for partitions to appear
      sleep 2

      # Detect partition names
      if [ -e "''${DISK}1" ]; then
        ESP="''${DISK}1"
        ROOT="''${DISK}2"
      elif [ -e "''${DISK}p1" ]; then
        ESP="''${DISK}p1"
        ROOT="''${DISK}p2"
      else
        echo "❌ Cannot detect partition layout"
        exit 1
      fi

      echo "[2/7] Formatting partitions..."
      mkfs.fat -F 32 -n BOOT "$ESP"
      mkfs.btrfs -f -L nixos "$ROOT"

      echo "[3/7] Creating BTRFS subvolumes..."
      mount "$ROOT" /mnt
      btrfs subvolume create /mnt/@root
      btrfs subvolume create /mnt/@cache
      btrfs subvolume create /mnt/@timing
      btrfs subvolume create /mnt/@vectors
      btrfs subvolume create /mnt/@state
      btrfs subvolume create /mnt/@snapshots
      umount /mnt

      echo "[4/7] Mounting filesystems..."
      mount -o subvol=@root,compress=zstd:1,noatime "$ROOT" /mnt
      mkdir -p /mnt/boot
      mount "$ESP" /mnt/boot

      mkdir -p /mnt/var/lib/op-dbus
      mount -o subvol=@cache,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus/cache
      mount -o subvol=@timing,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus/timing
      mount -o subvol=@vectors,compress=zstd:3,noatime "$ROOT" /mnt/var/lib/op-dbus/vectors
      mount -o subvol=@state,compress=zstd:1,noatime "$ROOT" /mnt/var/lib/op-dbus/state
      mount -o subvol=@snapshots,compress=zstd:1,noatime "$ROOT" /mnt/var/lib/op-dbus/snapshots

      echo "[5/7] Generating NixOS configuration..."
      nixos-generate-config --root /mnt

      # Download operation-dbus configuration
      mkdir -p /mnt/etc/nixos/operation-dbus
      if [ -n "$OPDBUS_CONFIG_URL" ]; then
        echo "Fetching configuration from $OPDBUS_CONFIG_URL..."
        curl -L "$OPDBUS_CONFIG_URL" | tar xz -C /mnt/etc/nixos/operation-dbus
      else
        echo "⚠️  No OPDBUS_CONFIG_URL set, using defaults"
      fi

      # Generate configuration.nix
      cat > /mnt/etc/nixos/configuration.nix <<'EOF_CONFIG'
      { config, pkgs, ... }:
      {
        imports = [
          ./hardware-configuration.nix
          ./operation-dbus/nixos/modules/operation-dbus.nix
        ];

        # Boot loader
        boot.loader.systemd-boot.enable = true;
        boot.loader.efi.canTouchEfiVariables = true;

        # Hostname
        networking.hostName = "$HOSTNAME";
        networking.useDHCP = true;

        # SSH
        services.openssh = {
          enable = true;
          settings.PermitRootLogin = "prohibit-password";
        };

        # operation-dbus configuration
        services.operation-dbus = {
          enable = true;

          # NUMA optimization (adjust for your hardware)
          numa = {
            enable = true;  # Set false for single-socket
            node = 0;
            cpuList = "0-7";
          };

          # BTRFS (already set up during install)
          btrfs = {
            enable = true;
            basePath = "/var/lib/op-dbus";
            compressionLevel = 3;
            subvolumes = [ "@cache" "@timing" "@vectors" "@state" ];
          };

          # ML vectorization
          ml = {
            enable = true;
            executionProvider = "cpu";  # Change to "cuda" for GPU
            numThreads = 8;
          };

          # Default state
          defaultState = {
            version = "1.0";
            plugins = {};
          };

          logLevel = "info";
        };

        # Root user SSH key
        users.users.root.openssh.authorizedKeys.keys = [
          # ADD YOUR SSH KEY HERE!
        ];

        # System packages
        environment.systemPackages = with pkgs; [
          vim
          git
          htop
          tmux
          btrfs-progs
          numactl
        ];

        system.stateVersion = "24.11";
      }
      EOF_CONFIG

      echo "[6/7] Installing NixOS..."
      nixos-install --no-root-passwd

      echo "[7/7] Installation complete!"
      echo ""
      echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
      echo "Next steps:"
      echo "1. Reboot: sudo reboot"
      echo "2. Remove netboot.xyz boot entry (or set disk as primary)"
      echo "3. SSH into system: ssh root@<ip>"
      echo "4. Configure operation-dbus: vim /etc/operation-dbus/state.json"
      echo "5. Start service: systemctl start operation-dbus"
      echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
      echo ""

      read -p "Reboot now? (yes/NO): " reboot_confirm
      if [ "$reboot_confirm" = "yes" ]; then
        reboot
      fi
    '';
    mode = "0755";
  };

  # Auto-install on boot (optional - requires confirmation)
  systemd.services.auto-install-prompt = {
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    serviceConfig = {
      Type = "oneshot";
      StandardInput = "tty";
      StandardOutput = "tty";
    };
    script = ''
      echo ""
      echo "════════════════════════════════════════════"
      echo "  operation-dbus Installer"
      echo "════════════════════════════════════════════"
      echo ""
      echo "This system is running in RAM from netboot.xyz"
      echo ""
      echo "To install to local disk:"
      echo "  sudo /etc/opdbus-install.sh /dev/sda"
      echo ""
      echo "To install over SSH:"
      echo "  ssh root@$(hostname -I | awk '{print $1}')"
      echo "  Password: nixos"
      echo ""
    '';
  };

  system.stateVersion = "24.11";
}
```

### Build the installer

```bash
# Create build script
cat > nixos/netboot/build-installer.sh <<'EOF'
#!/usr/bin/env bash
set -e

OUTPUT_DIR="/tmp/opdbus-installer"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Building operation-dbus installer image"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

nix-build '<nixpkgs/nixos>' \
  -A config.system.build.netbootRamdisk \
  -I nixos-config="./nixos/netboot/configs/installer.nix" \
  -o "$OUTPUT_DIR" \
  --show-trace

echo ""
echo "✅ Installer built!"
echo ""
echo "Files:"
ls -lh "$OUTPUT_DIR"/{bzImage,initrd}

KERNEL_SIZE=$(du -h "$OUTPUT_DIR/bzImage" | cut -f1)
INITRD_SIZE=$(du -h "$OUTPUT_DIR/initrd" | cut -f1)

echo ""
echo "Sizes:"
echo "  Kernel: $KERNEL_SIZE"
echo "  Initrd: $INITRD_SIZE"
echo ""
echo "Next: Host on HTTP server"
echo "  scp $OUTPUT_DIR/* user@server:/var/www/netboot/installer/"
EOF

chmod +x nixos/netboot/build-installer.sh
```

### Run the build

```bash
cd operation-dbus
./nixos/netboot/build-installer.sh
```

This builds a netboot image (~150MB) containing:
- NixOS installer environment
- BTRFS tools, partitioning tools
- operation-dbus configuration templates
- Automated installation script

## Step 2: Host Installer Image

### Option A: Existing Web Server

```bash
# Copy to web server
WEB_ROOT="/var/www/netboot"
mkdir -p "$WEB_ROOT/installer"

cp /tmp/opdbus-installer/bzImage "$WEB_ROOT/installer/"
cp /tmp/opdbus-installer/initrd "$WEB_ROOT/installer/"

chmod -R 755 "$WEB_ROOT/installer"

# Test access
curl -I http://your-server/netboot/installer/bzImage
# Should return 200 OK
```

### Option B: Python HTTP Server (Testing)

```bash
cd /tmp
python3 -m http.server 8080

# Available at:
# http://your-ip:8080/opdbus-installer/bzImage
# http://your-ip:8080/opdbus-installer/initrd
```

### Option C: nginx on NixOS

```nix
{
  services.nginx = {
    enable = true;
    virtualHosts."netboot.local" = {
      root = "/var/www/netboot";
      locations = {
        "/".extraConfig = "autoindex on;";
        "/installer/".extraConfig = ''
          autoindex on;
          add_header Cache-Control "public, max-age=3600";
        '';
      };
    };
  };

  networking.firewall.allowedTCPPorts = [ 80 443 ];
}
```

## Step 3: Add to netboot.xyz Menu

### Create custom installer entry

```ipxe
# /path/to/netboot.xyz/custom.ipxe

:custom
menu operation-dbus
item --gap -- Installers:
item opdbus-installer operation-dbus NixOS Installer (to disk)
item --gap -- Live Systems:
item opdbus-live operation-dbus NixOS Live (RAM only)
item --gap --
item return Return to main menu
choose custom_choice || goto custom
goto custom_${custom_choice}

:opdbus-installer
echo Booting operation-dbus installer...
echo This will allow installation to local disk
kernel http://YOUR-SERVER/netboot/installer/bzImage init=/nix/store/.../init console=tty0
initrd http://YOUR-SERVER/netboot/installer/initrd
boot

:opdbus-live
echo Booting operation-dbus live system (RAM)...
kernel http://YOUR-SERVER/netboot/proxmox/bzImage init=/nix/store/.../init
initrd http://YOUR-SERVER/netboot/proxmox/initrd
boot

:return
chain utils.ipxe
```

**Remember**: Replace `YOUR-SERVER` with your actual IP/hostname!

### Deploy to netboot.xyz

```bash
# If using Docker netboot.xyz
docker cp custom.ipxe netboot_xyz:/config/menus/
docker restart netboot_xyz

# If self-hosted
cp custom.ipxe /var/www/netboot.xyz/assets/custom.ipxe
```

## Step 4: Install to Target Machine

### Boot the machine

1. **PXE boot** the target machine
2. **Select** "Custom" → "operation-dbus NixOS Installer"
3. **Wait** for kernel/initrd to download (1-2 min)
4. **System boots** into installer environment

### Connect via SSH

```bash
# The installer will display its IP address on the console
# Or find it via network scan
nmap -sP 192.168.1.0/24

# SSH in (password: nixos, or use your SSH key)
ssh root@<installer-ip>
```

### Run installation

```bash
# On the target machine (via SSH or console)

# Check available disks
lsblk

# Run installer (CHANGE /dev/sda to your disk!)
sudo /etc/opdbus-install.sh /dev/sda opdbus-node-01

# The script will:
# 1. Partition the disk (GPT with EFI)
# 2. Format as BTRFS
# 3. Create subvolumes (@root, @cache, @timing, @vectors, @state)
# 4. Install NixOS with operation-dbus
# 5. Configure bootloader
# 6. Prompt for reboot
```

### Installation output

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
operation-dbus NixOS Installation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Target disk: /dev/sda
Hostname: opdbus-node-01

This will ERASE /dev/sda. Continue? (yes/NO): yes

[1/7] Partitioning /dev/sda...
[2/7] Formatting partitions...
[3/7] Creating BTRFS subvolumes...
[4/7] Mounting filesystems...
[5/7] Generating NixOS configuration...
[6/7] Installing NixOS...
  this derivation will be built:
    /nix/store/...-nixos-system-opdbus-node-01-24.11
  building '/nix/store/...-nixos-system-opdbus-node-01-24.11.drv'...
  ...
[7/7] Installation complete!

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Next steps:
1. Reboot: sudo reboot
2. Remove netboot.xyz boot entry (or set disk as primary)
3. SSH into system: ssh root@<ip>
4. Configure operation-dbus: vim /etc/operation-dbus/state.json
5. Start service: systemctl start operation-dbus
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Reboot now? (yes/NO): yes
```

### Post-install: First boot

```bash
# After reboot, SSH into the new system
ssh root@<new-ip>

# Verify installation
df -h | grep btrfs
# /dev/sda2 on / type btrfs (subvol=@root)
# /dev/sda2 on /var/lib/op-dbus/cache type btrfs (subvol=@cache)
# /dev/sda2 on /var/lib/op-dbus/timing type btrfs (subvol=@timing)
# /dev/sda2 on /var/lib/op-dbus/vectors type btrfs (subvol=@vectors)

# Check operation-dbus service
systemctl status operation-dbus

# Run diagnostics
op-dbus doctor

# Check NUMA (if multi-socket)
numactl --hardware
numastat -p $(pgrep op-dbus)

# Check BTRFS compression
compsize /var/lib/op-dbus/cache
```

## Disk Layout Details

### Partition Scheme

```
/dev/sda (example)
├── /dev/sda1  512MB   ESP (EFI System Partition)  FAT32
└── /dev/sda2  Rest    Root + Subvolumes           BTRFS
```

### BTRFS Subvolume Layout

```
/dev/sda2 (BTRFS)
├── @root                          → /                 (compress=zstd:1)
├── @cache                         → /var/lib/op-dbus/cache    (compress=zstd:3)
│   ├── embeddings/                   ML embeddings (384-dim vectors)
│   ├── queries/                      Query cache
│   ├── blocks/                       Block cache
│   └── diffs/                        Diff cache
├── @timing                        → /var/lib/op-dbus/timing   (compress=zstd:3)
│   └── blockchain.db                 SQLite timing database
├── @vectors                       → /var/lib/op-dbus/vectors  (compress=zstd:3)
│   └── vector-index.db               Vector search index
├── @state                         → /var/lib/op-dbus/state    (compress=zstd:1)
│   └── current-state.json            Current infrastructure state
└── @snapshots                     → /var/lib/op-dbus/snapshots (compress=zstd:1)
    ├── plugin-lxc-20250107/          Plugin snapshots
    ├── plugin-netmaker-20250107/
    └── cache-20250107-0900/          Hourly cache snapshots
```

### Mount Options

```nix
{
  fileSystems = {
    "/" = {
      device = "/dev/disk/by-label/nixos";
      fsType = "btrfs";
      options = [ "subvol=@root" "compress=zstd:1" "noatime" ];
    };

    "/var/lib/op-dbus/cache" = {
      device = "/dev/disk/by-label/nixos";
      fsType = "btrfs";
      options = [ "subvol=@cache" "compress=zstd:3" "noatime" ];
    };

    "/var/lib/op-dbus/timing" = {
      device = "/dev/disk/by-label/nixos";
      fsType = "btrfs";
      options = [ "subvol=@timing" "compress=zstd:3" "noatime" ];
    };

    "/var/lib/op-dbus/vectors" = {
      device = "/dev/disk/by-label/nixos";
      fsType = "btrfs";
      options = [ "subvol=@vectors" "compress=zstd:3" "noatime" ];
    };

    "/var/lib/op-dbus/state" = {
      device = "/dev/disk/by-label/nixos";
      fsType = "btrfs";
      options = [ "subvol=@state" "compress=zstd:1" "noatime" ];
    };

    "/var/lib/op-dbus/snapshots" = {
      device = "/dev/disk/by-label/nixos";
      fsType = "btrfs";
      options = [ "subvol=@snapshots" "compress=zstd:1" "noatime" ];
    };
  };
}
```

## Advanced: Multi-Machine Deployment

### Automated deployment script

```bash
#!/usr/bin/env bash
# deploy-fleet.sh - Deploy operation-dbus to multiple machines

MACHINES=(
  "192.168.1.10:opdbus-node-01:/dev/sda"
  "192.168.1.11:opdbus-node-02:/dev/nvme0n1"
  "192.168.1.12:opdbus-node-03:/dev/sda"
)

for machine in "${MACHINES[@]}"; do
  IFS=: read -r ip hostname disk <<< "$machine"

  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "Deploying to $hostname ($ip)"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # Boot via IPMI/iLO (if available)
  # ipmitool -H "$ip-mgmt" chassis bootdev pxe
  # ipmitool -H "$ip-mgmt" power reset

  # Wait for installer to boot
  echo "Waiting for installer to boot..."
  until ssh -o ConnectTimeout=5 root@"$ip" true 2>/dev/null; do
    sleep 10
  done

  # Run installation
  echo "Running installation..."
  ssh root@"$ip" "/etc/opdbus-install.sh $disk $hostname"

  echo "✓ $hostname deployed"
  echo ""
done

echo "All machines deployed!"
```

### State file distribution

```bash
# Create state files for each machine
mkdir -p states

cat > states/opdbus-node-01.json <<'EOF'
{
  "version": "1.0",
  "plugins": {
    "lxc": {
      "containers": [
        {"id": "100", "hostname": "web-01", "template": "debian-13"}
      ]
    }
  }
}
EOF

cat > states/opdbus-node-02.json <<'EOF'
{
  "version": "1.0",
  "plugins": {
    "lxc": {
      "containers": [
        {"id": "101", "hostname": "db-01", "template": "debian-13"}
      ]
    }
  }
}
EOF

# Host on HTTP server
scp -r states/ user@webserver:/var/www/netboot/

# Machines fetch their state on boot via systemd service
# (configured in installation)
```

## Troubleshooting

### Installer doesn't boot

```bash
# Check HTTP server accessibility
curl -I http://your-server/netboot/installer/bzImage

# Check netboot.xyz menu
# Verify URLs in custom.ipxe are correct

# Check boot logs on target machine (if accessible)
# Look for TFTP/HTTP errors
```

### Installation fails at partitioning

```bash
# Check disk is correct
lsblk

# Verify disk is writable
hdparm -r0 /dev/sda

# Check for existing partitions
sgdisk -p /dev/sda

# Wipe all partition tables
sgdisk --zap-all /dev/sda
```

### BTRFS subvolume creation fails

```bash
# Check BTRFS is properly formatted
btrfs filesystem show

# Verify mount point
mount | grep /mnt

# Check kernel module
lsmod | grep btrfs

# Try manual creation
mount /dev/sda2 /mnt
btrfs subvolume create /mnt/@test
btrfs subvolume list /mnt
```

### NixOS installation fails

```bash
# Check network connectivity
ping -c 4 1.1.1.1

# Check disk space
df -h /mnt

# Check Nix store
ls /nix/store

# View detailed error
nixos-install --show-trace
```

### operation-dbus doesn't start after install

```bash
# SSH into new system
ssh root@<new-ip>

# Check service status
systemctl status operation-dbus

# View logs
journalctl -u operation-dbus -xe

# Common issues:
# 1. Missing state file
ls -la /etc/operation-dbus/state.json

# 2. BTRFS subvolumes not mounted
df -h | grep op-dbus

# 3. D-Bus not running
systemctl status dbus

# 4. Permissions issue
ls -la /var/lib/op-dbus
```

## Performance Optimization

### SSD/NVMe Tuning

```nix
{
  # In /etc/nixos/configuration.nix

  fileSystems."/var/lib/op-dbus/cache".options = [
    "subvol=@cache"
    "compress=zstd:3"
    "noatime"
    "ssd"  # Enable SSD optimizations
    "discard=async"  # Async TRIM
  ];

  # Enable fstrim timer
  services.fstrim.enable = true;
  services.fstrim.interval = "weekly";
}
```

### NUMA Optimization Validation

```bash
# After installation, validate NUMA setup
ssh root@<new-ip>

# Check topology
numactl --hardware

# Verify CPU pinning
systemctl show operation-dbus | grep CPUAffinity

# Monitor NUMA stats
watch -n 1 'numastat -p $(pgrep op-dbus)'

# Should show most allocations on node 0 (or configured node)
```

### ML Cache Warmup

```bash
# After first boot, warm up ML cache
op-dbus cache warmup

# Monitor cache build
journalctl -u operation-dbus -f | grep "embedding"

# Check cache size
du -sh /var/lib/op-dbus/cache/embeddings

# Verify compression ratio
compsize /var/lib/op-dbus/cache
```

## Summary: Complete Workflow

```bash
# ═══════════════════════════════════════════════════════════════
# Complete netboot → disk installation workflow
# ═══════════════════════════════════════════════════════════════

# On build machine:
# ────────────────────────────────────────────────────────────────
cd operation-dbus
./nixos/netboot/build-installer.sh
scp /tmp/opdbus-installer/* user@webserver:/var/www/netboot/installer/

# Add to netboot.xyz:
# ────────────────────────────────────────────────────────────────
# :opdbus-installer
# kernel http://your-server/netboot/installer/bzImage init=/nix/store/.../init
# initrd http://your-server/netboot/installer/initrd
# boot

# On target machine:
# ────────────────────────────────────────────────────────────────
# 1. PXE boot → netboot.xyz → Custom → opdbus-installer
# 2. Wait for boot (1-2 min)
# 3. SSH: ssh root@<ip>  (password: nixos)
# 4. Install: sudo /etc/opdbus-install.sh /dev/sda opdbus-node-01
# 5. Reboot: sudo reboot
# 6. SSH into new system: ssh root@<new-ip>
# 7. Verify: systemctl status operation-dbus

# Done! System is installed with:
# ────────────────────────────────────────────────────────────────
# ✓ NixOS with operation-dbus
# ✓ BTRFS subvolumes for caching
# ✓ ML vectorization enabled
# ✓ NUMA optimization (if multi-socket)
# ✓ Persistent state storage
# ✓ Automated snapshot rotation
```

---

**Next Steps**:
1. Build installer: `./nixos/netboot/build-installer.sh`
2. Host on HTTP server
3. Add to netboot.xyz menu
4. Boot and install!

The combination of netboot.xyz convenience + local disk persistence gives you the best possible setup for operation-dbus.
