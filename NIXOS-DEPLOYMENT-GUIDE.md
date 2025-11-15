# NixOS Deployment Guide for op-dbus

This guide walks you through deploying op-dbus on your personal NixOS laptop/workstation, creating an authoritative reference implementation for **enterprise end-user computing**.

Enterprise infrastructure includes:
- **Servers**: Data centers, cloud instances, DGX systems
- **Workstations**: Developer machines, engineering workstations
- **Laptops**: Mobile workforce, executive laptops, field engineers

This guide focuses on **laptop/workstation deployment** - the real-world testing scenario.

## Prerequisites

### 1. NixOS System
```bash
# Verify you're running NixOS
nixos-version
# Expected: NixOS 23.11 or later

# Check your architecture
uname -m
# Expected: x86_64 (for NUMA support)
```

### 2. BTRFS Filesystem (Recommended)
op-dbus blockchain requires BTRFS for optimal performance:

```bash
# Check current filesystem
df -T /var/lib
# If not BTRFS, you'll need to create a BTRFS partition/subvolume

# Example: Create BTRFS partition
sudo mkfs.btrfs -L opdbus /dev/sdX  # Replace with your device
sudo mkdir -p /var/lib/op-dbus
sudo mount -t btrfs /dev/disk/by-label/opdbus /var/lib/op-dbus
```

Add to `/etc/nixos/hardware-configuration.nix`:
```nix
fileSystems."/var/lib/op-dbus" = {
  device = "/dev/disk/by-label/opdbus";
  fsType = "btrfs";
  options = [ "compress=zstd" "noatime" ];
};
```

### 3. D-Bus (Should be enabled by default)
```bash
# Verify D-Bus is running
systemctl status dbus
```

### 4. NUMA Detection (Optional, for multi-socket systems)
```bash
# Check if NUMA is available
ls /sys/devices/system/node/
# Expected: node0, node1, ... (or just node0 on single-socket)

# Check NUMA topology
numactl --hardware
# Shows: available nodes, CPUs per node, memory per node
```

## Installation Methods

### Method 1: From Flake (Recommended)

This repo provides a Nix flake for reproducible builds:

```bash
# Clone the repository
git clone https://github.com/repr0bated/operation-dbus
cd operation-dbus

# Build the package
nix build

# Test the binary
./result/bin/op-dbus --version

# Install system-wide
sudo nix profile install .#op-dbus
```

Then import the module in `/etc/nixos/configuration.nix`:
```nix
{
  inputs.op-dbus.url = "github:repr0bated/operation-dbus";

  outputs = { self, nixpkgs, op-dbus }: {
    nixosConfigurations.myhostname = nixpkgs.lib.nixosSystem {
      modules = [
        op-dbus.nixosModules.default
        ./configuration.nix
      ];
    };
  };
}
```

### Method 2: Local Development Build

For testing local changes:

```bash
cd operation-dbus

# Copy the example configuration
sudo cp nixos-example-configuration.nix /etc/nixos/op-dbus.nix

# Edit your main configuration
sudo nano /etc/nixos/configuration.nix
```

Add to `/etc/nixos/configuration.nix`:
```nix
{ config, pkgs, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ./op-dbus.nix
  ];

  # ... rest of your config
}
```

## Configuration

### Basic Configuration (Laptop/Workstation)

Edit `/etc/nixos/op-dbus.nix`:

```nix
{ config, pkgs, ... }:

{
  imports = [ /path/to/operation-dbus/nixos-module.nix ];

  services.op-dbus = {
    enable = true;

    # Laptop-specific state configuration
    state = {
      version = 1;
      plugins = {
        systemd = {
          units = {
            # Essential services for laptops
            "NetworkManager.service" = {
              active_state = "active";
              enabled = true;
            };
            "bluetooth.service" = {
              active_state = "active";
              enabled = true;
            };

            # Disable services not needed on laptops
            "cups.service" = {
              active_state = "inactive";  # No printer by default
              enabled = false;
            };
          };
        };
      };
    };

    # Enable blockchain audit trail
    # Useful for tracking configuration changes on developer machines
    blockchain = {
      enable = true;
      snapshotInterval = "hourly";  # Less frequent on laptops (battery)
      retention = {
        hourly = 5;
        daily = 3;
        weekly = 2;
        quarterly = 2;
      };
    };

    # NUMA disabled for single-socket laptops
    # Will auto-detect and fall back gracefully
    numa.enable = true;  # Safe to enable, will detect single-node
  };
}
```

### Enterprise Laptop Management (Fleet Deployment)

For managing a fleet of developer/executive laptops:

