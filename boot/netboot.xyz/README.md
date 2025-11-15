# netboot.xyz for operation-dbus

This directory contains netboot.xyz boot files for installation on ESP.

## Files

- `netboot.xyz.conf` - systemd-boot entry for netboot.xyz
- `netboot.xyz.efi` - EFI binary (~1MB, downloaded automatically or add to repo)

## Automatic Installation

The complete VPS installer (`tools/install-proxmox-vps.sh`) automatically:
1. Checks if `netboot.xyz.efi` exists in this repo directory
2. If found: Copies from repo
3. If not found: Downloads from https://boot.netboot.xyz/ipxe/netboot.xyz.efi

No manual intervention required!

## Manual Installation (Optional)

```bash
# Using the standalone installer
sudo ./tools/install-netboot-xyz.sh /boot/efi

# Or manually
wget -O /boot/efi/netboot.xyz/netboot.xyz.efi \
    https://boot.netboot.xyz/ipxe/netboot.xyz.efi
sudo cp boot/netboot.xyz/netboot.xyz.conf /boot/efi/loader/entries/
```

## Adding EFI Binary to Repo (Optional)

For offline installations, download and commit the .efi binary:
```bash
cd boot/netboot.xyz
wget https://boot.netboot.xyz/ipxe/netboot.xyz.efi
git add netboot.xyz.efi
git commit -m "Add netboot.xyz EFI binary for offline installs"
- `netboot.xyz.efi` - Download from https://boot.netboot.xyz/ipxe/netboot.xyz.efi

## Installation

```bash
# Copy to ESP
sudo cp boot/netboot.xyz/netboot.xyz.conf /boot/efi/loader/entries/
sudo cp boot/netboot.xyz/netboot.xyz.efi /boot/efi/netboot.xyz/

# Download latest netboot.xyz.efi
wget -O /boot/efi/netboot.xyz/netboot.xyz.efi \
    https://boot.netboot.xyz/ipxe/netboot.xyz.efi
```

## Integration

The Proxmox installer script (`tools/install-proxmox-to-esp.sh`) will:
- Detect existing netboot.xyz
- Preserve netboot.xyz entry in loader.conf
- Add both netboot.xyz and Proxmox entries
