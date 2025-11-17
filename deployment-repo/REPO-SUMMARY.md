# Operation D-Bus Deployment Repository Summary

## Purpose
This repository manages Btrfs-based deployment of Operation D-Bus using snapshot send/receive operations for efficient, atomic system deployments and upgrades.

## Key Components

### Source Environment (Build System)
- Golden environment: Btrfs subvolume with complete op-dbus installation
- Snapshot creation: Automated Btrfs snapshot generation
- Send stream generation: Incremental or full send streams for distribution

### Target Environment (Deployment System)
- Btrfs receive: Atomic deployment of snapshots
- Overlay mounts: Writable overlay on read-only snapshots
- System integration: Symlinks and systemd service management

### Upgrade/Rollback
- Rolling upgrades: Zero-downtime version transitions
- Atomic rollback: Instant reversion to previous versions
- Incremental updates: Only transfer changed data

## Workflow

### Build System
1. Update golden environment with new op-dbus version
2. Create Btrfs snapshot: `./btrfs/scripts/create-snapshot.sh v1.2.3`
3. Generate send stream: `./btrfs/scripts/send-snapshot.sh v1.2.3`
4. Release via GitHub Actions

### Target System
1. Prepare system: `./scripts/pre-deploy/prepare-target.sh`
2. Receive snapshot: `sudo btrfs receive /var/lib/op-dbus/deploy < snapshot.send`
3. Mount deployment: `./scripts/post-deploy/mount-deployment.sh v1.2.3`
4. Upgrade: `./scripts/upgrade/rolling-upgrade.sh v1.2.3 v1.3.0`

## Security Model
- Read-only deployments prevent tampering
- SHA-256 checksum verification
- GPG signature verification (planned)
- Audit trail via blockchain (inherited from op-dbus)

## Performance Characteristics
- Initial deployment: Full send stream (size of installation)
- Incremental updates: Only changed data transferred
- Atomic operations: All-or-nothing deployments
- Overlay filesystem: Minimal write amplification

## Integration Points
- GitHub Actions: Automated snapshot creation and releases
- GitHub Container Registry: Docker images for testing
- GitHub Releases: Send streams and metadata
- Systemd: Service management and dependencies
- D-Bus: System integration (inherited)

## Maintenance
- Automatic cleanup: Old snapshots removed after 3 versions
- Validation: Pre/post-deployment checks
- Monitoring: Service health and version tracking
- Backup: Deployment records and overlay preservation
