# Container Network Automation

## Overview

op-dbus automates container network enrollment with two modes:
- **Netmaker**: Automatic mesh network enrollment
- **Bridge**: Traditional OVS bridge attachment

## One-Time Setup (Install Script)

These operations happen **once** during `./install.sh`:

### 1. Netclient Installation Check
```bash
# Checks if netclient is installed
command -v netclient

# If not found, provides installation instructions:
curl -sL https://apt.netmaker.org/gpg.key | sudo apt-key add -
curl -sL https://apt.netmaker.org/debian.deb.txt | sudo tee /etc/apt/sources.list.d/netmaker.list
sudo apt update && sudo apt install netclient
```

### 2. Token Environment File Creation
```bash
# Creates /etc/op-dbus/netmaker.env (chmod 600)
# Contains reusable enrollment tokens
```

**File:** `/etc/op-dbus/netmaker.env`
```bash
# Default token (used for all containers unless overridden)
NETMAKER_TOKEN=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

# Optional: Multiple tokens for different networks
NETMAKER_TOKEN_PROD=token-for-production
NETMAKER_TOKEN_DEV=token-for-development
```

### 3. State File Template
```bash
# Creates /etc/op-dbus/state.json with introspected network config
# User adds container definitions here
```

---

## Per-Container Operations (On Creation)

These operations happen **each time** a container is created via `op-dbus apply`:

### Container State Definition

**State File:** `/etc/op-dbus/state.json`

#### Netmaker-Enabled Container
```json
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [{
        "id": "100",
        "veth": "vi100",
        "bridge": "vmbr0",
        "running": true,
        "properties": {
          "network_type": "netmaker"
        }
      }]
    }
  }
}
```

#### Traditional Bridge Container
```json
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [{
        "id": "101",
        "veth": "vi101",
        "bridge": "vmbr0",
        "running": true,
        "properties": {
          "network_type": "bridge",
          "ipv4": "192.168.1.101/24"
        }
      }]
    }
  }
}
```

#### Multi-Token Netmaker Container
```json
{
  "id": "102",
  "veth": "vi102",
  "bridge": "vmbr0",
  "properties": {
    "network_type": "netmaker",
    "netmaker_token_env": "NETMAKER_TOKEN_PROD"
  }
}
```

### Automatic Operations (LXC Plugin)

When `op-dbus apply` is executed:

1. **Container Creation**
   ```rust
   create_lxc_container(&container.id).await
   ```
   - Creates LXC container via `pct create` or native LXC API
   - Container gets default veth interface

2. **Veth Rename**
   ```rust
   rename_veth(&container.id, "vi{VMID}").await
   ```
   - Finds container's veth interface
   - Renames to `vi{VMID}` (e.g., `vi100`)
   - Makes it discoverable by op-dbus

3. **Network Enrollment (Conditional)**

   **If `network_type: "netmaker"`:**
   ```rust
   join_netmaker(&container.id, token_env).await
   ```
   - Loads token from environment variable
   - Executes: `netclient join -t $NETMAKER_TOKEN`
   - Container joins mesh network automatically
   - No manual bridge configuration needed

   **If `network_type: "bridge"`:**
   ```rust
   client.add_port(&container.bridge, &veth_name).await
   ```
   - Adds `vi{VMID}` to specified OVS bridge
   - Traditional networking
   - Optional static IP configuration

4. **Blockchain Commit**
   ```rust
   StateManager::record_footprint(&diff, "apply", data)
   ```
   - Records container creation in blockchain audit log
   - Automatic via StateManager

5. **NonNet DB Exposure**
   - Container automatically appears in `OpNonNet` database
   - Queryable via JSON-RPC at `/run/op-dbus/nonnet.db.sock`
   - Table: `lxc`, Columns: `id`, `veth`, `bridge`, `running`, `properties`

---

## Comparison: One-Time vs Per-Container

