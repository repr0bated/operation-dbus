# D-Bus Native Deployment Guide

op-dbus is designed to work **entirely through D-Bus APIs** - no CLI commands needed!

## Philosophy

Instead of:
```bash
lxc-attach -n 100  # ❌ CLI shell access
pct create 100     # ❌ CLI command
```

We use:
```bash
busctl introspect org.freedesktop.systemd1                    # ✅ D-Bus introspection
busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 org.freedesktop.systemd1.Manager StartUnit ss "pve-container@100.service" "replace"  # ✅ D-Bus method call
```

## Architecture

```
NixOS configuration.nix (declarative state)
         ↓
   /etc/op-dbus/state.json
         ↓
   op-dbus daemon reads JSON
         ↓
   Plugin system processes state
         ↓
   ┌─────────┬──────────┬──────────┐
   ↓         ↓          ↓          ↓
systemd   OVSDB    D-Bus      PackageKit
 D-Bus    JSON-RPC  APIs       D-Bus
```

## Inspecting System State via D-Bus

### Check op-dbus Status

```bash
# Via systemd D-Bus
busctl status org.opdbus

# Get op-dbus service unit
busctl call org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  GetUnit s "op-dbus.service"

# Get unit properties
busctl get-property org.freedesktop.systemd1 \
  /org/freedesktop/systemd1/unit/op_2ddbus_2eservice \
  org.freedesktop.systemd1.Unit \
  ActiveState
```

### Check Container Units via D-Bus

```bash
# List all container units
busctl call org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  ListUnits | grep container

# Check container 100 status
busctl get-property org.freedesktop.systemd1 \
  /org/freedesktop/systemd1/unit/pve_2dcontainer_40100_2eservice \
  org.freedesktop.systemd1.Unit \
  ActiveState

# Start container via D-Bus
busctl call org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  StartUnit ss "pve-container@100.service" "replace"

# Stop container via D-Bus
busctl call org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  StopUnit ss "pve-container@100.service" "replace"
```

### Check OVS via Native OVSDB

```bash
# op-dbus uses native OVSDB JSON-RPC, not ovs-vsctl CLI!

# List databases (via D-Bus if available, or direct JSON-RPC)
# op-dbus does this internally via OvsdbClient

# View current OVSDB state as op-dbus sees it
journalctl -u op-dbus | grep -i "ovsdb\|bridge\|port"
```

### Check Packages via PackageKit D-Bus

```bash
# List PackageKit daemon
busctl list | grep PackageKit

# Introspect PackageKit interface
busctl introspect org.freedesktop.PackageKit \
  /org/freedesktop/PackageKit

# Check if package is installed (via PackageKit D-Bus)
busctl call org.freedesktop.PackageKit \
  /org/freedesktop/PackageKit \
  org.freedesktop.PackageKit \
  GetPackages s "installed"
```

## How op-dbus Manages Containers (D-Bus Native)

### Declaration in NixOS

```nix
services.op-dbus.stateConfig = {
  # Systemd manages container units
  systemd = {
    units = {
      "pve-container@100.service" = {
        enabled = true;
        active_state = "active";
      };
      "pve-container@101.service" = {
        enabled = true;
        active_state = "active";
      };
    };
  };

  # LXC plugin discovers containers via OVS ports
  lxc = {
    containers = [
      {
        id = "100";
        veth = "veth100";
        bridge = "ovsbr0";
        running = true;
      }
    ];
  };
};
```

### What Happens

1. **NixOS** writes config to `/etc/op-dbus/state.json`
2. **op-dbus** reads state and calls plugins
3. **systemd plugin** uses D-Bus to manage container units:
   ```rust
   // src/state/plugins/systemd.rs
   proxy.call("StartUnit", &("pve-container@100.service", "replace")).await
   ```
4. **LXC plugin** uses OVSDB JSON-RPC to discover containers:
   ```rust
   // src/state/plugins/lxc.rs
   client.list_bridge_ports("ovsbr0").await
   ```

No CLI commands executed!

## Container Configuration

### Important Distinction

**Host-level (D-Bus native):**
- Container lifecycle: Create, start, stop → systemd D-Bus
- Network discovery: OVS ports → OVSDB JSON-RPC
- Package management: Host packages → PackageKit D-Bus

**Inside containers (CLI is fine!):**
- `wg-quick up wg0` - Start WARP tunnel
- `apt install curl` - Install packages
- `xray run -config /etc/xray/config.json` - Run Xray
- `iptables -t nat -A POSTROUTING -j MASQUERADE` - Configure NAT

The D-Bus philosophy applies to **op-dbus managing the system**, not to processes running **inside** containers.

### Container Setup Workflow

