# systemd Service Files

This directory contains systemd service files for Open vSwitch persistence and D-Bus introspection cache warmup.

## Files

### Open vSwitch Services
- **ovsdb-server.service** - OVSDB database server with persistent storage
- **ovs-vswitchd.service** - OVS switch daemon (depends on ovsdb-server)
- **openvswitch.service** - Meta-service for both components

### D-Bus Introspection Cache Services
- **dbus-cache-warmup.service** - Proactively caches D-Bus introspection data
- **dbus-cache-warmup.timer** - Schedules cache warmup on boot and daily

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

### Installing D-Bus Introspection Cache Warmup (Optional)

The cache warmup service proactively caches D-Bus introspection data for faster MCP queries:

```bash
# Install the warmup script
sudo cp scripts/warm-dbus-cache.sh /usr/local/bin/
sudo chmod +x /usr/local/bin/warm-dbus-cache.sh

# Install systemd service and timer
sudo cp systemd/dbus-cache-warmup.service /etc/systemd/system/
sudo cp systemd/dbus-cache-warmup.timer /etc/systemd/system/

# Create cache directory
sudo mkdir -p /var/cache
sudo mkdir -p /var/log

# Reload systemd configuration
sudo systemctl daemon-reload

# Enable and start the timer (runs on boot + daily at 3 AM)
sudo systemctl enable dbus-cache-warmup.timer
sudo systemctl start dbus-cache-warmup.timer

# Optionally, run warmup immediately
sudo systemctl start dbus-cache-warmup.service

# Verify timer is active
sudo systemctl list-timers dbus-cache-warmup.timer
```

**Benefits:**
- Reduces MCP tool query latency from 100-500ms to 1-5ms
- Caches introspection data for common services (systemd, NetworkManager, etc.)
- Runs automatically on boot and daily to keep cache fresh
- Uses only 50% CPU and 256MB RAM max (resource-limited)

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

### Verifying D-Bus Introspection Cache

After installing the cache warmup service:

```bash
# Check timer is active and scheduled
systemctl status dbus-cache-warmup.timer
sudo systemctl list-timers dbus-cache-warmup.timer

# View warmup service logs
sudo journalctl -u dbus-cache-warmup.service -n 50

# Check cache file exists and size
ls -lh /var/cache/dbus-introspection.db

# Query cache statistics (requires op-dbus with MCP feature)
# The cache should show entries for systemd1, NetworkManager, etc.
sqlite3 /var/cache/dbus-introspection.db "SELECT COUNT(*) FROM introspection_cache;"
sqlite3 /var/cache/dbus-introspection.db "SELECT service_name, COUNT(*) FROM service_methods GROUP BY service_name;"
```

Expected output:
- Timer should show next run time (e.g., "Next elapse: 03:00:00")
- Cache file should be 100KB-5MB depending on services cached
- Should see 10+ services cached (systemd, NetworkManager, login1, etc.)

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

### Cache warmup fails

```bash
# Check if D-Bus is running
systemctl status dbus.service

# Check warmup script logs
sudo journalctl -u dbus-cache-warmup.service -n 100

# Test warmup script manually
sudo bash -x /usr/local/bin/warm-dbus-cache.sh

# Check cache directory permissions
ls -ld /var/cache /var/log

# Verify services are accessible via D-Bus
dbus-send --system --print-reply --dest=org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 org.freedesktop.DBus.Introspectable.Introspect
```

Common issues:
- **Script not found:** Check `/usr/local/bin/warm-dbus-cache.sh` exists and is executable
- **Permission denied:** Ensure `/var/cache` and `/var/log` are writable
- **Services not available:** Some services may not be installed (this is normal, warmup continues with others)
- **Timeout errors:** Increase timeout in warmup script if needed (default: 5s per service)

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

### Open vSwitch Persistence
- [OVS-PERSISTENCE-SETUP.md](../docs/OVS-PERSISTENCE-SETUP.md) - Detailed setup guide
- [Network Plugin Source](../src/plugins/network.rs) - Implementation

### D-Bus Introspection Cache
- [INTROSPECTION-JSON-CACHE.md](../docs/INTROSPECTION-JSON-CACHE.md) - Cache architecture and design
- [Introspection Cache Source](../src/mcp/introspection_cache.rs) - Implementation
- [Cache Warmup Script](../scripts/warm-dbus-cache.sh) - Warmup script source