| Operation | When | Frequency | Tool |
|-----------|------|-----------|------|
| **Install netclient** | Setup | Once | `install.sh` |
| **Create token file** | Setup | Once | `install.sh` |
| **Add tokens** | Setup | Once (per network) | Manual edit |
| **Define container** | Usage | Per container | Edit state.json |
| **Create container** | Apply | Per container | LXC plugin |
| **Rename veth** | Apply | Per container | LXC plugin |
| **Join netmaker** | Apply | Per container | LXC plugin |
| **Add to bridge** | Apply | Per container | LXC plugin |
| **Blockchain commit** | Apply | Per container | StateManager |

---

## Usage Workflow

### Initial Setup (Once)
```bash
# 1. Install op-dbus
cargo build --release
sudo ./install.sh

# 2. Add netmaker token
echo "NETMAKER_TOKEN=your-token" | sudo tee -a /etc/op-dbus/netmaker.env

# 3. Verify
sudo cat /etc/op-dbus/netmaker.env
```

### Create Containers (Repeatable)
```bash
# 1. Edit state file to add container definition
sudo nano /etc/op-dbus/state.json

# 2. Preview changes
sudo op-dbus diff /etc/op-dbus/state.json

# 3. Apply (creates container, renames veth, joins netmaker)
sudo op-dbus apply /etc/op-dbus/state.json

# 4. Verify
sudo op-dbus query --plugin lxc
netclient list
```

### View Container State
```bash
# Query via op-dbus
op-dbus query --plugin lxc

# Query via NonNet DB (OVSDB-like)
echo '{"method":"transact","params":["OpNonNet",[{"op":"select","table":"lxc","where":[]}]],"id":1}' | \
  nc -U /run/op-dbus/nonnet.db.sock

# Check netmaker enrollment
netclient list
```

---

## Security

- **Token Storage**: `/etc/op-dbus/netmaker.env` (chmod 600, root-only)
- **Not in Logs**: Tokens loaded from env, not logged
- **Not in Blockchain**: Container properties don't include tokens
- **Reusable**: Same token can enroll multiple containers
- **Per-Network**: Different tokens for prod/dev/staging networks

---

## Implementation Status

### ✅ Completed
- [x] Install script netclient check
- [x] Token environment file creation
- [x] Container state schema design
- [x] Network type conditional logic

### 🚧 In Progress
- [ ] LXC plugin `apply_state()` implementation
- [ ] Veth rename function (rtnetlink)
- [ ] Netmaker join function
- [ ] Container creation (pct/LXC API)

### 📋 Planned
- [ ] Container modification support
- [ ] Container deletion support
- [ ] Netmaker leave on deletion
- [ ] Health checks and retry logic

---

## Example: Full Container Lifecycle

```bash
# 1. One-time setup (install script already did this)
ls /etc/op-dbus/netmaker.env  # ✓ Token file exists
which netclient                # ✓ netclient installed

# 2. Add container to state.json
cat >> /etc/op-dbus/state.json <<'EOF'
{
  "version": 1,
  "plugins": {
    "lxc": {
      "containers": [{
        "id": "100",
        "veth": "vi100",
        "bridge": "vmbr0",
        "running": true,
        "properties": {
          "network_type": "netmaker"
        }
      }]
    }
  }
}
EOF

# 3. Apply (automatic operations happen here)
sudo op-dbus apply /etc/op-dbus/state.json
# Output:
#   Created container 100
#   Renamed veth to vi100
#   Container 100 joined netmaker
#   Blockchain footprint recorded

# 4. Verify
sudo op-dbus query --plugin lxc | jq '.containers[0]'
# {
#   "id": "100",
#   "veth": "vi100",
#   "bridge": "vmbr0",
#   "running": true,
#   "properties": {
#     "network_type": "netmaker"
#   }
# }

netclient list
# Connected networks:
#   - prod-cluster (100.64.0.5)

# Container now has mesh networking automatically!
```

---

## Architecture Benefits

1. **One-Time Setup**: netclient installation and token configuration done once
2. **Declarative**: Container definitions in state.json
3. **Automatic**: Network enrollment happens on container creation
4. **Flexible**: Choose netmaker or bridge per container
5. **Secure**: Tokens in protected environment file
6. **Auditable**: All operations logged to blockchain
7. **Queryable**: Container state exposed via NonNet DB
