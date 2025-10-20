# Automatic Container Netmaker Setup

Complete zero-configuration netmaker mesh networking for LXC containers.

## Overview

When you create a container with `network_type: "netmaker"`, it will **automatically**:
1. Be created with netclient pre-installed (from custom template)
2. Start with the mesh bridge attached
3. Join netmaker via LXC hook on first boot
4. No manual configuration required!

## Quick Start

### 1. Create Netmaker-Ready Template (One-Time)

```bash
# On Proxmox host
sudo ./create-netmaker-template.sh
```

This creates `debian-11-netmaker_custom.tar.zst` with:
- netclient pre-installed
- wireguard support
- Ready to join netmaker

**Time**: ~5 minutes (downloads, installs, creates template)

### 2. Install op-dbus with Netmaker

```bash
# Add your netmaker enrollment token
echo "NETMAKER_TOKEN=your-token-here" | sudo tee /etc/op-dbus/netmaker.env

# Run install script
sudo ./install.sh
```

The install script will:
- Join HOST to netmaker
- Add host's netmaker interfaces to mesh bridge
- Install LXC hook for automatic container join
- Configure everything needed

### 3. Create Containers - Zero Config!

Add to `/etc/op-dbus/state.json`:

```json
{
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": "100",
          "veth": "vi100",
          "bridge": "vmbr0",
          "properties": {
            "network_type": "netmaker"
          }
        }
      ]
    }
  }
}
```

Apply state:

```bash
op-dbus apply /etc/op-dbus/state.json
```

**That's it!** The container will:
1. Be created from netmaker template
2. Start with mesh bridge
3. Automatically join netmaker on boot (via LXC hook)
4. Get mesh networking without any manual steps

## How It Works

### Architecture

```
┌─────────────────────────────────────────────────┐
│ Proxmox Host                                    │
│                                                 │
│  netclient (host joined to netmaker)           │
│      ↓                                          │
│  nm-mynetwork (wireguard interface)            │
│      ↓                                          │
│  mesh (OVS bridge)                              │
│      ├─ nm-mynetwork (wireguard)               │
│      ├─ vi100 (container 100's veth)           │
│      └─ vi102 (container 102's veth)           │
│                                                 │
│  ┌──────────────────────────────────────────┐  │
│  │ Container 100 (netmaker-enabled)        │  │
│  │                                          │  │
│  │  netclient (auto-joins via LXC hook)    │  │
│  │      ↓                                   │  │
│  │  nm-mynetwork (container's wireguard)   │  │
│  │      ↓                                   │  │
│  │  mesh networking active!                │  │
│  └──────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### Component Breakdown

**1. Custom Template**
- Created by `create-netmaker-template.sh`
- Debian 11 with netclient pre-installed
- Located: `/var/lib/vz/template/cache/debian-11-netmaker_custom.tar.zst`

**2. LXC Plugin**
- Uses custom template when `network_type: "netmaker"`
- Attaches container to mesh bridge
- Sets veth name to `vi{VMID}`

**3. LXC Hook** (installed by install.sh)
- Location: `/usr/share/lxc/hooks/netmaker-join`
- Triggers on container start (`lxc.hook.start-host`)
- Checks if container uses mesh bridge
- Automatically runs `netclient join` inside container
- Logs to `/var/log/lxc-netmaker-hook.log`

**4. Global LXC Config**
- Location: `/usr/share/lxc/config/common.conf.d/netmaker.conf`
- Enables hook for all containers
- Hook only acts on containers using mesh bridge

## Container Creation Flow

```
User: op-dbus apply state.json
    ↓
LXC Plugin: pct create 100 ... --net0 bridge=mesh
    ↓
Container Created
    ↓
pct start 100
    ↓
LXC Hook Triggered: /usr/share/lxc/hooks/netmaker-join
    ↓
Hook: Check if container uses mesh bridge → YES
    ↓
Hook: pct exec 100 -- netclient join -t $NETMAKER_TOKEN
    ↓
Container Joined Netmaker
    ↓
Container Has Mesh Networking (zero user config!)
```

## Verifying It Works

### Check Hook Installation

```bash
# Hook script exists
ls -la /usr/share/lxc/hooks/netmaker-join

