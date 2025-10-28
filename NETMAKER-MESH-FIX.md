# Netmaker Mesh Bridge Configuration Fix

## Issue
Containers were unable to connect to the netmaker server via the mesh bridge. The OVS data flows existed for containers to connect to the internet and allowed them to connect to the netmaker interface through the mesh bridge without needing a second interface in the container, but this functionality was lost.

## Root Cause
The `netmaker` interface was not added to the `mesh` OVS bridge. The mesh bridge existed with the correct OVS flows (`actions=NORMAL`), but without the netmaker interface attached, containers couldn't reach the netmaker mesh network.

## Architecture
```
┌─────────────────────────────────────────┐
│              Physical Host               │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────────┐      ┌─────────────┐  │
│  │    mesh     │      │    vmbr0    │  │
│  │ (Netmaker)  │      │ (Physical)  │  │
│  ├─────────────┤      ├─────────────┤  │
│  │ netmaker    │      │    ens1     │  │ ← Physical uplink
│  │ vi100       │      │    vi101    │  │ ← Containers
│  │ vi102       │      │    vi103    │  │
│  └─────────────┘      └─────────────┘  │
│         ↓                     ↓         │
│    (Wireguard)           (Physical)     │
│         ↓                     ↓         │
│   Mesh Network          Internet        │
└─────────────────────────────────────────┘
```

## What Was Fixed

### 1. Added Netmaker Interface to Mesh Bridge
Manually added the `netmaker` interface to the `mesh` bridge:
```bash
sudo ovs-vsctl add-port mesh netmaker
```

### 2. Updated Sync Script
Modified `sync-netmaker-mesh.sh` to:
- Detect interfaces named `netmaker` (not just `nm-*`)
- Properly check netclient JSON output instead of text output

Changes in `sync-netmaker-mesh.sh`:
```bash
# Before: Only looked for nm-* interfaces
for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-")) | .ifname'); do

# After: Looks for nm-* OR netmaker interface
for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-") or . == "netmaker") | .ifname'); do
```

### 3. Updated Install Script
Modified `install.sh` in two places to detect and add the `netmaker` interface name pattern:

```bash
# Lines 575 and 599
for iface in $(ip -j link show | jq -r '.[] | select(.ifname | startswith("nm-") or . == "netmaker") | .ifname'); do
```

## OVS Flow Configuration

### mesh Bridge Flows
```bash
$ sudo ovs-ofctl dump-flows mesh
priority=0 actions=NORMAL
```

**Behavior:**
- L2 learning switch
- Learns MAC addresses automatically
- Forwards to netmaker interface (wireguard)
- Container traffic → mesh bridge → netmaker interface → encrypted mesh

### vmbr0 Bridge Flows
```bash
$ sudo ovs-ofctl dump-flows vmbr0
priority=0 actions=NORMAL
```

**Behavior:**
- L2 learning switch
- Learns MAC addresses automatically
- Forwards to physical uplink (ens1)
- Container traffic → vmbr0 bridge → ens1 → internet

## How It Works Now

### For Netmaker Containers
1. Container created with `network_type: "netmaker"`
2. Container assigned to `mesh` bridge
3. Container veth renamed to `vi{VMID}`
4. Veth added to `mesh` bridge
5. Traffic flows: Container → mesh bridge → netmaker interface → encrypted mesh network

### For Bridge Containers
1. Container created with `network_type: "bridge"`
2. Container assigned to `vmbr0` bridge (or specified bridge)
3. Container veth renamed to `vi{VMID}`
4. Veth added to specified bridge
5. Traffic flows: Container → vmbr0 bridge → ens1 → internet

## Verification

### Check mesh bridge
```bash
sudo ovs-vsctl list-ports mesh
# Should show: netmaker

sudo ovs-ofctl dump-flows mesh
# Should show: priority=0 actions=NORMAL
```

### Check netmaker routes
```bash
ip route show dev netmaker
# Should show: 100.104.70.0/24 proto kernel scope link src 100.104.70.1
```

### Check container placement
```bash
# Netmaker container
sudo ovs-vsctl port-to-br vi100
# Should show: mesh

# Bridge container
sudo ovs-vsctl port-to-br vi101
# Should show: vmbr0
```

## Running the Sync Script

After making changes or restarting netmaker, run:
```bash
sudo ./sync-netmaker-mesh.sh
```

This will:
1. Check if mesh bridge exists
2. Verify netclient is installed
3. Check if host is joined to netmaker
4. Find and add netmaker interfaces to mesh bridge
5. Show current mesh bridge configuration

## Summary

✅ **Fixed:** Netmaker interface now added to mesh bridge  
✅ **Fixed:** Scripts updated to detect `netmaker` interface name  
✅ **Verified:** OVS flows configured correctly (NORMAL action)  
✅ **Verified:** Netmaker routes present (100.104.70.0/24)  

Containers can now connect to the netmaker server through the mesh bridge without needing a second interface in the container.
