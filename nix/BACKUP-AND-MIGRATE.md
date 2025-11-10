# Backup and Migration Guide - oo1424oo to NixOS

**CRITICAL:** Before wiping the VPS, backup your working oo1424oo system to use as reference for NixOS conversion.

## Why Backup First

Your oo1424oo system has:
- ✅ Working containers (gateway, warp, xray)
- ✅ Proven network configuration
- ✅ Tested OVS bridge setup
- ✅ Container templates and configs
- ✅ State that's been validated

This is your **golden reference** for converting to NixOS declarative config.

## Architecture Overview

```
oo1424oo (Proxmox/Traditional)     →     VPS (NixOS Declarative)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Install scripts + manual config    →     configuration.nix
/etc/op-dbus/state.json            →     services.op-dbus.stateConfig {}
LXC containers (pct create)        →     lxc.containers = []
Manual OVS commands                →     net.interfaces = []
systemctl enable                   →     services.op-dbus.enable = true
```

## Step 1: Backup oo1424oo System

### Option A: BTRFS Send/Receive (Recommended)

If oo1424oo uses BTRFS (Proxmox default):

```bash
# On oo1424oo
ssh root@oo1424oo

# Check filesystem type
df -T /
# If BTRFS, proceed

# Create snapshot of root
btrfs subvolume snapshot / /root-snapshot-$(date +%Y%m%d)

# Find container subvolumes
btrfs subvolume list /var/lib/lxc/
# Or
ls /var/lib/lxc/

# Snapshot each container
for container in /var/lib/lxc/*/; do
  name=$(basename "$container")
  btrfs subvolume snapshot "$container/rootfs" \
    "/var/lib/lxc/${name}/rootfs-snapshot-$(date +%Y%m%d)"
done

# Send to backup location (external drive or another server)
# Root filesystem
btrfs send /root-snapshot-20251109 | \
  ssh backup-server "btrfs receive /backup/oo1424oo/"

# Each container
for container in 100 101 102; do
  btrfs send /var/lib/lxc/${container}/rootfs-snapshot-20251109 | \
    ssh backup-server "btrfs receive /backup/oo1424oo/containers/"
done
```

### Option B: Tar Backup (Alternative)

If not using BTRFS:

```bash
# On oo1424oo

# Backup configurations
tar czf oo1424oo-configs-$(date +%Y%m%d).tar.gz \
  /etc/op-dbus/ \
  /etc/nixos/ \
  /etc/systemd/system/op-dbus.service \
  /etc/network/interfaces \
  /etc/netplan/ 2>/dev/null || true

# Backup OVS database
tar czf oo1424oo-ovs-$(date +%Y%m%d).tar.gz \
  /etc/openvswitch/ \
  /var/lib/openvswitch/

# Backup each container
for container in 100 101 102; do
  tar czf oo1424oo-container-${container}-$(date +%Y%m%d).tar.gz \
    -C /var/lib/lxc/${container} .
done

# Transfer to safe location
scp oo1424oo-*.tar.gz backup-server:/backup/
```

### Option C: Full System Backup

```bash
# Using rsync
rsync -aAXv --exclude={"/dev/*","/proc/*","/sys/*","/tmp/*","/run/*"} \
  / backup-server:/backup/oo1424oo-full/

# Or using Proxmox backup
vzdump --mode snapshot --compress zstd
```

## Step 2: Extract Reference Configuration

Before converting to NixOS, document your working setup:

```bash
# On oo1424oo

# 1. Query op-dbus state
op-dbus query > /tmp/oo1424oo-state.json

# 2. Get OVS configuration
ovs-vsctl show > /tmp/ovs-config.txt
ovs-vsctl list-br > /tmp/bridges.txt
for br in $(ovs-vsctl list-br); do
  echo "=== Bridge: $br ===" >> /tmp/bridge-details.txt
  ovs-vsctl list-ports $br >> /tmp/bridge-details.txt
  ip addr show $br >> /tmp/bridge-details.txt
done

# 3. Get container configuration
lxc-ls -f > /tmp/containers-list.txt
for container in 100 101 102; do
  pct config $container > /tmp/container-${container}-config.txt 2>/dev/null || \
  lxc-info -n $container > /tmp/container-${container}-info.txt
done

# 4. Get network configuration
ip addr > /tmp/ip-addr.txt
ip route > /tmp/ip-route.txt
cat /etc/network/interfaces > /tmp/interfaces.txt 2>/dev/null || true

# 5. Get systemd services
systemctl status openvswitch > /tmp/ovs-service.txt
systemctl status op-dbus > /tmp/op-dbus-service.txt

# 6. Package all reference data
tar czf oo1424oo-reference-$(date +%Y%m%d).tar.gz /tmp/*.txt /tmp/*.json

# Transfer
scp oo1424oo-reference-*.tar.gz your-machine:/home/user/
```

