# Boot Configuration Repository

This directory contains all boot-related files for the operation-dbus VPS Proxmox installation system.

## Directory Structure

```
boot/
├── grub/                         # GRUB configuration
│   └── grub.cfg                  # GRUB boot menu
├── netboot.xyz/                  # netboot.xyz boot manager
│   ├── netboot.xyz.conf          # Boot entry reference
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
# - Install GRUB
# - Copy ISO and netboot.xyz to ESP
# - Configure GRUB to boot ISO via loopback
# - Reboot into Proxmox installer

sudo ./tools/install-proxmox-vps.sh /dev/sda
```

## What Gets Installed

The installer script (`tools/install-proxmox-vps.sh`) installs these to the ESP:

1. **GRUB bootloader**: Installed to ESP with removable flag
2. **GRUB config**: `boot/grub/grub.cfg` → `/boot/efi/grub/grub.cfg`
3. **Proxmox ISO**: Copied to `/boot/efi/iso/proxmox-installer.iso`
4. **netboot.xyz**: Downloaded or copied to `/boot/efi/netboot.xyz/`

## Repository Philosophy

- **Version control boot configuration**: GRUB config is in git
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

## GRUB Loopback Booting

This system uses **GRUB** with ISO loopback mounting:
- **No extraction needed**: GRUB boots ISOs directly using loopback
- **Space efficient**: Stores 1.2GB ISO instead of extracting to ESP
- **Standard bootloader**: Works on all UEFI and BIOS systems
- **Chainloading support**: Easily chainloads netboot.xyz

### How It Works

```grub
menuentry "Proxmox VE 9 Installer (PackageKit)" {
    set isofile="/iso/proxmox-installer.iso"
    loopback loop $isofile
    linux (loop)/boot/linux26 boot=live findiso=$isofile
    initrd (loop)/boot/initrd.img
}
```

GRUB mounts the ISO internally and boots the kernel/initrd from it.
