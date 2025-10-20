# Dual Bridge Architecture: mesh + vmbr0

## Overview

Two separate OVS bridges for clear network isolation:
- **mesh** - Netmaker mesh containers
- **vmbr0** - Traditional bridge containers

## Architecture

```
┌─────────────────────────────────────────┐
│              Physical Host              │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────────┐      ┌─────────────┐  │
│  │    mesh     │      │    vmbr0    │  │
│  │ (Netmaker)  │      │ (Physical)  │  │
│  ├─────────────┤      ├─────────────┤  │
│  │ nm-network  │      │    ens1     │  │ ← Physical uplink
│  │ vi100       │      │    vi101    │  │
│  │ vi102       │      │    vi103    │  │
│  └─────────────┘      └─────────────┘  │
│         ↓                     ↓         │
│    (Wireguard)           (Physical)     │
│         ↓                     ↓         │
│   Mesh Network          Internet        │
└─────────────────────────────────────────┘
```

## Bridge Configuration

### mesh Bridge
**Purpose:** Netmaker mesh containers

**Ports:**
- `nm-*` - Netmaker/wireguard interface (host joined to mesh)
- `vi100`, `vi102`, etc. - Netmaker container interfaces

**Routing:**
```bash
# Mesh routes through wireguard
ip route show dev nm-mynetwork
100.64.0.0/10 dev nm-mynetwork
```

**Traffic Flow:**
```
Container vi100 → mesh bridge → nm-mynetwork (wireguard) → Encrypted mesh
```

### vmbr0 Bridge
**Purpose:** Traditional networking

**Ports:**
- `ens1` - Physical uplink to external network
- `vi101`, `vi103`, etc. - Bridge container interfaces

**Routing:**
```bash
# Default route through physical interface
ip route show dev vmbr0
default via 80.209.240.129 dev vmbr0
```

**Traffic Flow:**
```
Container vi101 → vmbr0 bridge → ens1 (physical) → Internet
```

## Benefits

### 1. Traffic Isolation
- Mesh traffic never touches physical uplink
- Bridge traffic never touches mesh
- Clear separation of concerns

### 2. Security
- Mesh containers can't accidentally bridge to external network
- Bridge containers isolated from mesh
- Easier to apply firewall rules per bridge

### 3. Management
```bash
# View mesh containers
sudo ovs-vsctl list-ports mesh
nm-mynetwork
vi100
vi102

# View bridge containers
sudo ovs-vsctl list-ports vmbr0
ens1
vi101
vi103
```

### 4. Performance
- Mesh traffic optimized for wireguard
- Bridge traffic has direct physical uplink
- No cross-contamination

### 5. Troubleshooting
```bash
# Debug mesh network
sudo ovs-ofctl dump-flows mesh
sudo wg show

# Debug bridge network
sudo ovs-ofctl dump-flows vmbr0
sudo tcpdump -i ens1
```

## Container Configuration

### Netmaker Container (mesh bridge)
```json
{
  "id": "100",
  "veth": "vi100",
  "bridge": "vmbr0",  // Ignored - automatically uses "mesh"
  "properties": {
    "network_type": "netmaker"
  }
}
```

**Result:**
- Container created on `mesh` bridge (automatic)
- Inherits host's netmaker membership
- Gets mesh IP automatically

### Traditional Container (vmbr0 bridge)
```json
{
  "id": "101",
  "veth": "vi101",
  "bridge": "vmbr0",  // Used for traditional mode
  "properties": {
    "network_type": "bridge"
  }
}
```

**Result:**
- Container created on `vmbr0` bridge
- Gets IP via DHCP or static
- Routes through physical uplink

## Implementation

### Install Script Creates mesh Bridge

**File:** `install.sh:221-234`

```bash
# Step 5: Create mesh bridge for netmaker containers
if ! sudo ovs-vsctl br-exists mesh; then
    sudo ovs-vsctl add-br mesh
    sudo ip link set mesh up
    echo "✓ Created 'mesh' bridge"
fi
```

### LXC Plugin Selects Bridge

**File:** `lxc.rs:144-156`

```rust
fn get_bridge_for_network_type(container: &ContainerInfo) -> String {
    let network_type = container.properties
        .as_ref()
        .and_then(|p| p.get("network_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("bridge");

    match network_type {
        "netmaker" => "mesh".to_string(),      // Netmaker bridge
        "bridge" | _ => container.bridge.clone(), // vmbr0
    }
}
```

### Container Creation Flow

```
1. User defines container with network_type
         ↓
2. Plugin calls get_bridge_for_network_type()
   - "netmaker" → bridge = "mesh"
   - "bridge"   → bridge = "vmbr0"
         ↓
3. pct create --net0 name=eth0,bridge={selected_bridge}
         ↓
4. Veth renamed to vi{VMID}
         ↓
5. Veth added to correct bridge:
   - mesh: client.add_port("mesh", veth)
   - vmbr0: client.add_port("vmbr0", veth)
         ↓
6. Container live on appropriate network
```

