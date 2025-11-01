# Blockchain Footprints & Container Networking

## Part 1: Blockchain Footprint Structure

### What Gets Recorded

Every operation (apply, create, modify, delete) creates a footprint with:

### Footprint Structure

**Source:** `src/blockchain/plugin_footprint.rs:10-18`

```rust
pub struct PluginFootprint {
    pub plugin_id: String,         // "lxc", "net", "systemd", etc.
    pub operation: String,          // "create", "modify", "delete", "apply"
    pub timestamp: u64,             // Unix timestamp
    pub data_hash: String,          // SHA-256 of operation data
    pub content_hash: String,       // SHA-256 of plugin:operation:timestamp
    pub metadata: HashMap<String, Value>,  // Operation details
    pub vector_features: Vec<f32>, // 64-dimensional vector for ML analysis
}
```

### Example: Container Creation Footprint

When you create container 100:

```json
{
  "plugin_id": "lxc",
  "operation": "apply",
  "timestamp": 1729414800,
  "data_hash": "a7f5c8d9e2b1f4a6c3d7e9f1b2c4d6e8f0a1b3c5d7e9f1b3c5d7e9f1b3c5d7e9",
  "content_hash": "1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2",
  "metadata": {
    "plugin": "lxc",
    "actions": [
      {
        "Create": {
          "resource": "100",
          "config": {
            "id": "100",
            "veth": "vi100",
            "bridge": "ovsbr0",
            "properties": {
              "network_type": "netmaker"
            }
          }
        }
      }
    ],
    "metadata": {
      "timestamp": 1729414800,
      "current_hash": "d41d8cd98f00b204e9800998ecf8427e",
      "desired_hash": "098f6bcd4621d373cade4e832627b4f6"
    },
    "result": {
      "success": true,
      "changes": [
        "Created container 100",
        "Renamed veth119abc to vi100",
        "Container 100 on netmaker mesh (inherited from host)"
      ],
      "errors": []
    }
  },
  "vector_features": [0.123, 0.456, 0.789, ... ] // 64 dimensions
}
```

### Hashing Details

**Data Hash** (SHA-256 of operation data):
```rust
let data_str = serde_json::to_string(&metadata)?;
let data_hash = sha256(data_str);
// Example: "a7f5c8d9e2b1f4a6c3d7e9f1b2c4d6e8f0a1b3c5d7e9f1b3c5d7e9f1b3c5d7e9"
```

**Content Hash** (SHA-256 of context):
```rust
let content = format!("{}:{}:{}", plugin_id, operation, timestamp);
let content_hash = sha256(content);
// Example: "lxc:apply:1729414800" -> "1b2c3d4e..."
```

### Vector Features (64 dimensions)

**Source:** `src/blockchain/plugin_footprint.rs:102-196`

Used for ML analysis and anomaly detection:

```rust
// Feature breakdown:
[0]     Plugin ID hash (normalized 0-1)
[1-4]   Operation type: [create, update, delete, query]
[5]     Is object (1.0) or not (0.0)
[6]     Data size (normalized)
[7]     Key diversity
[8-11]  Value type distribution (string, number, bool, null)
[12-17] Metadata features
[18-19] Temporal features (hour of day, day of week)
[20-63] Reserved for future features
```

**Example Vector:**
```
[0.342,   // Plugin hash
 1.0, 0.0, 0.0, 0.0,  // Operation: create
 1.0,     // Is object
 0.045,   // Data size
 0.231,   // Key diversity
 0.6, 0.2, 0.1, 0.1,  // Value types
 0.3,     // Metadata size
 1.0, 0.0, 1.0, 0.0, 1.0,  // Metadata keys present
 0.583,   // Hour (14:00)
 0.286,   // Day (Tuesday)
 0.0, ... // Reserved
]
```

### Storage Location

**Blockchain Directory:** `/var/lib/op-dbus/blockchain/`

```bash
/var/lib/op-dbus/blockchain/
├── block_0000000001.json  # First block
├── block_0000000002.json  # Second block
├── block_0000000003.json  # Third block
└── index.json              # Blockchain index
```

**Block File Format:**
```json
{
  "block_id": 1,
  "timestamp": 1729414800,
  "previous_hash": "0000000000000000000000000000000000000000000000000000000000000000",
  "footprints": [
    { /* PluginFootprint 1 */ },
    { /* PluginFootprint 2 */ },
    { /* PluginFootprint 3 */ }
  ],
  "block_hash": "abc123..."
}
```

---

## Part 2: OVS Flows & Container Networking

### Current OVS Configuration

