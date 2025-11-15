# netboot.xyz for operation-dbus

This directory contains netboot.xyz boot files for installation on ESP.

## Files

- `netboot.xyz.conf` - systemd-boot entry for netboot.xyz
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
