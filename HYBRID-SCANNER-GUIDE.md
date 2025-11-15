# Hybrid System Scanner Guide

## Overview

The Hybrid Scanner provides comprehensive system introspection by combining:
1. **D-Bus services** - systemd, NetworkManager, PackageKit, etc.
2. **Filesystem resources** - /dev, /proc, /sys, /run
3. **Process information** - Running services and applications
4. **Hardware devices** - PCI, block, network devices
5. **Network interfaces** - Physical and virtual interfaces
6. **Configuration files** - System configurations

All resources are exposed via a unified D-Bus interface at `org.opdbus.HybridSystem`.

## Architecture

```
┌──────────────────────────────────────────────┐
│  AI Assistant / MCP Client                   │
└────────────────┬─────────────────────────────┘
                 │
                 ↓ D-Bus
┌──────────────────────────────────────────────┐
│  Hybrid D-Bus Bridge                         │
│  org.opdbus.HybridSystem                     │
│  - Exposes all resources via D-Bus           │
└────────────────┬─────────────────────────────┘
                 │
                 ↓
┌──────────────────────────────────────────────┐
│  Hybrid Scanner                              │
│  - Orchestrates all discovery mechanisms     │
└────────────────┬─────────────────────────────┘
                 │
    ┌────────────┼────────────┬────────────┐
    ↓            ↓            ↓            ↓
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│ D-Bus  │  │ /proc  │  │ /sys   │  │ /etc   │
│ Svcs   │  │ /run   │  │ /dev   │  │ Config │
└────────┘  └────────┘  └────────┘  └────────┘
```

## Features

### 1. D-Bus Service Discovery

Automatically discovers and introspects all D-Bus services:

```bash
op-dbus introspect-all
```

Output includes:
- Service names (org.freedesktop.systemd1, etc.)
- Object paths
- Interfaces with methods, properties, signals
- Method signatures and parameter types

### 2. Filesystem Scanning

Scans important system directories:

- `/dev` - Device nodes
- `/proc` - Process information
- `/sys` - Hardware/kernel interface
- `/run` - Runtime data
- `/etc` - Configuration files

### 3. Process Discovery

Extracts running process information:

- PID, name, command line
- User/owner
- Memory usage
- Status (running, sleeping, etc.)

### 4. Hardware Detection

Discovers hardware devices from `/sys`:

- **PCI devices**: Graphics cards, network cards, etc.
- **Block devices**: Disks (sda, nvme0n1, etc.)
- **Network devices**: Ethernet, WiFi interfaces

### 5. Network Interface Scanning

Enumerates network interfaces:

- Interface names (eth0, wlan0, br0, etc.)
- MAC addresses
- IP addresses (via netlink)
- Status (up/down)
- MTU and type

### 6. Configuration File Tracking

Tracks system configuration files:

- systemd unit files
- NetworkManager configs
- D-Bus policies
- Format detection (JSON, YAML, INI, etc.)

## Usage

### CLI Tool

```bash
# Scan everything
op-dbus hybrid-scan

# List D-Bus services only
op-dbus hybrid-scan --dbus-only

# List processes
op-dbus hybrid-scan --processes

# List hardware
op-dbus hybrid-scan --hardware

# List network interfaces
op-dbus hybrid-scan --network

# Output to file
op-dbus hybrid-scan --output /tmp/system-scan.json
```

### D-Bus Interface

Start the bridge service:

```bash
# Start as systemd service (NixOS)
sudo systemctl start op-dbus-hybrid-scanner

# Or run manually
op-dbus hybrid-bridge start
```

Call methods via D-Bus:

```bash
# Scan all resources
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem scan_all

# List processes
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem list_processes

# Get process by PID
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem get_process u 1234

# List hardware
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem list_hardware

# List hardware by type (pci, block, network)
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem list_hardware_by_type s "pci"

# List network interfaces
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem list_network_interfaces

# Get interface by name
busctl call org.opdbus.HybridSystem /org/opdbus.HybridSystem \
  org.opdbus.HybridSystem get_network_interface s "eth0"

# Get stats summary
busctl call org.opdbus.HybridSystem /org/opdbus/HybridSystem \
  org.opdbus.HybridSystem get_stats_summary
```

### Rust API

```rust
use op_dbus::mcp::hybrid_scanner::HybridScanner;

#[tokio::main]
async fn main() -> Result<()> {
    let scanner = HybridScanner::new().await?;

    // Scan everything
    let result = scanner.scan_all().await?;

    println!("Found:");
    println!("  {} D-Bus services", result.dbus_services.len());
    println!("  {} processes", result.processes.len());
    println!("  {} hardware devices", result.hardware.len());
    println!("  {} network interfaces", result.network_interfaces.len());

    Ok(())
}
```

## Output Format

### Scan Result

