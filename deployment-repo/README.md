# Operation D-Bus Btrfs Deployment Repository

This repository manages Btrfs-based deployment of Operation D-Bus using snapshot send/receive operations for efficient, atomic system deployments and upgrades.

## Overview

The deployment strategy uses Btrfs snapshots to capture complete system states and distribute them via `btrfs send`/`btrfs receive` operations. This provides:

- **Atomic deployments**: All-or-nothing system state changes
- **Efficient updates**: Only changed data is transferred
- **Instant rollbacks**: Snapshot-based rollback capability
- **Immutable deployments**: Read-only deployed snapshots

## Repository Structure

```
├── btrfs/             # Btrfs deployment artifacts
│   ├── snapshots/    # Btrfs snapshot metadata and configs
│   └── scripts/      # Btrfs send/receive utilities
├── validation/       # Pre/post-deployment validation
├── scripts/          # Deployment orchestration scripts
│   ├── pre-deploy/   # Pre-deployment preparation
│   ├── post-deploy/  # Post-deployment configuration
│   └── rollback/     # Rollback procedures
└── ci-cd/           # CI/CD pipeline definitions
```

## Deployment Architecture

### Source Environment (Build System)

A dedicated build system maintains the "golden" Operation D-Bus environment:

```
Source System (Build Server)
├── Btrfs subvolume: @op-dbus-golden
│   ├── /usr/local/bin/op-dbus (compiled binary)
│   ├── /etc/op-dbus/ (configuration)
│   ├── /var/lib/op-dbus/ (state/data)
│   ├── /lib/systemd/system/op-dbus.service
│   └── Dependencies (OpenSSL, etc.)
├── Btrfs snapshots: @op-dbus-v1.0.0, @op-dbus-v1.1.0, etc.
└── Send streams: op-dbus-v1.1.0.send
```

### Target Environment (Deployment System)

Target systems receive and mount the immutable deployment:

```
Target System
├── Btrfs subvolume: @op-dbus-deploy (read-only mount)
│   ├── /opt/op-dbus/ (mounted deployment)
│   └── All files from source snapshot
├── Overlay mount: /usr/local/op-dbus/ (writable overlay)
└── System integration via symlinks/service config
```

## Quick Start

### Prepare Source Environment

```bash
# On build system - ensure environment is ready
cd deployment-repo
sudo ./scripts/pre-deploy/validate-source.sh

# Create deployment snapshot
sudo ./btrfs/scripts/create-snapshot.sh v1.2.3

# Generate send stream
sudo ./btrfs/scripts/send-snapshot.sh v1.2.3 > op-dbus-v1.2.3.send
```

### Deploy to Target System

```bash
# On target system - receive deployment
sudo ./scripts/pre-deploy/prepare-target.sh

# Receive the snapshot
sudo btrfs receive /var/lib/op-dbus/deploy < op-dbus-v1.2.3.send

# Mount and integrate
sudo ./scripts/post-deploy/mount-deployment.sh v1.2.3

# Validate deployment
sudo ./validation/post-deploy-check.sh
```

## Source Environment Setup

### Requirements

- Ubuntu 22.04+ or Debian 11+
- Btrfs root filesystem
- Operation D-Bus built and tested
- All dependencies installed

### Initial Setup

```bash
# Create dedicated subvolume for golden environment
sudo btrfs subvolume create /@op-dbus-golden

# Mount it for development
sudo mount -o subvol=@op-dbus-golden /dev/mapper/vg0-root /mnt/golden

# Copy current system as baseline
sudo rsync -a --exclude=/mnt --exclude=/proc --exclude=/sys --exclude=/dev / /mnt/golden/

# Install op-dbus in the golden environment
cd /path/to/operation-dbus
sudo make install DESTDIR=/mnt/golden
```

### Update Process

```bash
# Update golden environment
sudo mount -o subvol=@op-dbus-golden /dev/mapper/vg0-root /mnt/golden

# Update op-dbus
cd /mnt/golden/path/to/operation-dbus
git pull
cargo build --release
sudo make install

# Update dependencies
sudo apt update && sudo apt upgrade

# Unmount and snapshot
sudo umount /mnt/golden
sudo btrfs subvolume snapshot /@op-dbus-golden /@op-dbus-v1.2.3
```

## Target Environment Setup

### Requirements

- Ubuntu 22.04+ or Debian 11+
- Btrfs root filesystem
- Network access to deployment repository

### Preparation

```bash
# Create deployment directory structure
sudo mkdir -p /var/lib/op-dbus/{deploy,overlays}

# Ensure btrfs tools are available
sudo apt install btrfs-progs
```

### Deployment Process

1. **Pre-deployment validation**:
   ```bash
   sudo ./scripts/pre-deploy/validate-target.sh
   ```

