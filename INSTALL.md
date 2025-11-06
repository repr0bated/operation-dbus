# Installation Guide

Complete guide for installing op-dbus on any Linux system.

## Prerequisites

### Required

- **Linux** with kernel 4.4+
- **systemd** (most modern Linux distributions)
- **D-Bus** system bus
- **Rust** 1.70+ (for building from source)

Check if you have the requirements:

```bash
# Check systemd
systemctl --version

# Check D-Bus
ls -la /var/run/dbus/system_bus_socket

# Check Rust
rustc --version
cargo --version
```

### Optional Components

op-dbus will automatically detect and use these if available:

- **OpenVSwitch** (`ovs-vsctl`) - For network management
- **Proxmox VE** (`pct`) - For container orchestration

## Installation Methods

### Method 1: Portable Install Script (Recommended)

The portable installer works on **any** Linux system with systemd.

```bash
# 1. Clone the repository
git clone https://github.com/repr0bated/operation-dbus.git
cd operation-dbus

# 2. Build
cargo build --release

# 3. Install
sudo ./install-portable.sh
```

**What this does:**
1. Checks for systemd and D-Bus (required)
2. Detects optional components (OVS, Proxmox)
3. Installs binary to `/usr/local/bin/op-dbus`
4. Introspects your system and creates `/etc/op-dbus/state.json`
5. Creates directories for blockchain storage
6. Sets up systemd service (disabled by default)

### Method 2: Manual Installation

If you want more control:

```bash
# 1. Build
cargo build --release

# 2. Install binary
sudo install -m 755 target/release/op-dbus /usr/local/bin/op-dbus

# 3. Create directories
sudo mkdir -p /etc/op-dbus
sudo mkdir -p /var/lib/op-dbus/blockchain/{timing,vectors,snapshots}
sudo mkdir -p /run/op-dbus

# 4. Generate initial state
/usr/local/bin/op-dbus init --introspect --output /etc/op-dbus/state.json

# 5. Create systemd service (optional)
sudo cat > /etc/systemd/system/op-dbus.service <<'EOF'
[Unit]
Description=op-dbus - Declarative system state management
After=network-online.target dbus.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/op-dbus run --state-file /etc/op-dbus/state.json
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

NoNewPrivileges=false
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/op-dbus /run/op-dbus
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
```

## Post-Installation

### 1. Verify Installation

```bash
# Check binary
op-dbus --version

# Run diagnostics
op-dbus doctor

# View detected plugins
op-dbus query | jq '.plugins | keys'
```

Expected output with all components:
```
INFO Discovering available plugins...
INFO ✓ Registering plugin: systemd
INFO ✓ Registering plugin: login1
INFO ✓ Registering plugin: net (OpenVSwitch)
INFO ✓ Registering plugin: lxc (Proxmox)
```

Expected output on minimal system:
```
INFO Discovering available plugins...
INFO ✓ Registering plugin: systemd
INFO ✓ Registering plugin: login1
INFO ⊗ Skipping plugin: net - OpenVSwitch (ovs-vsctl) not found
INFO ⊗ Skipping plugin: lxc - Proxmox pct command not found
```

### 2. Test Commands

```bash
# Query current state
op-dbus query

# View specific plugin
op-dbus query --plugin systemd

# Inspect system databases
op-dbus introspect --pretty
```

### 3. Make Your First Change

```bash
# 1. View current state
op-dbus query --plugin systemd | jq '.units | keys | .[0:5]'

# 2. Edit state file
sudo nano /etc/op-dbus/state.json

# 3. Preview changes
op-dbus diff /etc/op-dbus/state.json

# 4. Apply (this creates first blockchain block)
sudo op-dbus apply /etc/op-dbus/state.json

# 5. Verify
op-dbus blockchain list
```

### 4. Enable Automatic State Management (Optional)

```bash
# Enable service
sudo systemctl enable op-dbus

# Start service
sudo systemctl start op-dbus

# Check status
sudo systemctl status op-dbus

# View logs
sudo journalctl -u op-dbus -f
```

## Installing Optional Components

### OpenVSwitch (Network Management)

**Debian/Ubuntu:**
```bash
sudo apt update
sudo apt install openvswitch-switch
sudo systemctl start openvswitch-switch
sudo systemctl enable openvswitch-switch
```

