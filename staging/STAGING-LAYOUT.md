# Staging Layout Architecture

## Concept

Each component has a staging folder containing all its files organized cleanly. During deployment:

1. **Staging folders** are BTRFS subvolumes (easy to snapshot)
2. **Deployment** creates symlinks from staging → real system locations
3. **System is updated** with actual working files via symlinks
4. **Snapshots** capture the clean staging folder state
5. **Rollback** removes symlinks and restores staging snapshot

## Directory Structure

```
/var/lib/op-dbus/staging/
├── stage-1-core-dbus/              # BTRFS subvolume
│   ├── bin/
│   │   └── op-dbus                 # Binary
│   ├── etc/
│   │   └── op-dbus/
│   │       └── state.json          # Default config
│   ├── manifest.yaml               # Component metadata
│   └── .deployed                   # Deployment marker
│
├── stage-2-introspection-cache/    # BTRFS subvolume
│   ├── bin/
│   │   └── warm-dbus-cache.sh      # Cache warmup script
│   ├── systemd/
│   │   ├── dbus-cache-warmup.service
│   │   └── dbus-cache-warmup.timer
│   ├── var/
│   │   └── cache/
│   │       └── .gitkeep            # Cache directory
│   ├── manifest.yaml
│   └── .deployed
│
├── stage-3-numa-cache/             # BTRFS subvolume
│   ├── etc/
│   │   └── op-dbus/
│   │       └── numa-policy.json
│   ├── manifest.yaml
│   └── .deployed
│
├── stage-4-network-layer/          # BTRFS subvolume
│   ├── etc/
│   │   ├── openvswitch/
│   │   │   └── .gitkeep
│   │   └── op-dbus/
│   │       └── network.json
│   ├── systemd/
│   │   ├── openvswitch.service
│   │   ├── ovsdb-server.service
│   │   └── ovs-vswitchd.service
│   ├── manifest.yaml
│   └── .deployed
│
├── stage-5-mcp-server/             # BTRFS subvolume
│   ├── bin/
│   │   ├── dbus-mcp
│   │   └── mcp-chat
│   ├── web/
│   │   ├── index.html
│   │   ├── app.js
│   │   └── styles.css
│   ├── etc/
│   │   └── op-dbus/
│   │       └── mcp-config.json
│   ├── systemd/
│   │   └── dbus-mcp.service
│   ├── manifest.yaml
│   └── .deployed
│
└── snapshots/                      # BTRFS snapshot storage
    ├── pre-stage-1-20251117-140522/
    ├── post-stage-1-20251117-140534/
    ├── pre-stage-2-20251117-140601/
    └── post-stage-2-20251117-140615/
```

## Deployment Mapping

When a stage is deployed, files are symlinked from staging → system:

### Stage 1: Core D-Bus
```
/var/lib/op-dbus/staging/stage-1-core-dbus/bin/op-dbus
  → /usr/local/bin/op-dbus

/var/lib/op-dbus/staging/stage-1-core-dbus/etc/op-dbus/state.json
  → /etc/op-dbus/state.json (if not exists)
```

### Stage 2: Introspection Cache
```
/var/lib/op-dbus/staging/stage-2-introspection-cache/bin/warm-dbus-cache.sh
  → /usr/local/bin/warm-dbus-cache.sh

/var/lib/op-dbus/staging/stage-2-introspection-cache/systemd/dbus-cache-warmup.service
  → /etc/systemd/system/dbus-cache-warmup.service

/var/lib/op-dbus/staging/stage-2-introspection-cache/systemd/dbus-cache-warmup.timer
  → /etc/systemd/system/dbus-cache-warmup.timer
```

### Stage 5: MCP Server
```
/var/lib/op-dbus/staging/stage-5-mcp-server/bin/dbus-mcp
  → /usr/local/bin/dbus-mcp

/var/lib/op-dbus/staging/stage-5-mcp-server/web/
  → /var/www/op-dbus-mcp/ (directory symlink or copy)

/var/lib/op-dbus/staging/stage-5-mcp-server/systemd/dbus-mcp.service
  → /etc/systemd/system/dbus-mcp.service
```

## Snapshot Workflow

