# Boot Configuration Repository

This directory contains all boot-related files for the operation-dbus VPS Proxmox installation system.

## Directory Structure

```
boot/
├── efi/                          # EFI boot files
│   └── loader/                   # systemd-boot configuration
│       ├── loader.conf           # Main loader config
│       └── entries/              # Boot entries
│           ├── proxmox-installer.conf  # Proxmox installer entry
│           └── netboot.xyz.conf        # (in ../netboot.xyz/)
├── netboot.xyz/                  # netboot.xyz boot manager
│   ├── netboot.xyz.conf          # Boot entry
│   ├── netboot.xyz.efi           # EFI binary (downloaded)
│   └── README.md                 # Installation docs
└── proxmox-iso/                  # Converted Proxmox ISOs
    ├── proxmox-ve_9.0-1-packagekit.iso  # Converted ISO (gitignored)
    └── README.md                 # ISO creation docs
```

## Usage Workflow

### 1. Create Converted ISO (one-time)

```bash
# Download original Proxmox VE 9
./tools/proxmox-extractor/download-pve9.sh

# Convert to PackageKit backend
sudo ./tools/patch-proxmox-iso.sh \
    proxmox-ve_9.0-1.iso \
    boot/proxmox-iso/proxmox-ve_9.0-1-packagekit.iso
```

### 2. Run Complete VPS Installer

```bash
# This will:
# - Partition drive (/dev/sda)
# - Create 2GB ESP + BTRFS root
# - Install systemd-boot
# - Copy ISO, netboot.xyz, and boot configs from repo
# - Reboot into Proxmox installer

sudo ./tools/install-proxmox-vps.sh /dev/sda
```

## What Gets Installed

The installer script (`tools/install-proxmox-vps.sh`) copies these files to the actual ESP:

1. **EFI loader config**: `boot/efi/loader/loader.conf` → `/boot/efi/loader/loader.conf`
2. **Boot entries**: `boot/efi/loader/entries/*.conf` → `/boot/efi/loader/entries/`
3. **netboot.xyz**: Downloads or copies from repo to `/boot/efi/netboot.xyz/`
4. **Proxmox ISO**: Extracts to `/boot/efi/proxmox-installer/`

## Repository Philosophy

- **Version control boot configuration**: All boot entries and loader configs are in git
- **Reproducible installations**: Anyone can clone and run the installer
- **Preserve netboot.xyz**: Always restore network boot manager
- **PackageKit-only**: Converted ISO ensures no dpkg/apt usage

## Git LFS for ISO Storage (Optional)

The converted ISO (~1.2GB) is gitignored by default. To track it with Git LFS:

```bash
git lfs install
git lfs track "boot/proxmox-iso/*.iso"
git add .gitattributes
git add -f boot/proxmox-iso/proxmox-ve_9.0-1-packagekit.iso
git commit -m "Add converted Proxmox ISO via Git LFS"
```

## systemd-boot vs GRUB

This system uses **systemd-boot** (not GRUB) for VPS compatibility:
- Simpler configuration
- Native UEFI support
- Faster boot times
- Easier to script and automate