**RHEL/CentOS:**
```bash
sudo yum install openvswitch
sudo systemctl start openvswitch
sudo systemctl enable openvswitch
```

After installing OVS, restart op-dbus to detect it:
```bash
sudo systemctl restart op-dbus
```

### Proxmox VE (Container Management)

Proxmox VE is a complete virtualization platform. See https://www.proxmox.com/en/proxmox-ve

op-dbus will automatically detect if you're running on Proxmox.

## Platform-Specific Notes

### Debian/Ubuntu

```bash
# Install dependencies
sudo apt update
sudo apt install build-essential pkg-config libdbus-1-dev

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install op-dbus
cd operation-dbus
cargo build --release
sudo ./install-portable.sh
```

### RHEL/CentOS/Fedora

```bash
# Install dependencies
sudo yum install gcc dbus-devel

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install op-dbus
cd operation-dbus
cargo build --release
sudo ./install-portable.sh
```

### Arch Linux

```bash
# Install dependencies
sudo pacman -S base-devel rust dbus

# Install op-dbus
cd operation-dbus
cargo build --release
sudo ./install-portable.sh
```

## Troubleshooting

### "OVSDB Connection Failed"

**Cause:** OpenVSwitch not running or not installed

**Solution:**
```bash
# Check if OVS is installed
which ovs-vsctl

# If not installed, see "Installing Optional Components" above

# If installed but not running
sudo systemctl start openvswitch-switch
sudo systemctl status openvswitch-switch
```

op-dbus will work fine without OVS - it just won't have network management capabilities.

### "Permission Denied" on D-Bus

**Cause:** Insufficient permissions

**Solution:**
```bash
# Run as root
sudo op-dbus query

# Or check D-Bus socket permissions
ls -la /var/run/dbus/system_bus_socket
```

### "Blockchain Directory Not Found"

**Cause:** Data directory not created

**Solution:**
```bash
sudo mkdir -p /var/lib/op-dbus/blockchain/{timing,vectors,snapshots}
sudo chmod 700 /var/lib/op-dbus
```

### Build Fails with Network Error

**Cause:** Cargo can't reach crates.io

**Solution:**
```bash
# Configure cargo to use a mirror or proxy
# Or build offline if dependencies are cached
cargo build --release --offline
```

## Upgrading

```bash
# 1. Pull latest changes
cd operation-dbus
git pull

# 2. Rebuild
cargo build --release

# 3. Stop service (if running)
sudo systemctl stop op-dbus

# 4. Reinstall
sudo ./install-portable.sh

# 5. Restart service
sudo systemctl start op-dbus
```

Your configuration and blockchain are preserved during upgrades.

## Uninstallation

```bash
# 1. Stop and disable service
sudo systemctl stop op-dbus
sudo systemctl disable op-dbus

# 2. Remove binary
sudo rm /usr/local/bin/op-dbus

# 3. Remove systemd service
sudo rm /etc/systemd/system/op-dbus.service
sudo systemctl daemon-reload

# 4. Optionally remove data (⚠️ deletes blockchain)
sudo rm -rf /etc/op-dbus
sudo rm -rf /var/lib/op-dbus
sudo rm -rf /run/op-dbus
```

## Directory Structure

After installation:

```
/usr/local/bin/
  └── op-dbus                 # Main binary

/etc/op-dbus/
  └── state.json              # Desired system state (user-editable)

/var/lib/op-dbus/
  └── blockchain/
      ├── timing/             # Blockchain blocks (timestamps, hashes)
      ├── vectors/            # ML embeddings (optional)
      └── snapshots/          # BTRFS snapshots (optional)

/run/op-dbus/
  └── nonnet.db.sock          # JSON-RPC socket for plugin queries

/etc/systemd/system/
  └── op-dbus.service         # Systemd service definition
```

## Next Steps

- Read [QUICKSTART.md](QUICKSTART.md) for a guided tutorial
- Read [README.md](README.md) for usage examples
- Check [PLUGIN-DEVELOPMENT-GUIDE.md](PLUGIN-DEVELOPMENT-GUIDE.md) to create custom plugins
- Join the community (GitHub Discussions)

## Support

- **Issues:** https://github.com/repr0bated/operation-dbus/issues
- **Documentation:** https://github.com/repr0bated/operation-dbus/tree/master/docs
- **Discussions:** https://github.com/repr0bated/operation-dbus/discussions
