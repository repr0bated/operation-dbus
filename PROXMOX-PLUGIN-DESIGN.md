# Proxmox VE Plugin Design

## Overview

The Proxmox plugin manages Proxmox Virtual Environment (VE) servers declaratively, including VMs, containers, storage, networking, and cluster configuration.

**Important Distinction**:
- **Host kernel params** (mitigations, BIOS workarounds) → NixOS `boot.kernelParams` (no plugin)
- **Proxmox server management** → `proxmox` plugin (this document)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│              NixOS Host Server                      │
│                                                     │
│  boot.kernelParams = ["mitigations=off"]  ← Config │
│                                                     │
│  ┌───────────────────────────────────────────────┐ │
│  │         Proxmox VE Server (PVE)               │ │
│  │                                               │ │
│  │  op-dbus proxmox plugin manages:             │ │
│  │  ├─ VMs (qm)           ← PVE API             │ │
│  │  ├─ Containers (pct)   ← PVE API             │ │
│  │  ├─ Storage (pvesm)    ← PVE API             │ │
│  │  ├─ Network (vmbr*)    ← PVE API             │ │
│  │  └─ Cluster (pvecm)    ← PVE API             │ │
│  └───────────────────────────────────────────────┘ │
│                                                     │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ VM 100  │  │ VM 101  │  │ CT 200  │           │
│  │ (nginx) │  │ (db)    │  │ (cache) │           │
│  └─────────┘  └─────────┘  └─────────┘           │
└─────────────────────────────────────────────────────┘
```

## Why Proxmox Needs a Plugin

### Current State
- Proxmox stores config in `/etc/pve/` (cluster filesystem)
- Manual management via CLI (`qm`, `pct`, `pvesm`) or Web UI
- No declarative configuration
- Configuration drift across cluster nodes

### With op-dbus Proxmox Plugin
- **Declarative**: Define desired state in JSON
- **Version controlled**: Store VM configs in Git
- **Blockchain audit**: Track all VM changes
- **Replication**: Clone Proxmox servers (disaster recovery)
- **Consistency**: Ensure all cluster nodes match

## Proxmox Management Interfaces

### 1. REST API (Recommended)
Proxmox exposes full REST API at `https://pve-host:8006/api2/json/`

**Advantages**:
- Official, stable API
- Comprehensive coverage
- Async operations
- Supports authentication tokens

**Example**:
```bash
# List VMs
curl -k -H "Authorization: PVEAPIToken=root@pam!mytoken=UUID" \
  https://localhost:8006/api2/json/nodes/pve/qemu

# Start VM
curl -k -X POST -H "Authorization: PVEAPIToken=root@pam!mytoken=UUID" \
  https://localhost:8006/api2/json/nodes/pve/qemu/100/status/start
```

### 2. CLI Tools (Fallback)
- `qm` - QEMU/KVM VM management
- `pct` - LXC container management
- `pvesm` - Storage management
- `pvecm` - Cluster management

**Example**:
```bash
qm list
qm start 100
pct list
pct start 200
```

### 3. Direct Config Files (Read-only)
- `/etc/pve/qemu-server/*.conf` - VM configs
- `/etc/pve/lxc/*.conf` - Container configs
- `/etc/pve/storage.cfg` - Storage pools

Useful for reading current state, but changes should go through API.

## Declarative State Format

### Example Configuration

