# systemd Service Files for OVS Persistence

This directory contains systemd service files for Open vSwitch with proper OVSDB persistence configuration.

## Files

- **ovsdb-server.service** - OVSDB database server with persistent storage
- **ovs-vswitchd.service** - OVS switch daemon (depends on ovsdb-server)
- **openvswitch.service** - Meta-service for both components

## Installation

### Manual Installation

```bash
# Copy service files to systemd directory
sudo cp systemd/ovsdb-server.service /etc/systemd/system/
sudo cp systemd/ovs-vswitchd.service /etc/systemd/system/
sudo cp systemd/openvswitch.service /etc/systemd/system/

# Create openvswitch user if it doesn't exist
sudo useradd -r -s /usr/sbin/nologin -d /var/lib/openvswitch openvswitch

# Create required directories
sudo mkdir -p /etc/openvswitch
sudo mkdir -p /var/run/openvswitch
sudo mkdir -p /var/log/openvswitch

# Set proper ownership
sudo chown -R openvswitch:openvswitch /etc/openvswitch
sudo chown -R openvswitch:openvswitch /var/run/openvswitch
sudo chown -R openvswitch:openvswitch /var/log/openvswitch

# Reload systemd configuration
sudo systemctl daemon-reload

# Enable services
sudo systemctl enable ovsdb-server.service
sudo systemctl enable ovs-vswitchd.service
sudo systemctl enable openvswitch.service

# Start services
sudo systemctl start openvswitch.service

# Verify services are running
sudo systemctl status ovsdb-server.service
sudo systemctl status ovs-vswitchd.service
```

### Automated Installation via Bootstrap

The `tools/bootstrap-minimal.sh` script can automatically install these services:

```bash
sudo ./tools/bootstrap-minimal.sh /dev/sda examples/complete-proxmox-install.json
```

This will:
1. Create minimal Debian base system
2. Install OVS packages
3. Install systemd service files
4. Configure OVSDB persistence
5. Set up first-boot `op-dbus apply` service

## Verification

After installation, verify OVSDB persistence is working:

```bash
# Check services are running
systemctl status openvswitch.service

# Check database file exists
ls -lh /etc/openvswitch/conf.db

# Create test bridge via op-dbus
cat > /tmp/test-bridge.json <<EOF
{
  "version": 1,
  "plugins": {
    "network": {
      "bridges": [
        {
          "name": "test-br0",
          "datapath_type": "system"
        }
      ]
    }
  }
}
EOF

sudo op-dbus apply /tmp/test-bridge.json

# Verify bridge exists
sudo ovs-vsctl show

# Check database file was modified (mtime should be recent)
ls -lh /etc/openvswitch/conf.db

# REBOOT to test persistence
sudo reboot

# After reboot, verify bridge still exists
sudo ovs-vsctl show

# If test-br0 still exists, persistence is working! ✓
```

## Troubleshooting

### Service won't start

```bash
# Check logs
journalctl -u ovsdb-server.service -n 50
journalctl -u ovs-vswitchd.service -n 50

# Check file permissions
ls -lh /etc/openvswitch/
ls -lh /var/run/openvswitch/

# Fix permissions if needed
sudo chown -R openvswitch:openvswitch /etc/openvswitch
sudo chown -R openvswitch:openvswitch /var/run/openvswitch
```

### Database file not persisting

```bash
# Check database file location in service
systemctl cat ovsdb-server.service | grep conf.db

# Should show: /etc/openvswitch/conf.db

# Check if directory is tmpfs (bad, will wipe on reboot)
df -h /etc/openvswitch

# Should NOT be tmpfs
```

### Bridges disappear after reboot

```bash
# Check datapath_type
sudo ovs-vsctl get Bridge vmbr0 datapath_type

# Should be: "system"
# If "netdev", recreate with datapath_type="system" in state.json
```

## Service Dependency Order

```
ovsdb-server.service
  ↓
ovs-vswitchd.service
  ↓
network.target
  ↓
op-dbus apply state.json (creates bridges)
```

## See Also

- [OVS-PERSISTENCE-SETUP.md](../docs/OVS-PERSISTENCE-SETUP.md) - Detailed setup guide
- [Network Plugin Source](../src/plugins/network.rs) - Implementation
