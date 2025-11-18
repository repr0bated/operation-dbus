# Staging Deployment Guide

**How to use the staging system for deploying op-dbus with plugins**

## Overview

The staging system uses BTRFS subvolumes and symlinks to deploy components incrementally:
- Each **stage** is a BTRFS subvolume containing component files
- **Deployment** creates symlinks from staging → system locations
- **Snapshots** capture state before/after each stage
- **Plugins** are deployed as part of their respective stages

## Quick Start

### 1. Initialize Staging Environment

```bash
sudo op-dbus image init
# Creates: /var/lib/op-dbus/deployment/
```

### 2. Create Deployment Image with Plugins

```bash
# Create image with core binary and plugins
sudo op-dbus image create PROXMOX-DBUS_STAGE \
  --files target/release/op-dbus \
  --files src/state/plugins/*.rs \
  --files schemas/*.toml
```

### 3. Deploy from Image

```bash
# Stream snapshot to target system
btrfs send /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE/snapshot \
  | ssh target-host 'btrfs receive /var/lib/op-dbus/deployment/'
```

## Staging Workflow with Plugins

### Stage 1: Core D-Bus + Base Plugins

```bash
# Initialize staging
sudo op-dbus-stage init

# Deploy Stage 1: Core binary + essential plugins
sudo op-dbus-stage deploy stage-1-core-dbus

# This deploys:
# - /usr/local/bin/op-dbus (binary)
# - /etc/op-dbus/state.json (default config)
# - Core plugins: net, systemd, login1
```

**Manifest** (`staging/stage-1-core-dbus/manifest.yaml`):
```yaml
stage: 1
component: core-dbus
version: 1.0.0
plugins:
  - net
  - systemd
  - login1
files:
  - path: bin/op-dbus
    target: /usr/local/bin/op-dbus
    type: symlink
  - path: etc/op-dbus/state.json
    target: /etc/op-dbus/state.json
    type: copy-if-not-exists
```

### Stage 2: Plugin Extensions

```bash
# Deploy Stage 2: Additional plugins
sudo op-dbus-stage deploy stage-2-plugins

# This deploys:
# - LXC plugin
# - PackageKit plugin
# - DNS resolver plugin
# - PCI declaration plugin
```

**Manifest** (`staging/stage-2-plugins/manifest.yaml`):
```yaml
stage: 2
component: plugins
version: 1.0.0
plugins:
  - lxc
  - packagekit
  - dnsresolver
  - pcidecl
dependencies:
  - stage-1-core-dbus
```

### Stage 3: OpenFlow + Privacy Router Plugins

```bash
# Deploy Stage 3: OpenFlow and privacy features
sudo op-dbus-stage deploy stage-3-openflow

# This deploys:
# - OpenFlow plugin
# - Privacy router plugin
# - Netmaker plugin
# - Privacy plugin
```

**Manifest** (`staging/stage-3-openflow/manifest.yaml`):
```yaml
stage: 3
component: openflow-privacy
version: 1.0.0
plugins:
  - openflow
  - privacy_router
  - privacy
  - netmaker
dependencies:
  - stage-1-core-dbus
  - stage-2-plugins
requires_feature: openflow
```

### Stage 4: MCP Server + Auto-Plugins

```bash
# Deploy Stage 4: MCP server and auto-plugin discovery
sudo op-dbus-stage deploy stage-4-mcp

# This deploys:
# - MCP server binaries
# - Auto-plugin discovery
# - Introspection cache
```

## Plugin Deployment Details

### Plugin Files Structure

Each plugin stage contains:

```
stage-X-plugins/
├── plugins/
│   ├── lxc/
│   │   ├── plugin.rs          # Plugin source (if needed)
│   │   ├── plugin.toml        # Plugin metadata
│   │   └── examples/          # Example configs
│   ├── packagekit/
│   │   ├── plugin.toml
│   │   └── examples/
│   └── ...
├── schemas/                   # JSON schemas for validation
│   ├── lxc.schema.json
│   └── packagekit.schema.json
├── manifest.yaml              # Stage manifest
└── .deployed                  # Deployment marker
```