# Hook config exists
ls -la /usr/share/lxc/config/common.conf.d/netmaker.conf

# View hook logs
sudo tail -f /var/log/lxc-netmaker-hook.log
```

### Check Container Netmaker Status

```bash
# Inside container
pct exec 100 -- netclient list

# Should show:
# Connected networks:
#   - mynetwork (your network name)
```

### Check Mesh Networking

```bash
# From host, test connectivity to another mesh node
ping <other-node-mesh-ip>

# From container, test connectivity
pct exec 100 -- ping <other-node-mesh-ip>
```

## Troubleshooting

### Container Not Joining Netmaker

**Check hook logs:**
```bash
sudo tail -20 /var/log/lxc-netmaker-hook.log
```

**Common issues:**

1. **"netclient not found in container"**
   - Template doesn't have netclient
   - Rebuild template: `sudo ./create-netmaker-template.sh`

2. **"NETMAKER_TOKEN not set"**
   - Add token: `echo "NETMAKER_TOKEN=..." | sudo tee /etc/op-dbus/netmaker.env`

3. **"Container not using mesh bridge"**
   - Check state.json: must have `"network_type": "netmaker"`
   - Container config should show `bridge=mesh`

### Hook Not Triggering

```bash
# Check hook is executable
ls -l /usr/share/lxc/hooks/netmaker-join

# Check LXC config
cat /usr/share/lxc/config/common.conf.d/netmaker.conf

# Manually trigger hook (for testing)
sudo /usr/share/lxc/hooks/netmaker-join
```

### Template Issues

**Re-create template:**
```bash
sudo ./create-netmaker-template.sh
```

**Use standard template temporarily:**
Edit `src/state/plugins/lxc.rs` line 175:
```rust
.unwrap_or("local:vztmpl/debian-11-standard_11.7-1_amd64.tar.zst");
```

## Advanced Configuration

### Custom Template Per Container

Override template in state.json:

```json
{
  "id": "100",
  "properties": {
    "network_type": "netmaker",
    "template": "local:vztmpl/my-custom-template.tar.zst"
  }
}
```

### Traditional Bridge Mode

For containers that shouldn't use netmaker:

```json
{
  "id": "101",
  "properties": {
    "network_type": "bridge"
  }
}
```

Container will use `vmbr0` instead of `mesh`, no netmaker join.

### Manual Container Join

If hook fails, manually join:

```bash
pct exec 100 -- netclient join -t $NETMAKER_TOKEN
```

## Files Created/Modified

| File | Purpose |
|------|---------|
| `/usr/share/lxc/hooks/netmaker-join` | LXC hook script |
| `/usr/share/lxc/config/common.conf.d/netmaker.conf` | Enable hook globally |
| `/etc/op-dbus/netmaker.env` | Host netmaker token |
| `/var/log/lxc-netmaker-hook.log` | Hook execution logs |
| `/var/lib/vz/template/cache/debian-11-netmaker_custom.tar.zst` | Custom template |

## Uninstallation

Remove netmaker integration:

```bash
sudo rm /usr/share/lxc/hooks/netmaker-join
sudo rm /usr/share/lxc/config/common.conf.d/netmaker.conf
sudo rm /etc/op-dbus/netmaker.env
sudo rm /var/log/lxc-netmaker-hook.log
```

Containers will still work, just without automatic netmaker join.

## Benefits

✅ **Zero Configuration**: No manual netmaker setup in containers
✅ **Automatic**: Join happens on first boot
✅ **Consistent**: All netmaker containers use same flow
✅ **Auditable**: Hook logs every join attempt
✅ **Flexible**: Easy to disable per-container
✅ **Scalable**: Works for 1 or 1000 containers

## See Also

- [CONTAINER-SETUP.md](CONTAINER-SETUP.md) - General container setup
- [DUAL-BRIDGE-ARCHITECTURE.md](DUAL-BRIDGE-ARCHITECTURE.md) - Network architecture
- [DEPLOYMENT.md](DEPLOYMENT.md) - Production deployment