## Step 3: Convert to NixOS Configuration

Using your reference data, create NixOS config:

```bash
# On your local machine
tar xzf oo1424oo-reference-20251109.tar.gz

# View current state
cat tmp/oo1424oo-state.json

# Example output:
{
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "ovsbr0",
          "type": "ovs-bridge",
          "ports": ["ens1"],
          "ipv4": {
            "address": [{"ip": "192.168.1.10", "prefix": 24}],
            "gateway": "192.168.1.1"
          }
        },
        {
          "name": "mesh",
          "type": "ovs-bridge",
          "ports": ["vi100", "vi101", "vi102"]
        }
      ]
    },
    "lxc": {
      "containers": [
        {"id": "100", "name": "gateway", "veth": "vi100", "bridge": "mesh"},
        {"id": "101", "name": "warp", "veth": "vi101", "bridge": "mesh"},
        {"id": "102", "name": "xray-client", "veth": "vi102", "bridge": "mesh"}
      ]
    }
  }
}
```

### Create NixOS configuration.nix from this:

```nix
# /etc/nixos/configuration.nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    /path/to/operation-dbus/nix/module.nix
  ];

  services.op-dbus = {
    enable = true;
    mode = "full";

    # Converted from oo1424oo state.json
    stateConfig = {
      net = {
        interfaces = [
          {
            name = "ovsbr0";
            type = "ovs-bridge";
            ports = [ "ens1" ];  # From oo1424oo
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "192.168.1.10"; prefix = 24; }  # From oo1424oo
              ];
              gateway = "192.168.1.1";  # From oo1424oo
            };
          }
          {
            name = "mesh";
            type = "ovs-bridge";
            ports = [];  # Will be populated by containers
            ipv4 = {
              enabled = true;
              dhcp = false;
              address = [
                { ip = "10.0.0.1"; prefix = 24; }  # From oo1424oo
              ];
            };
          }
        ];
      };

      # Containers from oo1424oo
      lxc = {
        containers = [
          {
            id = "100";
            name = "gateway";
            veth = "vi100";
            bridge = "mesh";
            running = true;
          }
          {
            id = "101";
            name = "warp";
            veth = "vi101";
            bridge = "mesh";
            running = true;
          }
          {
            id = "102";
            name = "xray-client";
            veth = "vi102";
            bridge = "mesh";
            running = true;
          }
        ];
      };

      systemd = {
        units = {
          "openvswitch.service" = {
            enabled = true;
            active_state = "active";
          };
        };
      };
    };

    enableBlockchain = true;
    enableCache = true;
  };

  # System packages (based on oo1424oo)
  environment.systemPackages = with pkgs; [
    openvswitch
    lxc
    vim
    git
    curl
    htop
  ];

  # Enable services
  virtualisation.vswitch.enable = true;
  virtualisation.lxc.enable = true;
  systemd.services.openvswitch.enable = true;

  networking = {
    hostName = "nixos-vps";
    useDHCP = false;
    firewall.allowedTCPPorts = [ 22 9573 9574 ];
  };

  system.stateVersion = "24.05";
}
```

## Step 4: Test Conversion (Dry Run)

Before wiping VPS, test NixOS config in a VM or test server:

```bash
# Build config without activating
nix-build '<nixpkgs/nixos>' -A config.system.build.toplevel \
  -I nixos-config=/path/to/configuration.nix

# Or in a VM
nixos-rebuild build-vm -I nixos-config=/path/to/configuration.nix
./result/bin/run-nixos-vm

# Inside VM, verify matches oo1424oo:
ovs-vsctl show      # Compare to oo1424oo ovs-config.txt
lxc-ls -f           # Compare to oo1424oo containers-list.txt
ip addr             # Compare to oo1424oo ip-addr.txt
op-dbus query       # Compare to oo1424oo-state.json
```

## Step 5: Deploy to VPS (After Validation)

Only after confirming NixOS config matches oo1424oo:

```bash
# 1. Backup VPS if it has anything important
ssh root@80.209.240.244 "tar czf /root/vps-backup.tar.gz /etc /var/lib"

# 2. Install NixOS on VPS
# (Follow nix/VPS-DEPLOYMENT.md)

# 3. Apply your tested configuration
sudo nixos-rebuild switch

# 4. Verify matches oo1424oo
systemctl status op-dbus
ovs-vsctl show
lxc-ls -f
op-dbus query
```