### Plugin Registration

Plugins are automatically registered when deployed:

```bash
# After deploying stage with plugins, they're registered in:
/etc/op-dbus/plugins/
├── lxc.toml
├── packagekit.toml
└── ...

# And loaded by op-dbus on startup
```

## Complete Deployment Example

### Step-by-Step: Deploy Full System with All Plugins

```bash
# 1. Initialize staging
sudo op-dbus-stage init

# 2. Build all components
cargo build --release

# 3. Deploy Stage 1: Core
sudo op-dbus-stage deploy stage-1-core-dbus
# ✓ Creates snapshot: pre-stage-1-TIMESTAMP
# ✓ Symlinks: /usr/local/bin/op-dbus
# ✓ Creates snapshot: post-stage-1-TIMESTAMP

# 4. Deploy Stage 2: Base Plugins
sudo op-dbus-stage deploy stage-2-plugins
# ✓ Deploys: lxc, packagekit, dnsresolver, pcidecl
# ✓ Creates snapshots before/after

# 5. Deploy Stage 3: OpenFlow (if feature enabled)
sudo op-dbus-stage deploy stage-3-openflow
# ✓ Deploys: openflow, privacy_router, privacy, netmaker
# ✓ Creates snapshots before/after

# 6. Deploy Stage 4: MCP Server
sudo op-dbus-stage deploy stage-4-mcp
# ✓ Deploys: MCP servers, auto-plugin discovery
# ✓ Creates snapshots before/after

# 7. Verify deployment
sudo op-dbus-stage verify
# ✓ Checks all symlinks
# ✓ Verifies plugins are registered
# ✓ Tests plugin functionality
```

## Rollback with Plugins

### Rollback a Specific Stage

```bash
# Rollback Stage 3 (removes OpenFlow plugins)
sudo op-dbus-stage rollback stage-3-openflow

# This:
# 1. Creates snapshot: pre-rollback-stage-3-TIMESTAMP
# 2. Removes symlinks created by stage-3
# 3. Removes plugin registrations
# 4. Restores staging folder from pre-stage-3 snapshot
```

### Rollback All Stages

```bash
# Rollback to before Stage 1
sudo op-dbus-stage rollback --all

# Or rollback to specific snapshot
sudo op-dbus-stage rollback --snapshot pre-stage-2-TIMESTAMP
```

## Plugin-Specific Deployment

### Deploy Single Plugin

```bash
# Deploy just the LXC plugin
sudo op-dbus-stage deploy-plugin lxc

# This:
# 1. Copies plugin files to staging
# 2. Registers plugin in /etc/op-dbus/plugins/
# 3. Creates snapshot
```

### Update Plugin

```bash
# Update existing plugin
sudo op-dbus-stage update-plugin lxc

# This:
# 1. Creates snapshot before update
# 2. Updates plugin files
# 3. Re-registers plugin
# 4. Creates snapshot after update
```

## Using Deployment Images (Alternative)

### Create Image with All Plugins

```bash
# Create image containing all plugins
sudo op-dbus image create PROXMOX-DBUS_STAGE \
  --files target/release/op-dbus \
  --files src/state/plugins/lxc.rs \
  --files src/state/plugins/packagekit.rs \
  --files src/state/plugins/openflow.rs \
  --files src/state/plugins/privacy_router.rs \
  --files schemas/lxc.schema.json \
  --files schemas/packagekit.schema.json

# Image is created at:
# /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE/
```

### Stream Image to Target

```bash
# On source system
btrfs send /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE/snapshot \
  | ssh target-host 'btrfs receive /var/lib/op-dbus/deployment/'

# On target system
cd /var/lib/op-dbus/deployment/images/PROXMOX-DBUS_STAGE
sudo op-dbus-stage deploy-from-image .
```

## Practical Example: Deploy Privacy Router with Plugins

### Step 1: Build and Prepare

```bash
# Build release binary
cargo build --release

# Initialize deployment directory
sudo op-dbus image init
```

