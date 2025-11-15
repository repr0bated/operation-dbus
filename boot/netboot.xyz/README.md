# netboot.xyz for operation-dbus

This directory contains netboot.xyz boot files for installation on ESP.

## Files

- `netboot.xyz.conf` - Boot entry reference (for documentation)
- `netboot.xyz.efi` - EFI binary (~1MB, downloaded automatically or add to repo)

## Automatic Installation

The complete VPS installer (`tools/install-proxmox-vps.sh`) automatically:
1. Checks if `netboot.xyz.efi` exists in this repo directory
2. If found: Copies from repo
3. If not found: Downloads from https://boot.netboot.xyz/ipxe/netboot.xyz.efi
4. Installs to `/boot/efi/netboot.xyz/`
5. Adds GRUB menu entry for chainloading

No manual intervention required!

## Manual Installation (Optional)

```bash
# Using the standalone installer
sudo ./tools/install-netboot-xyz.sh /boot/efi

# Or manually
mkdir -p /boot/efi/netboot.xyz
wget -O /boot/efi/netboot.xyz/netboot.xyz.efi \
    https://boot.netboot.xyz/ipxe/netboot.xyz.efi
```

## Adding EFI Binary to Repo (Optional)

For offline installations, download and commit the .efi binary:
```bash
cd boot/netboot.xyz
wget https://boot.netboot.xyz/ipxe/netboot.xyz.efi
git add netboot.xyz.efi
git commit -m "Add netboot.xyz EFI binary for offline installs"
```

## Integration with GRUB

The Proxmox VPS installer (`tools/install-proxmox-vps.sh`) creates a GRUB entry:

```grub
menuentry "netboot.xyz" {
    chainloader /netboot.xyz/netboot.xyz.efi
}
```

This allows booting into the netboot.xyz network boot manager from the GRUB menu.