**Bridge Ports:**
```bash
$ sudo ovs-vsctl list-ports ovsbr0
ens1           # Physical uplink
fwln99999o0    # Firewall link (Proxmox)
```

**Flow Rules:**
```bash
$ sudo ovs-ofctl dump-flows ovsbr0
cookie=0x0, duration=50609s, table=0, n_packets=3586645,
  n_bytes=1643976035, idle_age=0, priority=0 actions=NORMAL
```

### What "NORMAL" Action Means

**Action: NORMAL**
- OVS acts like a traditional L2 learning switch
- MAC learning enabled
- Broadcasts/floods unknown destinations
- No custom flow rules

This is **perfect for containers** because:
- Simple L2 forwarding
- Automatic MAC learning
- Works with any protocol
- No flow programming needed

### After Container Creation

When container 100 is created:

```bash
$ sudo ovs-vsctl list-ports ovsbr0
ens1           # Physical uplink
fwln99999o0    # Firewall link
vi100          # Container 100 (NEW)
```

**Flow remains:**
```
actions=NORMAL  # Still learning switch
```

**What happens:**
1. Container 100 sends packet from `vi100`
2. OVS learns MAC address on port `vi100`
3. Future packets to that MAC forwarded to `vi100` only
4. Broadcasts go to all ports (ens1, fwln99999o0, vi100)

---

## Part 3: Single Interface Implementation

### The Architecture

**One Interface Per Container:**
```
Container 100
    └── eth0 (inside container)
         └── Connected to vi100 (host side veth)
              └── Attached to ovsbr0 OVS bridge
                   └── Connected to ens1 (physical uplink)
```

### How It Works

#### Step 1: Container Created
```bash
pct create 100 template \
  --net0 name=eth0,bridge=ovsbr0,firewall=1
```

**What pct does:**
- Creates veth pair: `vethXXX` (host) ↔ `eth0` (container)
- Container sees only `eth0`
- Host sees `vethXXX` (temporary name)

#### Step 2: Veth Renamed
```rust
link_set_name("veth119abc", "vi100")
```

**Why rename:**
- Discoverable pattern `vi{VMID}`
- Consistent naming
- Easy to identify in `ovs-vsctl list-ports`

#### Step 3: Added to Bridge

**Netmaker Mode:**
```rust
// Do nothing - veth already created by pct with bridge=ovsbr0
// Container inherits host's netmaker/wireguard
log::info!("Container {} on netmaker mesh (inherited from host)");
```

**Bridge Mode:**
```rust
// Explicitly add to OVS (redundant if pct already did it)
client.add_port("ovsbr0", "vi100").await
```

### Interface Details

**Inside Container:**
```bash
$ sudo pct enter 100
root@ct100:~# ip addr
1: lo: <LOOPBACK,UP> ...
2: eth0@if123: <BROADCAST,MULTICAST,UP> ...
    link/ether 02:00:00:00:01:64
    inet 192.168.1.100/24  # Or DHCP
```

**On Host:**
```bash
$ ip link show vi100
123: vi100@if2: <BROADCAST,MULTICAST,UP> mtu 1500
    link/ether fe:00:00:00:01:64
```

**Note the numbers:**
- `eth0@if123` - Container's eth0 paired with host's interface 123
- `vi100@if2` - Host's vi100 paired with container's interface 2

### Why Single Interface is Sufficient

**One Interface Does Everything:**

1. **Netmaker Mode:**
   ```
   Container eth0
       ↓
   Host vi100 on ovsbr0
       ↓
   Host's wireguard mesh interface (nm-*)
       ↓
   Encrypted mesh traffic
   ```

   - Container traffic routes through host
   - Host has wireguard (from netmaker join)
   - Container inherits mesh automatically
   - No second interface needed!

2. **Bridge Mode:**
   ```
   Container eth0
       ↓
   Host vi100 on ovsbr0
       ↓
   Bridge to ens1 (uplink)
       ↓
   External network
   ```

   - Simple L2 bridging
   - One interface sufficient
   - Standard container networking

### Multiple Interfaces (Future Enhancement)

**If you wanted multiple interfaces:**

```json
{
  "id": "100",
  "interfaces": [
    {
      "name": "eth0",
      "veth": "vi100",
      "bridge": "ovsbr0",
      "network_type": "netmaker"
    },
    {
      "name": "eth1",
      "veth": "vi100-1",
      "bridge": "vmbr1",
      "network_type": "bridge"
    }
  ]
}
```

**Implementation would need:**
```bash
pct set 100 --net1 name=eth1,bridge=vmbr1
# Creates second veth pair
# Rename second veth to vi100-1
# Add to second bridge
```