```nix
{ config, pkgs, ... }:

{
  services.op-dbus = {
    enable = true;

    state = {
      version = 1;
      plugins = {
        systemd = {
          units = {
            # Security: Ensure firewall is active
            "firewalld.service" = {
              active_state = "active";
              enabled = true;
            };

            # Corporate VPN (example: WireGuard)
            "wg-quick-corporate.service" = {
              active_state = "active";
              enabled = true;
            };

            # Remote management
            "sshd.service" = {
              active_state = "active";
              enabled = true;
            };

            # Battery optimization
            "tlp.service" = {
              active_state = "active";
              enabled = true;
            };

            # Backup (optional)
            "restic-backup.timer" = {
              active_state = "active";
              enabled = true;
            };
          };
        };
      };
    };

    # Blockchain for compliance/audit
    # Track all configuration changes on corporate laptops
    blockchain = {
      enable = true;
      snapshotInterval = "every-30-minutes";
      retention = {
        hourly = 8;      # Last work day
        daily = 7;       # Last week
        weekly = 4;      # Last month
        quarterly = 4;   # Last year (compliance)
      };
    };

    # Single-socket optimization (typical laptop)
    numa = {
      enable = true;
      strategy = "local-node";  # Will detect node 0 only
    };

    # Lightweight cache for fast operations
    cache = {
      compression = "zstd";
      maxSnapshots = 12;  # Less than servers
    };
  };
}
```

### Enterprise Configuration (DGX/Multi-Socket)

For high-performance multi-socket systems:

```nix
services.op-dbus = {
  enable = true;

  # Aggressive blockchain retention for compliance
  blockchain = {
    enable = true;
    snapshotInterval = "every-5-minutes";
    retention = {
      hourly = 24;     # Last 24 hours
      daily = 30;      # Last month
      weekly = 12;     # Last quarter
      quarterly = 8;   # Last 2 years
    };
  };

  # NUMA optimization for DGX
  numa = {
    enable = true;
    strategy = "local-node";  # Pin to local NUMA node
    # nodePreference = 0;     # Uncomment to force node 0
  };

  # Aggressive cache compression
  cache = {
    compression = "zstd";
    maxSnapshots = 48;
  };
};
```

## Deployment Steps

### 1. Verify Configuration Syntax

```bash
# Check for Nix syntax errors
sudo nixos-rebuild dry-build
```

### 2. Apply Configuration

```bash
# Apply the configuration (doesn't switch boot)
sudo nixos-rebuild test

# If successful, make it permanent
sudo nixos-rebuild switch
```

### 3. Verify Service Status

```bash
# Check op-dbus service
sudo systemctl status op-dbus

# View logs
sudo journalctl -u op-dbus -f

# Check NUMA detection
op-dbus cache numa-info

# View blockchain status
op-dbus blockchain snapshots
```

### 4. Validate NUMA Optimization

```bash
# Check detected topology
op-dbus cache numa-info

# Expected output:
# NUMA Topology
# ============
# Node 0: 8 CPUs, 32768 MB RAM
# Node 1: 8 CPUs, 32768 MB RAM
#
# Current Node: 0
# Optimal Node: 0
# Strategy: local-node

# Monitor NUMA statistics
op-dbus cache numa-stats

# Expected output shows local vs remote hit rates:
# NUMA Statistics
# ==============
# Node 0: 94.2% local hits, avg 45ns latency
# Node 1: 87.3% local hits, avg 52ns latency
```

### 5. Test Blockchain Functionality

```bash
# Apply a simple state change
sudo op-dbus apply

# List blockchain snapshots
op-dbus blockchain snapshots

# View a specific snapshot
op-dbus blockchain show <snapshot-id>

# Test rollback (BE CAREFUL!)
# sudo op-dbus blockchain rollback <snapshot-id>
```

### 6. Verify State Persistence

```bash
# View current state
cat /etc/op-dbus/state.json

# View blockchain state record
cat /var/lib/op-dbus/blockchain/state/current.json

# They should match!
diff <(jq --sort-keys . /etc/op-dbus/state.json) \
     <(jq --sort-keys . /var/lib/op-dbus/blockchain/state/current.json)
```

## Performance Validation

### Benchmark Cache Performance

```bash
# Generate some embeddings to test cache
# (Requires ML features to be enabled)

# Check cache statistics
op-dbus cache stats

# Example output:
# Cache Statistics
# ===============
# Total Entries: 15,420
# Hit Rate: 94.2%
# Memory Usage: 45 MB
# NUMA Local Hits: 94.2%
# Avg Latency: 45ns
```

### Benchmark Blockchain Performance

```bash
# Time a simple state apply
time sudo op-dbus apply

# Expected: < 50ms for simple operations
# BTRFS subvolumes make this very fast
```

### Stress Test (Optional)

```bash
# Rapid state changes to test retention policy
for i in {1..100}; do
  sudo op-dbus apply
  sleep 60
done

# Check snapshot count (should be bounded by retention policy)
op-dbus blockchain snapshots | wc -l

# Expected: ~30-50 snapshots (depending on retention settings)
```