Once containers are created via D-Bus, configure them using standard tools:

```bash
# SSH into host
ssh root@oo1424oo

# Enter container (this is fine for setup!)
lxc-attach -n 101

# Inside container: Install and configure WARP
apt update && apt install curl
curl -L -o /usr/local/bin/wgcf https://github.com/ViRb3/wgcf/releases/latest/download/wgcf_linux_amd64
chmod +x /usr/local/bin/wgcf
wgcf register
wgcf generate
apt install wireguard-tools
mv wgcf-profile.conf /etc/wireguard/wg0.conf

# Add OVS integration
echo "PostUp = ovs-vsctl add-port ovsbr0 wg0" >> /etc/wireguard/wg0.conf
echo "PreDown = ovs-vsctl del-port ovsbr0 wg0" >> /etc/wireguard/wg0.conf

# Start WARP
systemctl enable --now wg-quick@wg0
```

**Key Point:** Once configured, containers are managed via D-Bus (start/stop/monitor). Manual configuration is a one-time setup.

## Verification via D-Bus

### Check Container State

```bash
# Get all container units
busctl call org.freedesktop.systemd1 \
  /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager \
  ListUnits | grep pve-container

# Output:
# pve-container@100.service loaded active running Proxmox VE LXC Container: 100
# pve-container@101.service loaded active running Proxmox VE LXC Container: 101
# pve-container@102.service loaded active running Proxmox VE LXC Container: 102
```

### Check Network State via OVSDB

```bash
# op-dbus logs show OVSDB queries
journalctl -u op-dbus -f | grep -i "port\|bridge"

# Example output:
# Discovered container 100 on bridge ovsbr0 via port veth100
# Discovered container 101 on bridge ovsbr0 via port veth101
```

### Check Package State via PackageKit

```bash
# View PackageKit transactions
busctl call org.freedesktop.PackageKit \
  /org/freedesktop/PackageKit \
  org.freedesktop.PackageKit \
  GetTransactionList

# Monitor PackageKit signals
busctl monitor org.freedesktop.PackageKit
```

## Full D-Bus Deployment Flow

### 1. Deploy Configuration

```bash
# Copy NixOS config (declarative state)
scp nix/oo1424oo-config.nix root@oo1424oo:/etc/nixos/configuration.nix

# Build and switch (NixOS handles systemd)
ssh root@oo1424oo nixos-rebuild switch
```

### 2. Verify via D-Bus

```bash
# Check op-dbus is running
busctl status org.opdbus

# Check systemd started containers
busctl call org.freedesktop.systemd1 /org/freedesktop/systemd1 \
  org.freedesktop.systemd1.Manager ListUnits | grep container

# Check op-dbus logs
journalctl -u op-dbus -f
```

### 3. Inspect State via D-Bus

```bash
# Get container unit status
for i in 100 101 102; do
  busctl get-property org.freedesktop.systemd1 \
    "/org/freedesktop/systemd1/unit/pve_2dcontainer_40${i}_2eservice" \
    org.freedesktop.systemd1.Unit ActiveState
done

# Expected output:
# s "active"
# s "active"
# s "active"
```

## Benefits of D-Bus Native Approach

1. **No CLI parsing** - Structured data via D-Bus
2. **Type-safe** - D-Bus has strong typing
3. **Event-driven** - Subscribe to D-Bus signals for state changes
4. **Auditable** - All D-Bus calls logged by systemd
5. **Secure** - D-Bus has built-in auth/authorization
6. **Introspectable** - Can query entire system state

## Current Limitations

The LXC plugin currently uses `pct` CLI for container **creation**. This should be replaced with:

1. **Proxmox API** (JSON-RPC over HTTP) for container creation
2. **systemd D-Bus** for container lifecycle (start/stop/status)
3. **OVSDB JSON-RPC** for network discovery (already implemented!)

See `src/state/plugins/lxc.rs` lines 288-314 for where CLI needs to be replaced with API calls.

## Next Steps

1. Replace `pct` CLI with Proxmox API calls
2. Add D-Bus event monitoring for container state changes
3. Pre-build container templates with configuration baked in
4. Document full D-Bus API surface for op-dbus

## Example: Full D-Bus Monitoring

```bash
#!/bin/bash
# Monitor all op-dbus D-Bus activity

# Watch systemd D-Bus for container unit changes
busctl monitor org.freedesktop.systemd1 &

# Watch PackageKit D-Bus for package changes
busctl monitor org.freedesktop.PackageKit &

# Watch op-dbus logs (shows OVSDB queries)
journalctl -u op-dbus -f &

wait
```

This gives you **complete visibility** into system state changes without ever touching CLI tools!
