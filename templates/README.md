# LXC Templates for op-dbus

## Netmaker-Ready Template

The `debian-13-netmaker_custom.tar.zst` template includes:
- ✅ Debian 13 (Trixie) base
- ✅ netclient pre-installed
- ✅ wireguard support
- ✅ Ready for automatic mesh networking

## Creating the Template

### Option 1: Manual Preparation (Recommended)

1. **Create a container manually in Proxmox:**
   - ID: 9999 (temporary)
   - Template: debian-13-standard
   - Storage: local-btrfs
   - Memory: 512MB

2. **Install netclient inside the container:**
   ```bash
   pct start 9999
   
   # Inside container (via console or pct exec):
   apt-get update
   apt-get install -y curl gnupg ca-certificates wireguard jq
   
   # Add netmaker repository (modern method for Debian 13)
   curl -fsSL https://apt.netmaker.org/gpg.key | gpg --dearmor -o /usr/share/keyrings/netmaker-archive-keyring.gpg
   echo "deb [signed-by=/usr/share/keyrings/netmaker-archive-keyring.gpg] https://apt.netmaker.org/debian stable main" | tee /etc/apt/sources.list.d/netmaker.list
   
   apt-get update
   apt-get install -y netclient
   
   # Verify
   netclient --version
   ```

3. **Export as template:**
   ```bash
   sudo ./export-template.sh 9999
   ```

### Option 2: Automated Script

```bash
sudo ./create-netmaker-template.sh
```

Note: May fail on Debian 13 due to deprecated `apt-key` command. Use Option 1 instead.

## Using the Template

The template is automatically used by op-dbus when creating containers:

```json
{
  "lxc": {
    "containers": [{
      "id": "100",
      "veth": "vi100",
      "bridge": "mesh",
      "properties": {
        "network_type": "netmaker",
        "template": "local-btrfs:vztmpl/debian-13-netmaker_custom.tar.zst"
      }
    }]
  }
}
```

## Template Location

- **Path**: `/var/lib/pve/local-btrfs/template/cache/debian-13-netmaker_custom.tar.zst`
- **Storage**: `local-btrfs`
- **Size**: ~200-300MB (compressed)

## Downloading Pre-built Template

If available, download from releases:

```bash
cd /var/lib/pve/local-btrfs/template/cache/
curl -L https://github.com/repr0bated/operation-dbus/releases/download/v0.1.0/debian-13-netmaker_custom.tar.zst -o debian-13-netmaker_custom.tar.zst
```

*(Pre-built template not yet available - use manual preparation method)*