```json
{
  "version": 1,
  "plugins": {
    "proxmox": {
      "node": "pve",

      "vms": [
        {
          "vmid": 100,
          "name": "nginx-web",
          "state": "running",
          "cores": 2,
          "memory": 2048,
          "disk": {
            "storage": "local-lvm",
            "size": "20G"
          },
          "network": {
            "bridge": "vmbr0",
            "model": "virtio"
          },
          "boot": "order=scsi0;net0",
          "ostype": "l26"
        },
        {
          "vmid": 101,
          "name": "postgres-db",
          "state": "running",
          "cores": 4,
          "memory": 8192,
          "disk": {
            "storage": "local-lvm",
            "size": "100G"
          },
          "network": {
            "bridge": "vmbr0",
            "model": "virtio"
          },
          "cpu_mitigations": "off",
          "comment": "Database server - mitigations disabled for performance"
        }
      ],

      "containers": [
        {
          "vmid": 200,
          "name": "redis-cache",
          "state": "running",
          "cores": 2,
          "memory": 1024,
          "rootfs": {
            "storage": "local-lvm",
            "size": "8G"
          },
          "network": {
            "name": "eth0",
            "bridge": "vmbr0",
            "ip": "dhcp"
          },
          "ostemplate": "local:vztmpl/debian-12-standard_12.0-1_amd64.tar.zst"
        }
      ],

      "storage": [
        {
          "id": "local-lvm",
          "type": "lvmthin",
          "content": ["images", "rootdir"],
          "thinpool": "data",
          "vgname": "pve"
        },
        {
          "id": "nfs-backup",
          "type": "nfs",
          "server": "192.168.1.100",
          "export": "/mnt/backups",
          "content": ["backup", "iso"]
        }
      ],

      "network": [
        {
          "iface": "vmbr0",
          "type": "bridge",
          "bridge_ports": "eno1",
          "address": "192.168.1.10",
          "netmask": "255.255.255.0",
          "gateway": "192.168.1.1"
        },
        {
          "iface": "vmbr1",
          "type": "bridge",
          "bridge_ports": "eno2",
          "address": "10.0.0.1",
          "netmask": "255.255.255.0",
          "comment": "Internal VM network"
        }
      ],

      "cluster": {
        "enabled": true,
        "name": "prod-cluster",
        "nodes": [
          {"name": "pve1", "ip": "192.168.1.10"},
          {"name": "pve2", "ip": "192.168.1.11"},
          {"name": "pve3", "ip": "192.168.1.12"}
        ]
      },

      "backups": [
        {
          "schedule": "daily",
          "time": "02:00",
          "storage": "nfs-backup",
          "mode": "snapshot",
          "compression": "zstd",
          "vms": [100, 101],
          "containers": [200],
          "retention": {
            "keep_daily": 7,
            "keep_weekly": 4,
            "keep_monthly": 3
          }
        }
      ]
    }
  }
}
```

## Plugin Implementation

### File Structure

```
src/state/plugins/
├── proxmox/
│   ├── mod.rs              # Main plugin
│   ├── api.rs              # Proxmox API client
│   ├── vms.rs              # VM management (qm)
│   ├── containers.rs       # Container management (pct)
│   ├── storage.rs          # Storage management (pvesm)
│   ├── network.rs          # Network management
│   ├── cluster.rs          # Cluster management (pvecm)
│   └── backups.rs          # Backup scheduling (vzdump)
```

### Core Plugin (src/state/plugins/proxmox/mod.rs)

```rust
use crate::state::plugin::{StatePlugin, StateDiff, ApplyResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProxmoxConfig {
    pub node: String,  // PVE node name
    pub vms: Vec<VmConfig>,
    pub containers: Vec<ContainerConfig>,
    pub storage: Vec<StorageConfig>,
    pub network: Vec<NetworkConfig>,
    pub cluster: Option<ClusterConfig>,
    pub backups: Vec<BackupConfig>,
}

pub struct ProxmoxPlugin {
    api_client: ProxmoxApiClient,
    node: String,
}

impl ProxmoxPlugin {
    pub async fn new(node: String) -> Result<Self> {
        let api_client = ProxmoxApiClient::new(
            "https://localhost:8006",
            std::env::var("PVE_TOKEN_ID")?,
            std::env::var("PVE_TOKEN_SECRET")?,
        ).await?;

        Ok(Self { api_client, node })
    }
}

#[async_trait]
impl StatePlugin for ProxmoxPlugin {
    fn name(&self) -> &str {
        "proxmox"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn is_available(&self) -> bool {
        // Check if Proxmox API is accessible
        std::path::Path::new("/usr/sbin/qm").exists() ||
        std::path::Path::new("/usr/sbin/pct").exists()
    }

    async fn query_current_state(&self) -> Result<serde_json::Value> {
        // Query current state from Proxmox
        let vms = self.api_client.list_vms(&self.node).await?;
        let containers = self.api_client.list_containers(&self.node).await?;
        let storage = self.api_client.list_storage().await?;
        let network = self.read_network_config()?;

        Ok(serde_json::json!({
            "node": self.node,
            "vms": vms,
            "containers": containers,
            "storage": storage,
            "network": network,
        }))
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        let mut actions = Vec::new();

        // Compare VMs
        let current_vms = &current["vms"];
        let desired_vms = &desired["vms"];
        actions.extend(self.diff_vms(current_vms, desired_vms)?);

        // Compare containers
        let current_cts = &current["containers"];
        let desired_cts = &desired["containers"];
        actions.extend(self.diff_containers(current_cts, desired_cts)?);

        // Compare storage
        // Compare network
        // etc.

        Ok(StateDiff {
            plugin: "proxmox".to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: format!("{:x}", md5::compute(current.to_string())),
                desired_hash: format!("{:x}", md5::compute(desired.to_string())),
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    // Create VM or container
                    match self.create_resource(resource, config).await {
                        Ok(change) => changes.push(change),
                        Err(e) => errors.push(format!("Failed to create {}: {}", resource, e)),
                    }
                }
                StateAction::Modify { resource, changes: cfg } => {
                    // Modify VM or container
                    match self.modify_resource(resource, cfg).await {
                        Ok(change) => changes.push(change),
                        Err(e) => errors.push(format!("Failed to modify {}: {}", resource, e)),
                    }
                }
                StateAction::Delete { resource } => {
                    // Delete VM or container
                    match self.delete_resource(resource).await {
                        Ok(change) => changes.push(change),
                        Err(e) => errors.push(format!("Failed to delete {}: {}", resource, e)),
                    }
                }
                StateAction::NoOp { resource } => {
                    // No change needed
                }
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied: changes,
            errors,
            checkpoint: None,
        })
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: true,   // Can restore VM/CT snapshots
            supports_checkpoints: true, // Can create VM/CT snapshots
            supports_verification: true,
            atomic_operations: false,   // VM operations are async
        }
    }
}
```

