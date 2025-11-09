# op-dbus Installation Guide

Complete installation guide for op-dbus - declarative system state management via native protocols.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Start](#quick-start)
3. [Deployment Modes](#deployment-modes)
4. [Step-by-Step Installation](#step-by-step-installation)
5. [Configuration](#configuration)
6. [Verification](#verification)
7. [Post-Installation](#post-installation)
8. [Troubleshooting](#troubleshooting)
9. [Advanced Topics](#advanced-topics)
10. [Uninstallation](#uninstallation)

---

## Prerequisites

### System Requirements

**Operating System:**
- Debian 11+ / Ubuntu 20.04+
- RHEL 8+ / CentOS 8+ / Fedora 35+ (community supported)
- Linux kernel 4.15+ (for OVS datapath)

**Hardware:**
- CPU: x86_64 (ARM64 experimental)
- RAM: Minimum 512MB, recommended 2GB+
- Disk: Minimum 1GB free space
- Optional: NUMA-capable CPU for cache optimization
- Optional: BTRFS filesystem for advanced caching

**Network:**
- Root/sudo access required
- Network interfaces for bridge creation
- Optional: Proxmox VE 7.0+ for container mode
- Optional: Netmaker for mesh networking

### Required Software

**Core Dependencies:**
- OpenVSwitch 2.13+ (CRITICAL - cannot run without this)
- systemd (for service management)
- D-Bus (for system integration)

**Build Dependencies:**
- Rust 1.70+ (latest stable recommended)
- Cargo (Rust package manager)
- pkg-config
- libssl-dev / openssl-devel
- build-essential / gcc + make

**Optional Dependencies:**
- netclient (Netmaker mesh networking)
- pct (Proxmox container management)
- jq (JSON processing in scripts)
- BTRFS tools (btrfs-progs)
- numactl (NUMA optimization)

### Platform-Specific Notes

**Debian/Ubuntu:**
```bash
# All dependencies can be installed via apt
sudo apt update
```

**RHEL/CentOS/Fedora:**
```bash
# Enable EPEL repository for additional packages
sudo yum install epel-release
```

**Proxmox:**
```bash
# Proxmox includes most dependencies by default
# OpenVSwitch may need to be installed separately
```

---

## Quick Start

### Three-Command Installation

For experienced users who want to install quickly:

```bash
# 1. Install system dependencies
sudo ./install-dependencies.sh

# 2. Build the binary
./build.sh

# 3. Install and configure
sudo ./install.sh --standalone
```

This installs op-dbus in standalone mode with OVS bridges but without container support.

---

## Deployment Modes

op-dbus supports three deployment modes to fit different use cases:

### 1. Full (Proxmox) Mode

**Use case:** Container-based deployments with mesh networking

**Includes:**
- D-Bus plugin system
- Blockchain audit logging
- OVS bridge management (ovsbr0 + mesh)
- LXC/Proxmox container integration
- Netmaker mesh networking support
- OpenFlow policy management

**Requirements:**
- Proxmox VE or LXC
- OpenVSwitch
- Optional: Netmaker

**Install command:**
```bash
sudo ./install.sh --full
```

### 2. Standalone Mode

**Use case:** Enterprise deployments without containers

**Includes:**
- D-Bus plugin system
- Blockchain audit logging
- OVS bridge management (ovsbr0)
- Network state management
- Service orchestration

**Requirements:**
- OpenVSwitch only

**Install command:**
```bash
sudo ./install.sh --standalone
```

### 3. Agent-Only Mode

**Use case:** Lightweight plugin-only deployments

**Includes:**
- D-Bus plugin system
- Minimal configuration
- Service management only
- No OVS bridges created
- Blockchain directories created but not used

**Requirements:**
- Minimal (systemd + D-Bus)

**Install command:**
```bash
sudo ./install.sh --agent-only
```

---

## Step-by-Step Installation

### Step 1: Install Dependencies

The `install-dependencies.sh` script handles all system prerequisites:

```bash
sudo ./install-dependencies.sh
```

**What it does:**
1. Detects your platform (Debian/Ubuntu/RHEL)
2. Installs OpenVSwitch (CRITICAL)
3. Installs build tools
4. Checks/installs Rust (offers to install via rustup)
5. Verifies OVS is working
6. Optionally installs netclient (Netmaker)

**Interactive prompts:**
- Install Rust if not found: `[Y/n]`
- Install Netmaker netclient: `[y/N]`

**Output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  op-dbus Dependency Installer
  Installing generic prerequisites...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📋 Detected platform: ubuntu 22.04
━━━ Installing Debian/Ubuntu packages ━━━
  ✅ openvswitch-switch (already installed)
  ✅ build-essential installed
  ...
✅ Dependency Installation Complete
```

### Step 2: Build op-dbus

Build the op-dbus binary with the desired features:

**Default build (includes web UI):**
```bash
./build.sh
```

**With MCP features:**
```bash
cargo build --release --features mcp
```

**With ML features:**
```bash
cargo build --release --features ml
```

**All features:**
```bash
cargo build --release --all-features
```

**Minimal (no extra features):**
```bash
cargo build --release --no-default-features
```

**Build time:** 2-10 minutes depending on features and system

**Output:**
```
Building op-dbus...
   Compiling op-dbus v0.1.0
    Finished release [optimized] target(s) in 3m 42s

✓ Build complete!

Binary: target/release/op-dbus
Size:   15M
```

### Step 3: Install op-dbus

Run the installation script in your chosen mode:

**Interactive mode (prompts for selection):**
```bash
sudo ./install.sh
```

**Command-line mode (specify mode):**
```bash
sudo ./install.sh --full         # Proxmox mode
sudo ./install.sh --standalone   # Standalone mode
sudo ./install.sh --agent-only   # Agent-only mode
```

**Installation phases:**

**Phase 0: Preflight Checks**
- Verifies root access
- Checks binary exists
- Verifies OpenVSwitch installed and running

**Phase 1: Mode Selection** (if not specified via flag)
```
Select deployment mode:

  [1] Full (Proxmox)
      D-Bus + Blockchain + LXC/Proxmox + Netmaker
      For container-based deployments with mesh networking

  [2] Standalone
      D-Bus + Blockchain (no containers)
      For enterprise deployments without containers

  [3] Agent Only
      D-Bus plugins only (minimal)
      For lightweight plugin-only deployments

Enter choice [1-3]:
```

**Phase 2: Binary Installation**
- Copies `op-dbus` to `/usr/local/bin/`
- Sets permissions (755)
- Verifies binary works
- TODO: Installs MCP binaries if built with MCP features

**Phase 3: Directory Structure**
Creates all required directories:
```
/etc/op-dbus/              # Configuration
/var/lib/op-dbus/          # Data storage
├── blockchain/            # Audit log
│   ├── timing/            # Footprint timestamps
│   ├── vectors/           # ML embeddings
│   └── snapshots/         # BTRFS snapshots
└── @cache/                # Cache storage (TODO: BTRFS subvolume)
/run/op-dbus/              # Runtime sockets
```

**Phase 4: State File Generation**

Option A - Introspection (recommended):
```
Use introspection to auto-detect system state? [Y/n]: Y
⏳ Running introspection...
✅ State file generated via introspection
```

This runs `op-dbus init --introspect` to detect your current system configuration and generate a realistic state.json.

Option B - Template:
```
Use introspection to auto-detect system state? [Y/n]: n
⏳ Generating template state file...
✅ Template state file created
```

Generates a minimal template based on the selected mode.

**Phase 5: Systemd Service**

Creates `/etc/systemd/system/op-dbus.service` with appropriate dependencies:

- **Full/Standalone mode:** Depends on `openvswitch-switch.service`
- **Agent-only mode:** Minimal dependencies

Service configuration includes:
- Security hardening (ProtectSystem, PrivateTmp)
- Network capabilities (CAP_NET_ADMIN, CAP_NET_RAW)
- TODO: NUMA CPU pinning

**Phase 6: Declarative State Application**

**THIS IS THE KEY FEATURE** - op-dbus installs itself declaratively!

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  DECLARATIVE STATE APPLICATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

This is where op-dbus installs itself declaratively!

📄 State file: /etc/op-dbus/state.json

━━━ State Preview ━━━
{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": [
        {
          "name": "ovsbr0",
          "type": "ovs-bridge",
          ...
        }
      ]
    },
    "systemd": {
      "units": {
        "openvswitch-switch.service": {
          "enabled": true,
          "active_state": "active"
        }
      }
    }
  }
}
━━━━━━━━━━━━━━━━━━━━

Apply this state now? [Y/n]:
```

If you choose "Y", it runs:
```bash
op-dbus apply /etc/op-dbus/state.json
```

This creates:
- OVS bridges (ovsbr0, mesh if full mode)
- Network configuration
- Service states
- All op-dbus-specific infrastructure

**Declaratively!**

**Phase 7: Service Management**

Enable and start the systemd service:

```
Enable op-dbus service to start at boot? [Y/n]: Y
⏳ Enabling op-dbus service...
✅ Service enabled

Start op-dbus service now? [y/N]: y
⏳ Starting op-dbus service...
✅ Service is running
```

**Phase 8: Summary**

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✅ INSTALLATION COMPLETE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Installation Summary:
  Mode:          standalone
  Binary:        /usr/local/bin/op-dbus
  Config:        /etc/op-dbus
  State file:    /etc/op-dbus/state.json
  Data:          /var/lib/op-dbus
  Service:       /etc/systemd/system/op-dbus.service
  Service:       enabled
  Status:        running ✅

Useful commands:
  Query state:         sudo op-dbus query
  Check differences:   sudo op-dbus diff /etc/op-dbus/state.json
  Apply state:         sudo op-dbus apply /etc/op-dbus/state.json
  Service status:      sudo systemctl status op-dbus
  View logs:           sudo journalctl -fu op-dbus
  Run diagnostics:     sudo op-dbus doctor

Next steps:
  1. Review state file: /etc/op-dbus/state.json
  2. Verify installation: sudo ./verify-installation.sh

Documentation: README.md, ENTERPRISE-DEPLOYMENT.md
```

### Step 4: Verify Installation

Run the verification script to ensure everything is working:

```bash
sudo ./verify-installation.sh
```

This performs 10 comprehensive checks:
1. Binary installation
2. Directory structure
3. System dependencies
4. Systemd service
5. OVS bridges
6. D-Bus access
7. Blockchain storage
8. Command tests
9. Network connectivity
10. Netmaker (if configured)

**Expected output:**
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  op-dbus Installation Verification
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  1. Binary Installation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ PASS: op-dbus binary exists: /usr/local/bin/op-dbus
✅ PASS: Binary is executable
✅ PASS: Binary runs: op-dbus 0.1.0
...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  VERIFICATION SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Results:
  Passed:   45
  Failed:   0
  Warnings: 2
  Total:    47

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ✅ INSTALLATION VERIFIED SUCCESSFULLY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Configuration

### State File Format

The `/etc/op-dbus/state.json` file defines your system's desired state.

**Basic structure:**
```json
{
  "version": 1,
  "plugins": {
    "net": { ... },
    "systemd": { ... },
    "lxc": { ... }
  }
}
```

### Network Plugin (net)

Defines OVS bridges, IP addresses, routes:

```json
"net": {
  "interfaces": [
    {
      "name": "ovsbr0",
      "type": "ovs-bridge",
      "ports": ["eth0"],
      "ipv4": {
        "enabled": true,
        "dhcp": false,
        "address": [
          {
            "ip": "192.168.1.10",
            "prefix": 24
          }
        ],
        "gateway": "192.168.1.1"
      }
    }
  ]
}
```

### Systemd Plugin

Defines service states:

```json
"systemd": {
  "units": {
    "openvswitch-switch.service": {
      "enabled": true,
      "active_state": "active"
    },
    "my-custom.service": {
      "enabled": false,
      "active_state": "inactive"
    }
  }
}
```

### LXC Plugin (Full mode only)

Defines containers:

```json
"lxc": {
  "containers": [
    {
      "id": "101",
      "veth": "vi101",
      "bridge": "mesh",
      "running": true,
      "properties": {
        "network_type": "socket"
      }
    }
  ]
}
```

### Editing State

1. Edit the state file:
```bash
sudo nano /etc/op-dbus/state.json
```

2. Check what will change:
```bash
sudo op-dbus diff /etc/op-dbus/state.json
```

3. Apply the changes:
```bash
sudo op-dbus apply /etc/op-dbus/state.json
```

4. Verify:
```bash
sudo op-dbus query
```

---

## Verification

### Manual Verification Commands

**Check binary:**
```bash
which op-dbus
op-dbus --version
```

**Check service:**
```bash
systemctl status op-dbus.service
systemctl is-enabled op-dbus.service
```

**Check OVS bridges:**
```bash
ovs-vsctl list-br
ovs-vsctl show
ip link show ovsbr0  # should be visible in kernel
```

**Check configuration:**
```bash
cat /etc/op-dbus/state.json | jq .
```

**Check blockchain:**
```bash
ls -l /var/lib/op-dbus/blockchain/timing/
```

**Test commands:**
```bash
sudo op-dbus doctor
sudo op-dbus query
sudo op-dbus introspect --pretty
```

---

## Post-Installation

### Common Tasks

**View current state:**
```bash
sudo op-dbus query
sudo op-dbus query --plugin net      # Specific plugin
sudo op-dbus query --plugin systemd
```

**Modify configuration:**
```bash
sudo nano /etc/op-dbus/state.json
sudo op-dbus diff /etc/op-dbus/state.json
sudo op-dbus apply /etc/op-dbus/state.json
```

**Monitor logs:**
```bash
sudo journalctl -fu op-dbus         # Follow logs
sudo journalctl -u op-dbus -n 50    # Last 50 lines
```

**View blockchain audit:**
```bash
op-dbus blockchain list
op-dbus blockchain search "bridge"
op-dbus verify
```

**Introspect databases:**
```bash
op-dbus introspect --pretty
op-dbus introspect --database ovsdb
op-dbus introspect --database nonnet
```

### Netmaker Setup (Optional)

If you want mesh networking for containers:

1. Install netclient (if not done during dependency installation):
```bash
curl -sL https://apt.netmaker.org/gpg.key | sudo apt-key add -
curl -sL https://apt.netmaker.org/debian.deb.txt | sudo tee /etc/apt/sources.list.d/netmaker.list
sudo apt update && sudo apt install netclient
```

2. Configure enrollment token:
```bash
echo "NETMAKER_TOKEN=your-token-here" | sudo tee /etc/op-dbus/netmaker.env
```

3. Join the host to Netmaker:
```bash
sudo netclient join -t "your-token-here"
```

4. Sync Netmaker interfaces to mesh bridge:
```bash
sudo ./sync-netmaker-mesh.sh
```

### Container Management (Full mode)

**List containers:**
```bash
sudo op-dbus container list
sudo op-dbus container list --running
```

**Create container:**
```bash
sudo op-dbus container create 101 --network-type socket
```

**Start/stop:**
```bash
sudo op-dbus container start 101
sudo op-dbus container stop 101
```

**View details:**
```bash
sudo op-dbus container show 101
```

---

## Troubleshooting

### Installation Issues

**Problem: "ovs-vsctl not found"**
```
Solution: Install dependencies first
sudo ./install-dependencies.sh
```

**Problem: "Binary not found: target/release/op-dbus"**
```
Solution: Build the binary first
./build.sh
```

**Problem: "OpenVSwitch is not running"**
```
Solution: Start OVS services
sudo systemctl start openvswitch-switch
sudo systemctl status openvswitch-switch
```

**Problem: "Permission denied"**
```
Solution: Run with sudo
sudo ./install.sh
```

### Runtime Issues

**Problem: Service fails to start**
```
Check logs:
sudo journalctl -xeu op-dbus.service

Common causes:
- State file syntax error (check with: jq . /etc/op-dbus/state.json)
- OVS not running
- Permission issues
```

**Problem: Bridges not created**
```
Check if state was applied:
sudo op-dbus diff /etc/op-dbus/state.json

Apply if needed:
sudo op-dbus apply /etc/op-dbus/state.json

Check OVS:
sudo ovs-vsctl show
```

**Problem: Commands fail with "Permission denied"**
```
Many commands require root:
sudo op-dbus query
sudo op-dbus apply state.json
```

### Diagnostic Commands

**Run system diagnostics:**
```bash
sudo op-dbus doctor
```

**Test OVSDB connectivity:**
```bash
sudo ovs-vsctl show
```

**Test D-Bus connectivity:**
```bash
busctl status org.freedesktop.systemd1
```

**Verify blockchain integrity:**
```bash
op-dbus verify --full
```

---

## Advanced Topics

### BTRFS Cache Optimization (TODO)

Future enhancement for high-performance caching:

```bash
# Create BTRFS filesystem for cache
sudo mkfs.btrfs /dev/sdX
sudo mount /dev/sdX /var/lib/op-dbus/@cache

# Convert existing cache to BTRFS subvolume
sudo btrfs subvolume create /var/lib/op-dbus/@cache
```

### NUMA CPU Pinning (TODO)

Future enhancement for NUMA-aware performance:

```bash
# Detect NUMA topology
numactl --hardware

# Configure in systemd service
# CPUAffinity=0-3
# NUMAPolicy=bind
# NUMAMask=0
```

### MCP Component Installation (TODO)

When built with `--features mcp`, install MCP binaries:

```bash
# Copy MCP binaries
sudo cp target/release/dbus-mcp /usr/local/bin/
sudo cp target/release/dbus-orchestrator /usr/local/bin/
sudo cp target/release/dbus-mcp-web /usr/local/bin/
sudo cp target/release/mcp-chat /usr/local/bin/

# Create MCP configuration
sudo mkdir -p /etc/op-dbus/agents
```

### Custom Plugin Development

Create custom plugins by implementing the `StatePlugin` trait:

```rust
#[async_trait]
pub trait StatePlugin: Send + Sync {
    fn name(&self) -> &str;
    async fn query_current_state(&self) -> Result<Value>;
    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff>;
    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult>;
    ...
}
```

---

## Uninstallation

### Remove op-dbus

Run the uninstall script:

```bash
sudo ./uninstall.sh
```

The script will:
1. Prompt for confirmation
2. Stop and disable the service
3. Remove binaries from `/usr/local/bin/`
4. Remove service file
5. Optionally remove OVS bridges (prompts)
6. Optionally remove data directories (prompts)

### Manual Uninstallation

If the uninstall script is not available:

```bash
# Stop and disable service
sudo systemctl stop op-dbus.service
sudo systemctl disable op-dbus.service

# Remove binaries
sudo rm /usr/local/bin/op-dbus
sudo rm /usr/local/bin/dbus-*  # MCP binaries if installed

# Remove service file
sudo rm /etc/systemd/system/op-dbus.service
sudo systemctl daemon-reload

# Remove OVS bridges (optional)
sudo ovs-vsctl del-br ovsbr0
sudo ovs-vsctl del-br mesh

# Remove data (optional, CAUTION: deletes blockchain!)
sudo rm -rf /etc/op-dbus
sudo rm -rf /var/lib/op-dbus
sudo rm -rf /run/op-dbus
```

### Reinstallation

After uninstallation, you can reinstall cleanly:

```bash
sudo ./install.sh --standalone
```

---

## Support & Documentation

**Documentation:**
- README.md - Project overview
- AGENTS.md - Coding guidelines
- ENTERPRISE-DEPLOYMENT.md - Enterprise deployment guide
- MCP-INTEGRATION.md - MCP feature documentation
- TESTING-PLAN.md - Testing strategy

**Logs:**
```bash
sudo journalctl -u op-dbus.service
```

**Diagnostics:**
```bash
sudo op-dbus doctor
```

**Community:**
- GitHub Issues: https://github.com/ghostbridge/op-dbus/issues
- Documentation: README.md and related docs

---

## Quick Reference

**Installation:**
```bash
sudo ./install-dependencies.sh
./build.sh
sudo ./install.sh --standalone
sudo ./verify-installation.sh
```

**Daily Usage:**
```bash
sudo op-dbus query              # View current state
sudo op-dbus diff state.json    # Check changes
sudo op-dbus apply state.json   # Apply state
sudo op-dbus doctor             # Run diagnostics
```

**Service Management:**
```bash
sudo systemctl status op-dbus
sudo systemctl restart op-dbus
sudo journalctl -fu op-dbus
```

**Uninstall:**
```bash
sudo ./uninstall.sh
```

---

**Last Updated:** 2025-11-08
**Version:** 0.1.0