## Troubleshooting

### Issue: op-dbus service fails to start

```bash
# Check logs
sudo journalctl -u op-dbus -n 50

# Common causes:
# 1. BTRFS not mounted → Check fileSystems in hardware-configuration.nix
# 2. D-Bus not available → Ensure services.dbus.enable = true
# 3. Permission issues → Check that /var/lib/op-dbus is writable
```

### Issue: NUMA detection fails

```bash
# Check if NUMA is available
ls /sys/devices/system/node/

# If no node* directories, your system doesn't support NUMA
# op-dbus will gracefully fall back to single-node mode

# Verify in logs:
sudo journalctl -u op-dbus | grep -i numa
# Should see: "NUMA topology detected: 1 nodes" (single-socket)
# or:         "NUMA topology detected: 2 nodes" (multi-socket)
```

### Issue: Blockchain snapshots filling disk

```bash
# Check disk usage
du -sh /var/lib/op-dbus/blockchain/snapshots

# List snapshots
op-dbus blockchain snapshots

# Manually trigger pruning
sudo systemctl restart op-dbus

# Or adjust retention policy in configuration.nix:
blockchain.retention.hourly = 5;  # Reduce from default 10
```

### Issue: State drift (current.json doesn't match state.json)

```bash
# Force reconciliation
sudo op-dbus apply --force

# This will:
# 1. Read /etc/op-dbus/state.json
# 2. Apply it to the system
# 3. Update blockchain/state/current.json
# 4. Create a snapshot
```

## Monitoring in Production

### Systemd Journal

```bash
# Follow logs in real-time
sudo journalctl -u op-dbus -f

# Filter for errors
sudo journalctl -u op-dbus -p err

# Export logs for analysis
sudo journalctl -u op-dbus --since "1 hour ago" -o json > opdbus-logs.json
```

### Prometheus Metrics (Optional)

If you're using Prometheus:

```nix
services.prometheus = {
  enable = true;
  exporters.node = {
    enable = true;
    enabledCollectors = [ "systemd" "btrfs" ];
  };
};
```

Then scrape:
- `node_systemd_unit_state{name="op-dbus.service"}`
- `node_btrfs_info{mountpoint="/var/lib/op-dbus"}`

### Health Check Script

Create `/usr/local/bin/opdbus-health.sh`:
```bash
#!/usr/bin/env bash

echo "=== op-dbus Health Check ==="

# Service status
if systemctl is-active --quiet op-dbus; then
  echo "✓ Service running"
else
  echo "✗ Service NOT running"
  exit 1
fi

# Blockchain directory
if [ -d /var/lib/op-dbus/blockchain ]; then
  echo "✓ Blockchain directory exists"
else
  echo "✗ Blockchain directory missing"
  exit 1
fi

# Recent snapshot (within last hour)
LATEST=$(ls -t /var/lib/op-dbus/blockchain/snapshots | head -1)
if [ -n "$LATEST" ]; then
  AGE=$(( $(date +%s) - $(stat -c %Y "/var/lib/op-dbus/blockchain/snapshots/$LATEST") ))
  if [ $AGE -lt 3600 ]; then
    echo "✓ Recent snapshot found ($AGE seconds old)"
  else
    echo "⚠ No snapshot in last hour"
  fi
else
  echo "✗ No snapshots found"
fi

echo "=== Health check complete ==="
```

Make it executable and run:
```bash
sudo chmod +x /usr/local/bin/opdbus-health.sh
sudo /usr/local/bin/opdbus-health.sh
```

## Next Steps

1. **Production Deployment**: Once validated on your personal computer, export the configuration to deploy on production servers

2. **Backup Strategy**: Set up BTRFS send/receive to backup blockchain to external storage:
   ```bash
   sudo btrfs send /var/lib/op-dbus/blockchain/snapshots/snapshot-123 | \
     ssh backup@server 'btrfs receive /backup/opdbus'
   ```

3. **Multi-Node Testing**: If you have access to multi-socket hardware or DGX, validate NUMA performance improvements

4. **Integration Testing**: Test with your actual infrastructure (containers, network config, etc.)

5. **Contribute Back**: Share your configuration and lessons learned with the community!

## Reference

- **NixOS Manual**: https://nixos.org/manual/nixos/stable/
- **BTRFS Wiki**: https://btrfs.wiki.kernel.org/
- **NUMA Architecture**: https://www.kernel.org/doc/html/latest/vm/numa.html
- **op-dbus Repository**: https://github.com/repr0bated/operation-dbus
- **Issue Tracker**: https://github.com/repr0bated/operation-dbus/issues

---

**Note**: This is an authoritative test deployment. Document any issues or improvements you discover during testing - they'll help improve the production deployment guide.