### Step 2: Create Image with Privacy Router Components

```bash
# Create image with all privacy router files
sudo op-dbus image create PROXMOX-DBUS_STAGE \
  --files target/release/op-dbus \
  --files src/state/plugins/privacy_router.rs \
  --files src/state/plugins/openflow.rs \
  --files src/state/plugins/netmaker.rs \
  --files src/state/plugins/privacy.rs \
  --files src/state/plugins/lxc.rs \
  --files src/state/plugins/net.rs \
  --files example-privacy-network.json \
  --files privacy-tunnel-state.json
```

### Step 3: Deploy Using Staging System

```bash
# Initialize staging
sudo op-dbus-stage init

# Deploy core (required first)
sudo op-dbus-stage deploy stage-1-core-dbus

# Deploy network layer (required for privacy router)
sudo op-dbus-stage deploy stage-4-network-layer

# Deploy OpenFlow + Privacy Router
sudo op-dbus-stage deploy stage-3-openflow

# Verify plugins are registered
op-dbus query --plugin privacy_router
op-dbus query --plugin openflow
op-dbus query --plugin netmaker
```

### Step 4: Apply Privacy Router Configuration

```bash
# Apply privacy router state
sudo op-dbus apply privacy-tunnel-state.json

# This uses the deployed plugins to:
# - Create OVS bridge
# - Set up WireGuard gateway container
# - Configure WARP tunnel
# - Set up XRay client container
# - Configure OpenFlow privacy flows
# - Set up Netmaker mesh
```

### Step 5: Verify Deployment

```bash
# Check all plugins are working
op-dbus query --plugin all

# Check privacy router state
op-dbus query --plugin privacy_router

# List snapshots (for rollback if needed)
op-dbus-stage snapshots
```

## Verification

### Check Deployed Plugins

```bash
# List all deployed plugins
op-dbus query --plugin all

# Check specific plugin
op-dbus query --plugin lxc

# Verify plugin is registered
ls -la /etc/op-dbus/plugins/
```

### Test Plugin Functionality

```bash
# Test LXC plugin
op-dbus query --plugin lxc

# Test network plugin
op-dbus query --plugin net

# Test with state file
op-dbus diff /etc/op-dbus/state.json --plugin lxc
```

## Snapshot Management

### List Snapshots

```bash
op-dbus-stage snapshots

# Output:
# pre-stage-1-20251117-140522   [195 MB]  stage-1-core-dbus
# post-stage-1-20251117-140534  [196 MB]  stage-1-core-dbus
# pre-stage-2-20251117-140601   [196 MB]  stage-2-plugins
# post-stage-2-20251117-140615  [201 MB]  stage-2-plugins
```

### Restore from Snapshot

```bash
# Restore to specific snapshot
sudo op-dbus-stage restore pre-stage-2-20251117-140601

# This restores all stages up to that point
```

## Best Practices

1. **Always snapshot before deployment**: Automatic with `op-dbus-stage deploy`
2. **Deploy in order**: Stage 1 → Stage 2 → Stage 3 → Stage 4
3. **Verify after each stage**: `op-dbus-stage verify`
4. **Test plugins before next stage**: `op-dbus query --plugin <name>`
5. **Keep snapshots**: Don't delete snapshots until deployment is verified

## Troubleshooting

### Plugin Not Found After Deployment

```bash
# Check if plugin file exists
ls -la /etc/op-dbus/plugins/<plugin>.toml

# Re-register plugin
sudo op-dbus-stage deploy-plugin <plugin> --force
```

### Symlink Broken

```bash
# Check symlink
ls -la /usr/local/bin/op-dbus

# Recreate symlink
sudo op-dbus-stage fix-symlinks stage-1-core-dbus
```

### Rollback Failed

```bash
# Manual rollback
sudo btrfs subvolume delete /var/lib/op-dbus/staging/stage-X
sudo btrfs subvolume snapshot \
  /var/lib/op-dbus/staging/snapshots/pre-stage-X-TIMESTAMP \
  /var/lib/op-dbus/staging/stage-X
```