2. **Receive snapshot**:
   ```bash
   # Download send stream
   wget https://github.com/repr0bated/operation-dbus-deployment/releases/download/v1.2.3/op-dbus-v1.2.3.send

   # Receive into deployment subvolume
   sudo btrfs receive /var/lib/op-dbus/deploy < op-dbus-v1.2.3.send
   ```

3. **Mount deployment**:
   ```bash
   sudo ./scripts/post-deploy/mount-deployment.sh v1.2.3
   ```

4. **Integration**:
   ```bash
   sudo ./scripts/post-deploy/integrate-system.sh
   ```

## Validation Scripts

### Source Validation

```bash
# Validate golden environment is ready
sudo ./validation/validate-golden.sh

# Check all required files exist
sudo ./validation/check-dependencies.sh

# Verify op-dbus functionality
sudo ./validation/test-op-dbus.sh
```

### Target Validation

```bash
# Post-deployment checks
sudo ./validation/post-deploy-check.sh

# Service functionality tests
sudo ./validation/test-services.sh

# Rollback capability check
sudo ./validation/test-rollback.sh
```

## Upgrade Process

### Rolling Upgrade

```bash
# On target system
sudo ./scripts/upgrade/rolling-upgrade.sh v1.2.3 v1.3.0

# This will:
# 1. Download new send stream
# 2. Receive into new subvolume
# 3. Switch overlay mount atomically
# 4. Validate new version
# 5. Cleanup old deployment
```

### Rollback

```bash
# Immediate rollback to previous version
sudo ./scripts/rollback/perform-rollback.sh

# Or rollback to specific version
sudo ./scripts/rollback/rollback-to.sh v1.2.3
```

## CI/CD Pipeline

### Automated Build Process

```yaml
# .github/workflows/build.yml
name: Build Deployment Snapshot

on:
  push:
    tags: ['v*']

jobs:
  build:
    runs-on: [self-hosted, btrfs]
    steps:
      - name: Update golden environment
        run: ./scripts/ci/update-golden.sh ${{ github.ref_name }}

      - name: Create snapshot
        run: sudo ./btrfs/scripts/create-snapshot.sh ${{ github.ref_name }}

      - name: Generate send stream
        run: sudo ./btrfs/scripts/send-snapshot.sh ${{ github.ref_name }} > snapshot.send

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: op-dbus-${{ github.ref_name }}.send
          path: snapshot.send
```

### Release Process

1. **Tag release** in main operation-dbus repository
2. **CI triggers** deployment repository build
3. **Golden environment updates** with new op-dbus version
4. **Snapshot created** and send stream generated
5. **Release published** with deployment artifact

## Scripts Overview

### Pre-deployment
- `validate-source.sh`: Ensure source environment is deployment-ready
- `prepare-target.sh`: Prepare target system for deployment
- `check-compatibility.sh`: Verify target system compatibility

### Deployment
- `create-snapshot.sh`: Create Btrfs snapshot of golden environment
- `send-snapshot.sh`: Generate send stream from snapshot
- `receive-snapshot.sh`: Receive and mount deployment on target
- `mount-deployment.sh`: Set up overlay mounts and symlinks

### Post-deployment
- `integrate-system.sh`: Configure systemd services and paths
- `cleanup-old.sh`: Remove previous deployment artifacts
- `validate-deployment.sh`: Comprehensive post-deploy validation

### Upgrade/Rollback
- `rolling-upgrade.sh`: Zero-downtime upgrade between versions
- `perform-rollback.sh`: Immediate rollback to previous version
- `rollback-to.sh`: Rollback to specific version

## Security Considerations

### Snapshot Integrity
- SHA-256 checksums for all send streams
- GPG signature verification for releases
- Immutable read-only deployments

### Access Control
- Restricted access to golden environment
- Audit logging of all deployment operations
- Service account isolation

## Troubleshooting

### Common Issues

**Send stream corrupted**:
```bash
# Verify checksum
sha256sum -c op-dbus-v1.2.3.send.sha256

# Re-download if corrupted
wget -c https://github.com/.../op-dbus-v1.2.3.send
```

**Mount failures**:
```bash
# Check btrfs filesystem
sudo btrfs filesystem show

# Repair if needed
sudo btrfs check --repair /dev/mapper/vg0-root
```

**Service integration issues**:
```bash
# Check systemd service status
sudo systemctl status op-dbus

# View service logs
sudo journalctl -u op-dbus -f
```

## Contributing

1. Test changes on isolated Btrfs environment
2. Update validation scripts for new checks
3. Document any new deployment requirements
4. Ensure rollback compatibility

## Related Documentation

- [Operation D-Bus Main Repository](https://github.com/repr0bated/operation-dbus)
- [Btrfs Send/Receive Documentation](https://btrfs.readthedocs.io/en/latest/btrfs-send.html)
- [Systemd Integration Guide](https://github.com/repr0bated/operation-dbus/blob/master/docs/systemd-integration.md)