### Proxmox API Client (src/state/plugins/proxmox/api.rs)

```rust
use reqwest::{Client, header};
use serde_json::Value;

pub struct ProxmoxApiClient {
    client: Client,
    base_url: String,
    token_id: String,
    token_secret: String,
}

impl ProxmoxApiClient {
    pub async fn new(base_url: &str, token_id: String, token_secret: String) -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)  // For self-signed certs
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            token_id,
            token_secret,
        })
    }

    fn auth_header(&self) -> String {
        format!("PVEAPIToken={}={}", self.token_id, self.token_secret)
    }

    pub async fn list_vms(&self, node: &str) -> Result<Vec<Value>> {
        let url = format!("{}/api2/json/nodes/{}/qemu", self.base_url, node);
        let response = self.client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        let data: Value = response.json().await?;
        Ok(data["data"].as_array().unwrap_or(&vec![]).clone())
    }

    pub async fn create_vm(&self, node: &str, vmid: u32, config: &Value) -> Result<()> {
        let url = format!("{}/api2/json/nodes/{}/qemu", self.base_url, node);

        let mut params = serde_json::Map::new();
        params.insert("vmid".to_string(), vmid.into());
        params.insert("name".to_string(), config["name"].clone());
        params.insert("cores".to_string(), config["cores"].clone());
        params.insert("memory".to_string(), config["memory"].clone());
        // ... more params

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create VM: {:?}", response.text().await?));
        }

        Ok(())
    }

    pub async fn start_vm(&self, node: &str, vmid: u32) -> Result<()> {
        let url = format!("{}/api2/json/nodes/{}/qemu/{}/status/start",
                         self.base_url, node, vmid);

        let response = self.client
            .post(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to start VM {}: {:?}",
                                      vmid, response.text().await?));
        }

        Ok(())
    }

    // Similar methods for:
    // - stop_vm, destroy_vm, config_vm
    // - list_containers, create_container, start_container, etc.
    // - list_storage, create_storage, etc.
}
```

## Usage Examples

### Example 1: Declarative VM Management

```json
{
  "version": 1,
  "plugins": {
    "proxmox": {
      "node": "pve",
      "vms": [
        {
          "vmid": 100,
          "name": "web-prod",
          "state": "running",
          "cores": 4,
          "memory": 8192
        }
      ]
    }
  }
}
```

```bash
# Apply state
sudo op-dbus apply proxmox-state.json

# op-dbus will:
# 1. Query current VMs via PVE API
# 2. Calculate diff (VM 100 doesn't exist → create it)
# 3. Create VM 100
# 4. Start VM 100
# 5. Record to blockchain
```