```json
{
  "dbus_services": [
    {
      "service_name": "org.freedesktop.systemd1",
      "object_paths": ["/org/freedesktop/systemd1"],
      "interfaces": {
        "/org/freedesktop/systemd1": [
          {
            "name": "org.freedesktop.systemd1.Manager",
            "methods": [
              {
                "name": "StartUnit",
                "inputs": [
                  { "name": "name", "type_sig": "s", "type_name": "string" },
                  { "name": "mode", "type_sig": "s", "type_name": "string" }
                ],
                "outputs": [
                  { "name": "job", "type_sig": "o", "type_name": "object_path" }
                ]
              }
            ],
            "properties": [],
            "signals": []
          }
        ]
      }
    }
  ],
  "processes": [
    {
      "pid": 1,
      "name": "systemd",
      "cmdline": "/sbin/init",
      "user": "0",
      "status": "S",
      "memory_kb": 15420,
      "cpu_percent": 0.0
    }
  ],
  "hardware": [
    {
      "device_type": "pci",
      "name": "0000:00:1f.2",
      "path": "/sys/bus/pci/devices/0000:00:1f.2",
      "vendor": "0x8086",
      "model": "0x2829",
      "driver": "ahci",
      "attributes": {
        "class": "0x010601"
      }
    }
  ],
  "network_interfaces": [
    {
      "name": "eth0",
      "mac_address": "00:1a:2b:3c:4d:5e",
      "ip_addresses": ["192.168.1.100"],
      "status": "up",
      "mtu": 1500,
      "type_": "ethernet"
    }
  ],
  "filesystem_resources": [
    {
      "path": "/dev/sda",
      "resource_type": "block",
      "permissions": "0660",
      "owner": "0:6",
      "size": null,
      "metadata": {}
    }
  ],
  "system_config": [
    {
      "path": "/etc/systemd/system/myservice.service",
      "service": "myservice",
      "format": "ini",
      "last_modified": 1699564800,
      "size": 1024
    }
  ],
  "timestamp": 1699564800
}
```

## MCP Integration

The hybrid scanner generates MCP tools automatically from discovered resources.

### Auto-Generated Tools

For each D-Bus method discovered:

```json
{
  "name": "org_freedesktop_systemd1__startunit",
  "description": "org.freedesktop.systemd1.Manager.StartUnit on org.freedesktop.systemd1",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": { "type": "string" },
      "mode": { "type": "string" }
    },
    "required": ["name", "mode"]
  }
}
```

### Usage from AI Assistants

AI assistants can now:

1. **Discover available tools**: Query hybrid scanner to see what's available
2. **Introspect services**: Get detailed information about any D-Bus service
3. **Manage system**: Call methods on discovered services
4. **Monitor resources**: Track processes, hardware, network interfaces

## Use Cases

### 1. System Audit

```bash
op-dbus hybrid-scan --output system-audit.json
```

Generates complete system inventory:
- All D-Bus services
- Running processes
- Hardware devices
- Network configuration
- Installed packages (via PackageKit)

### 2. Security Scanning

Find listening services:

```bash
op-dbus hybrid-scan --processes | jq '.processes[] | select(.cmdline | contains("listen"))'
```

### 3. Configuration Management

Track all configuration files:

```bash
op-dbus hybrid-scan --config-files
```

### 4. Hardware Inventory

```bash
op-dbus hybrid-scan --hardware | jq '.hardware[] | select(.device_type == "pci")'
```

### 5. Network Topology

```bash
op-dbus hybrid-scan --network
```

## NixOS Integration

The NixOS module automatically enables the hybrid scanner:

```nix
services.op-dbus = {
  enable = true;
  mcp.hybridScanner = true;
};
```

This creates the `op-dbus-hybrid-scanner` systemd service.

## Performance

### Scan Times

Typical scan times on a modern system:

- D-Bus services: 2-5 seconds
- Processes: 0.5-1 second
- Hardware: 0.2-0.5 seconds
- Network: 0.1-0.2 seconds
- Full scan: 3-7 seconds

### Optimizations

1. **Concurrent scanning**: Different resource types scanned in parallel
2. **Caching**: Results cached for 60 seconds
3. **Lazy loading**: Only scan requested resource types
4. **Limits**: Configurable limits to prevent overwhelming system

## Security Considerations

### Permissions

The hybrid scanner requires:

- **D-Bus**: Read access to system bus
- **Filesystem**: Read access to /proc, /sys, /dev
- **Process info**: Read access to all process information

### Sandboxing

NixOS configuration restricts access:

```nix
serviceConfig = {
  ProtectSystem = "strict";
  ProtectHome = true;
  ReadOnlyPaths = ["/proc" "/sys" "/dev"];
};
```

### Data Exposure

Be careful when exposing scan results:

- May contain sensitive process information
- Shows system topology
- Reveals installed software

Restrict D-Bus access with PolicyKit.

## Troubleshooting

### "Permission denied" accessing /proc

Ensure the service runs with appropriate permissions:

```nix
serviceConfig.User = "root";
```

Or use capabilities:

```nix
AmbientCapabilities = ["CAP_SYS_PTRACE"];
```

### Scan times out

Increase timeout or reduce scope:

```bash
op-dbus hybrid-scan --timeout 30 --limit 100
```

### D-Bus connection failed

Ensure D-Bus is running:

```bash
systemctl status dbus
```

## Future Enhancements

- [ ] Real-time monitoring (watch for changes)
- [ ] Filtering and queries (SQL-like)
- [ ] Historical tracking (diff over time)
- [ ] Alert on changes
- [ ] Integration with monitoring systems (Prometheus, etc.)
- [ ] Cloud resource discovery (AWS, Azure, GCP)

## References

- [PACKAGEKIT-PLUGIN-GUIDE.md](./PACKAGEKIT-PLUGIN-GUIDE.md) - Package management
- [MCP-DBUS-BENEFITS.md](./MCP-DBUS-BENEFITS.md) - MCP integration
- [nixos/README.md](./nixos/README.md) - NixOS deployment

## See Also

- `/proc` filesystem documentation
- `/sys` filesystem documentation
- D-Bus specification
- PackageKit API reference
