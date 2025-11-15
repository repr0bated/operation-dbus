# Proxmox PackageKit ISO Storage

This directory stores the pre-converted Proxmox VE 9 ISO with PackageKit backend.

## File Location

Place the converted ISO here:
```
boot/proxmox-iso/proxmox-ve_9.0-1-packagekit.iso
```

## Creating the ISO

If not present, create it with:
```bash
# Download original ISO
./tools/proxmox-extractor/download-pve9.sh

# Patch to use PackageKit
sudo ./tools/patch-proxmox-iso.sh \
    proxmox-ve_9.0-1.iso \
    boot/proxmox-iso/proxmox-ve_9.0-1-packagekit.iso
```

## Using the ISO

The complete VPS installer will use this ISO:
```bash
sudo ./tools/install-proxmox-vps.sh /dev/sda \
    boot/proxmox-iso/proxmox-ve_9.0-1-packagekit.iso
```

## Git LFS (Optional)

If storing large ISOs in git, consider using Git LFS:
```bash
git lfs track "*.iso"
git add .gitattributes
```

## File Size

The converted ISO is approximately 1.2GB. If this is too large for your
repository, store it externally and download before installation.