### Example 2: Disaster Recovery

```bash
# Production Proxmox server
prod-pve$ sudo op-dbus discover --export --output prod-pve.json

# Disaster strikes! Rebuild on new hardware
new-pve$ sudo nixos-rebuild switch  # Apply host config
new-pve$ sudo op-dbus apply prod-pve.json

# Result:
# - All VMs recreated with same IDs
# - All containers recreated
# - Storage pools configured
# - Network bridges set up
# - Cluster rejoined (if applicable)
# - Identical Proxmox server in <30 minutes
```

### Example 3: Fleet Management (3 Proxmox Nodes)

```json
{
  "version": 1,
  "plugins": {
    "proxmox": {
      "cluster": {
        "enabled": true,
        "name": "prod-cluster",
        "nodes": [
          {"name": "pve1", "ip": "192.168.1.10"},
          {"name": "pve2", "ip": "192.168.1.11"},
          {"name": "pve3", "ip": "192.168.1.12"}
        ]
      },
      "vms": [
        {
          "vmid": 100,
          "name": "web-prod",
          "node": "pve1",  # Pin to specific node
          "state": "running",
          "ha": {
            "enabled": true,
            "group": "web-servers",
            "max_relocate": 1
          }
        }
      ]
    }
  }
}
```

```bash
# Deploy to all 3 nodes
for node in pve{1..3}; do
  ssh $node "sudo op-dbus apply /shared/cluster-state.json"
done

# All 3 nodes now:
# - Part of same cluster
# - Running designated VMs
# - HA configured
# - Blockchain audit synced
```

## Relation to Existing LXC Plugin

We already have `src/state/plugins/lxc.rs` for standalone LXC. How does it relate?

**Standalone LXC Plugin** (`lxc.rs`):
- Manages LXC containers directly on the host
- Uses `lxc-*` commands
- No Proxmox involvement
- For simple container deployments

**Proxmox Plugin** (`proxmox/containers.rs`):
- Manages LXC containers through Proxmox (`pct`)
- Part of Proxmox VE ecosystem
- Includes VM management, storage, networking
- For enterprise Proxmox deployments

**When to use which**:
- **Standalone LXC**: Simple container host, no VMs
- **Proxmox**: Enterprise VM/container infrastructure, HA, clustering

## Implementation Priority

### Phase 1: MVP (Week 1-2)
- [ ] Proxmox API client
- [ ] VM management (create, start, stop, destroy)
- [ ] Basic query/diff/apply
- [ ] Integration with existing op-dbus

### Phase 2: Full Management (Week 3-4)
- [ ] Container management (pct)
- [ ] Storage management (pvesm)
- [ ] Network management
- [ ] Backup scheduling (vzdump)

### Phase 3: Advanced Features (Week 5-6)
- [ ] Cluster management (pvecm)
- [ ] HA configuration
- [ ] Live migration
- [ ] Resource pools

### Phase 4: Enterprise Features (Week 7-8)
- [ ] Multi-node coordination
- [ ] Backup rotation
- [ ] Performance monitoring integration
- [ ] Compliance reporting

## Benefits

### Before (Manual Proxmox)
- Manual VM creation via Web UI
- No version control
- Configuration drift across nodes
- No audit trail
- Disaster recovery is manual restoration

### After (op-dbus Proxmox Plugin)
- **Declarative**: Define VMs in JSON
- **Version controlled**: Git for VM configs
- **Consistent**: All nodes match desired state
- **Audited**: Blockchain tracks all changes
- **Recoverable**: Rebuild entire Proxmox server from config

## Testing Plan

1. **Unit Tests**: Mock Proxmox API responses
2. **Integration Tests**: Test against real Proxmox VE server
3. **E2E Tests**: Create VM → modify → destroy
4. **Cluster Tests**: Multi-node coordination
5. **DR Tests**: Backup → destroy → restore

## Documentation Needed

- [ ] Proxmox plugin user guide
- [ ] API authentication setup
- [ ] Example configurations
- [ ] Migration guide (manual Proxmox → op-dbus)
- [ ] Disaster recovery procedures
- [ ] Performance tuning (mitigations=off for VMs)

---

**This plugin enables enterprise VM infrastructure management with the same declarative, audited, reproducible approach as the rest of op-dbus.**