### Installation Flow:
```bash
1. Create staging subvolume if not exists
   btrfs subvolume create /var/lib/op-dbus/staging/stage-X-component

2. Pre-installation snapshot
   btrfs subvolume snapshot -r /var/lib/op-dbus/staging \
     /var/lib/op-dbus/staging/snapshots/pre-stage-X-TIMESTAMP

3. Build/copy files into staging folder
   cargo build --release
   cp target/release/op-dbus staging/stage-1-core-dbus/bin/

4. Create symlinks to system locations
   ln -sf /var/lib/op-dbus/staging/stage-1-core-dbus/bin/op-dbus \
          /usr/local/bin/op-dbus

5. Post-installation snapshot
   btrfs subvolume snapshot -r /var/lib/op-dbus/staging \
     /var/lib/op-dbus/staging/snapshots/post-stage-X-TIMESTAMP

6. Mark as deployed
   touch /var/lib/op-dbus/staging/stage-X-component/.deployed
```

### Rollback Flow:
```bash
1. Pre-rollback snapshot
   btrfs subvolume snapshot -r /var/lib/op-dbus/staging \
     /var/lib/op-dbus/staging/snapshots/pre-rollback-TIMESTAMP

2. Remove symlinks created by stage
   rm /usr/local/bin/op-dbus
   rm /etc/systemd/system/dbus-cache-warmup.*

3. Delete current staging subvolume for that stage
   btrfs subvolume delete /var/lib/op-dbus/staging/stage-X-component

4. Restore from snapshot
   btrfs subvolume snapshot \
     /var/lib/op-dbus/staging/snapshots/pre-stage-X-TIMESTAMP \
     /var/lib/op-dbus/staging/stage-X-component

5. Re-verify system state
```

## Benefits

1. **Clean Organization**: Each stage's files are in one clear location
2. **Easy Snapshots**: Snapshot entire staging tree, not scattered system files
3. **Real System Updates**: Symlinks mean binaries/services actually work
4. **Atomic Rollback**: Delete stage folder + restore snapshot
5. **Visual Clarity**: `ls staging/` shows exactly what stages are deployed
6. **Space Efficient**: BTRFS CoW means snapshots use minimal space

## Stage Manifest Example

Each stage folder contains `manifest.yaml`:

```yaml
stage: 1
component: core-dbus
version: 1.0.0
description: Core D-Bus infrastructure

# Files in this staging folder
files:
  - path: bin/op-dbus
    target: /usr/local/bin/op-dbus
    mode: 0755
    type: symlink

  - path: etc/op-dbus/state.json
    target: /etc/op-dbus/state.json
    mode: 0644
    type: copy-if-not-exists  # Don't overwrite user config

# Dependencies
dependencies:
  - D-Bus system bus
  - Rust toolchain (build time only)

# Verification
verify:
  - command: "op-dbus --version"
    expect_exit: 0
  - command: "dbus-send --system --print-reply --dest=org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.ListNames"
    expect_exit: 0

# Cleanup on uninstall
cleanup:
  - /etc/op-dbus/state.json (if default, remove; if modified, keep)
```

## Implementation Phases

### Phase 1: Staging Setup
- Create staging directory structure
- Initialize BTRFS subvolumes for each stage
- Populate staging folders with component files

### Phase 2: Deployment Engine
- Read manifest files
- Create symlinks from staging → system
- Handle different file types (symlink, copy, copy-if-not-exists)
- Enable systemd services

### Phase 3: Snapshot Management
- Create snapshots before/after each stage
- List snapshots with metadata
- Rollback to specific snapshots

### Phase 4: Verification
- Run verification commands
- Check symlinks are valid
- Verify services are running

## Usage Examples

```bash
# Initialize staging environment
sudo op-dbus-stage init

# This creates:
# /var/lib/op-dbus/staging/ (BTRFS subvolume)

# List available stages (shows what's in repo vs what's deployed)
op-dbus-stage list
# Output:
#   ✓ stage-1-core-dbus           [deployed]
#   ✓ stage-2-introspection-cache [deployed]
#   ○ stage-3-numa-cache          [available]
#   ○ stage-4-network-layer       [available]

# Deploy a stage
sudo op-dbus-stage deploy stage-3-numa-cache

# This:
# 1. Creates snapshot pre-stage-3-TIMESTAMP
# 2. Builds/copies files to staging/stage-3-numa-cache/
# 3. Symlinks files to system locations
# 4. Creates snapshot post-stage-3-TIMESTAMP

# List snapshots
op-dbus-stage snapshots
# Output:
#   pre-stage-1-20251117-140522   [195 MB]
#   post-stage-1-20251117-140534  [196 MB]
#   pre-stage-2-20251117-140601   [196 MB]
#   post-stage-2-20251117-140615  [201 MB]
#   pre-stage-3-20251117-140701   [201 MB]

# Rollback stage 3 (removes it, restores to pre-stage-3 state)
sudo op-dbus-stage rollback stage-3

# Verify all deployed stages
sudo op-dbus-stage verify
```