## OVS Flow Tables

### mesh Bridge Flows
```bash
$ sudo ovs-ofctl dump-flows mesh
priority=0 actions=NORMAL
```

**Behavior:**
- L2 learning switch
- Learns MAC addresses automatically
- Forwards to wireguard interface (nm-*)

### vmbr0 Bridge Flows
```bash
$ sudo ovs-ofctl dump-flows vmbr0
priority=0 actions=NORMAL
```

**Behavior:**
- L2 learning switch
- Learns MAC addresses automatically
- Forwards to physical uplink (ens1)

## Network Namespace View

### Host View
```bash
$ ip link show type openvswitch
4: mesh: <BROADCAST,MULTICAST,UP> mtu 1500
    link/ether aa:bb:cc:dd:ee:ff
5: vmbr0: <BROADCAST,MULTICAST,UP> mtu 1500
    link/ether 11:22:33:44:55:66

$ ip link show type veth
123: vi100@if2: <BROADCAST,MULTICAST,UP> ...  # On mesh
124: vi101@if2: <BROADCAST,MULTICAST,UP> ...  # On vmbr0
```

### Routing Table
```bash
$ ip route
default via 80.209.240.129 dev vmbr0          # Physical default
100.64.0.0/10 dev nm-mynetwork scope link     # Mesh routes
80.209.240.128/25 dev vmbr0 scope link        # Physical subnet
```

## Example Deployment

### Setup
```bash
# 1. Install (creates both bridges)
sudo ./install.sh

# 2. Join host to netmaker
echo "NETMAKER_TOKEN=your-token" | sudo tee /etc/op-dbus/netmaker.env
sudo ./install.sh  # Re-run to join

# 3. Verify bridges
sudo ovs-vsctl list-br
mesh
vmbr0

sudo ovs-vsctl list-ports mesh
nm-mynetwork

sudo ovs-vsctl list-ports vmbr0
ens1
```

### Create Containers
```bash
# State file with mixed containers
cat > /tmp/containers.json <<'EOF'
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [
        {
          "id": "100",
          "veth": "vi100",
          "bridge": "vmbr0",
          "properties": {"network_type": "netmaker"}
        },
        {
          "id": "101",
          "veth": "vi101",
          "bridge": "vmbr0",
          "properties": {"network_type": "bridge"}
        }
      ]
    }
  }
}
EOF

# Apply
sudo op-dbus apply /tmp/containers.json

# Verify placement
sudo ovs-vsctl list-ports mesh
nm-mynetwork
vi100        # Netmaker container

sudo ovs-vsctl list-ports vmbr0
ens1
vi101        # Bridge container
```

## Troubleshooting

### Container on Wrong Bridge?
```bash
# Check which bridge has the veth
sudo ovs-vsctl port-to-br vi100
mesh  # Correct for netmaker

sudo ovs-vsctl port-to-br vi101
vmbr0  # Correct for bridge

# Move if wrong (manual)
sudo ovs-vsctl del-port vmbr0 vi100
sudo ovs-vsctl add-port mesh vi100
```

### Netmaker Not Working?
```bash
# Check host joined
netclient list

# Check mesh bridge has wireguard interface
sudo ovs-vsctl list-ports mesh | grep nm-

# Check routing
ip route show dev nm-mynetwork

# Test from container
sudo pct enter 100
ping 100.64.1.5  # Another mesh node
```

### Bridge Connectivity Issues?
```bash
# Check physical uplink on vmbr0
sudo ovs-vsctl list-ports vmbr0 | grep ens1

# Check bridge is up
ip link show vmbr0

# Test from container
sudo pct enter 101
ping 8.8.8.8  # External
```

## Migration

### Moving Container Between Bridges

**Example:** Move container 100 from vmbr0 to mesh

```bash
# 1. Stop container
sudo pct stop 100

# 2. Update state.json
# Change: "properties": {"network_type": "bridge"}
# To:     "properties": {"network_type": "netmaker"}

# 3. Manually move veth (or delete and recreate)
sudo ovs-vsctl del-port vmbr0 vi100
sudo ovs-vsctl add-port mesh vi100

# 4. Start container
sudo pct start 100

# 5. Verify
sudo ovs-vsctl port-to-br vi100  # Should show "mesh"
```

## Summary

### Bridge Roles
| Bridge | Purpose | Connected To | Containers |
|--------|---------|--------------|------------|
| **mesh** | Netmaker mesh | nm-* (wireguard) | Mesh containers (vi100, vi102) |
| **vmbr0** | Traditional | ens1 (physical) | Bridge containers (vi101, vi103) |

### Automatic Bridge Selection
- `network_type: "netmaker"` → `mesh` bridge
- `network_type: "bridge"` → `vmbr0` bridge
- No manual configuration needed!

### Benefits
✅ Clear isolation
✅ Security boundaries
✅ Easy management
✅ Better performance
✅ Simplified troubleshooting

**Two bridges, zero confusion!**