**Current implementation: One interface (sufficient for most use cases)**

---

## Part 4: Netmaker + Single Interface

### How Containers Get Mesh Without Second Interface

**Traditional Approach (Wrong):**
```
Container
  ├── eth0 (regular network)
  └── nm-ct100 (netmaker interface)  ← Separate interface
```

**Your Implementation (Correct):**
```
Host (joined netmaker)
  └── nm-mynetwork (wireguard mesh)

Container
  └── eth0 → routes through host → uses host's nm-mynetwork
      ↑
      Single interface, mesh via host routing!
```

### Why This Works

**Network Namespace Inheritance:**
- Container in same network namespace as host (for routing)
- Or traffic NAT'd through host (if unprivileged)
- Either way, uses host's wireguard interface

**Routing Example:**
```bash
# On Host
$ ip route
default via 192.168.1.1 dev ovsbr0
100.64.0.0/10 dev nm-mynetwork  # Netmaker mesh routes

# Container packets to mesh IP
Container 100 → 100.64.1.5
  ↓
Host routing table matches 100.64.0.0/10
  ↓
Send via nm-mynetwork (wireguard)
  ↓
Encrypted mesh traffic
```

**Key Point:** Container doesn't need its own wireguard interface. Host's interface handles mesh routing for all containers.

---

## Part 5: Flow Visualization

### Container Creation Flow with Footprint

```
User: op-dbus apply state.json
         ↓
StateManager.apply_state()
         ↓
LxcPlugin.calculate_diff()
  → StateAction::Create detected
         ↓
LxcPlugin.apply_state()
  1. pct create 100 → Creates vethXXX
  2. pct start 100 → Brings up container
  3. find_veth() → Finds vethXXX
  4. link_set_name(vethXXX, vi100) → Renames
  5. network_type check:
     - netmaker: Log "inherited from host"
     - bridge: add_port(vi100, ovsbr0)
         ↓
StateManager.record_footprint()
  → FootprintGenerator.create_footprint()
    → SHA-256 hashes calculated
    → 64-dim vector generated
    → Metadata packaged
         ↓
StreamingBlockchain.add_footprint()
  → Appended to block file
  → Block hash updated
         ↓
Success!
  → Container live on network
  → Footprint in blockchain
  → Queryable via NonNet DB
```

### OVS Packet Flow (After Container Created)

```
Packet from Container 100 (10.0.0.100 → 8.8.8.8)
         ↓
Container eth0 (02:00:00:00:01:64)
         ↓
Host vi100 (fe:00:00:00:01:64)
         ↓
OVS bridge ovsbr0
  → MAC learning: 02:00:00:00:01:64 on port vi100
  → actions=NORMAL (L2 forwarding)
         ↓
If netmaker mode:
  Host routing table → nm-mynetwork (wireguard)
  → Encrypted mesh packet
         ↓
If bridge mode:
  Forward to ens1 (uplink)
  → External network
```

---

## Part 6: Querying Footprints

### View Blockchain

```bash
# List blocks
ls -lh /var/lib/op-dbus/blockchain/

# View latest block
sudo cat /var/lib/op-dbus/blockchain/block_$(ls /var/lib/op-dbus/blockchain/ | grep block | tail -1 | grep -o '[0-9]*').json | jq .

# Search for container operations
sudo grep -r "lxc" /var/lib/op-dbus/blockchain/*.json | jq .

# Find container 100 operations
sudo jq '.footprints[] | select(.metadata.plugin == "lxc") | select(.metadata.actions[].Create.config.id == "100")' \
  /var/lib/op-dbus/blockchain/*.json
```

### Query via op-dbus

```bash
# Current container state (from NonNet DB)
op-dbus query --plugin lxc

# Historical operations (from blockchain)
# (Would need separate blockchain query command - not yet implemented)
```

---

## Summary

### Footprints
- ✅ SHA-256 hashed audit trail
- ✅ 64-dimensional vectors for ML
- ✅ Stored in `/var/lib/op-dbus/blockchain/`
- ✅ Immutable append-only log

### OVS Flows
- ✅ Simple NORMAL action (L2 learning switch)
- ✅ No custom flows needed
- ✅ Automatic MAC learning
- ✅ Broadcast/flood support

### Single Interface
- ✅ One veth per container (eth0 inside, vi{VMID} outside)
- ✅ Sufficient for both netmaker and bridge modes
- ✅ Netmaker via host routing (inherited wireguard)
- ✅ Bridge via L2 forwarding
- ✅ No second interface needed

**Architecture is clean, simple, and fully auditable!**