## Step 6: Restore Container Contents (If Needed)

If you backed up container filesystems with BTRFS send:

```bash
# On new NixOS VPS

# Receive container backup
ssh backup-server "btrfs send /backup/oo1424oo/containers/gateway" | \
  btrfs receive /var/lib/lxc/100/

# Or with tar
scp backup-server:/backup/oo1424oo-container-100-*.tar.gz /tmp/
tar xzf /tmp/oo1424oo-container-100-*.tar.gz -C /var/lib/lxc/100/

# Restart container
lxc-start -n 100
```

## Validation Checklist

Compare NixOS VPS against oo1424oo reference:

### Network Configuration
```bash
# oo1424oo
ssh oo1424oo "ip addr show ovsbr0"

# NixOS VPS
ssh vps "ip addr show ovsbr0"

# Should match: IP, netmask, gateway
```

### OVS Bridges
```bash
# oo1424oo
ssh oo1424oo "ovs-vsctl show"

# NixOS VPS
ssh vps "ovs-vsctl show"

# Should match: bridges, ports, interfaces
```

### Containers
```bash
# oo1424oo
ssh oo1424oo "lxc-ls -f"

# NixOS VPS
ssh vps "lxc-ls -f"

# Should match: count, names, status
```

### op-dbus State
```bash
# oo1424oo
ssh oo1424oo "op-dbus query" > oo1424oo-state.json

# NixOS VPS
ssh vps "op-dbus query" > vps-state.json

# Compare
diff oo1424oo-state.json vps-state.json
```

## Rollback Strategy

If NixOS deployment fails:

### Option 1: Restore VPS from Backup
```bash
# If you backed up the VPS before wiping
ssh root@80.209.240.244
tar xzf /root/vps-backup.tar.gz -C /
reboot
```

### Option 2: Keep oo1424oo Untouched
```bash
# Just revert to using oo1424oo
# The backup ensures you lose nothing
```

### Option 3: NixOS Rollback
```bash
# On NixOS VPS
sudo nixos-rebuild --rollback

# Or boot into previous generation
# At boot: Select previous generation from menu
```

## Migration Phases

### Phase 1: Backup and Reference (Safe)
- ✅ Backup oo1424oo completely
- ✅ Extract all configuration
- ✅ Keep oo1424oo running
- ❌ No changes yet

### Phase 2: Test Conversion (Safe)
- ✅ Create NixOS config from oo1424oo data
- ✅ Test in VM or test server
- ✅ Validate matches oo1424oo
- ❌ VPS still untouched

### Phase 3: Deploy to VPS (Destructive)
- ⚠️ Wipe VPS and install NixOS
- ⚠️ Apply converted configuration
- ✅ Can rollback to oo1424oo if needed

### Phase 4: Validation (Verify)
- ✅ Compare VPS against oo1424oo reference
- ✅ Test all functionality
- ✅ Keep oo1424oo as backup for a while

### Phase 5: Production (Commit)
- ✅ VPS running successfully
- ✅ Matches oo1424oo functionality
- ✅ Can decommission oo1424oo (keep backup)

## Container Contents Migration

The NixOS module creates container structure, but container **contents** must be migrated separately:

```bash
# What NixOS creates automatically:
/var/lib/lxc/100/              # Container directory
/var/lib/lxc/100/config        # LXC config
vi100                          # veth interface

# What you need to migrate from oo1424oo:
/var/lib/lxc/100/rootfs/       # Container filesystem
  /var/lib/lxc/100/rootfs/etc  # Container configs
  /var/lib/lxc/100/rootfs/usr  # Container binaries

# Migration options:
1. BTRFS send/receive - Exact copy
2. Tar archive - Portable
3. Rsync - Incremental
4. Rebuild from scratch - Clean slate
```

## Best Practice: Parallel Running

Safest approach:

1. **Keep oo1424oo running** with working setup
2. **Deploy NixOS to VPS** as new system
3. **Test VPS thoroughly** against oo1424oo
4. **Run both in parallel** until confident
5. **Switch traffic** to VPS when ready
6. **Keep oo1424oo backup** for safety

This way you never lose your working system.

## Summary

**Before wiping VPS:**
1. ✅ Backup oo1424oo completely (BTRFS send or tar)
2. ✅ Extract all configuration data
3. ✅ Convert to NixOS configuration.nix
4. ✅ Test conversion in VM
5. ✅ Validate against oo1424oo reference

**Only then:**
6. Deploy to VPS with confidence
7. Keep oo1424oo as fallback

**The backup is your insurance policy** - don't skip it!
