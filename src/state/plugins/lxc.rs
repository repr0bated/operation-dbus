//! LXC plugin - read-only introspection of LXC containers on the host.
//!
//! Design
//! - Discovers LXC containers by correlating OVS ports (vi{VMID}) with OVS bridges
//!   and known cgroup paths on Proxmox (pve-container@{VMID}.service) for running state.
//! - No CLI; native OVSDB JSON-RPC and filesystem reads only.
//! - Read-only for now (apply is a no-op). StateManager will still produce footprints on apply.

use crate::state::plugin::{
    ApplyResult, Checkpoint, DiffMetadata, PluginCapabilities, StateAction, StateDiff, StatePlugin,
};
use crate::state::plugtree::PlugTree;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LxcState {
    pub containers: Vec<ContainerInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContainerInfo {
    pub id: String,
    pub veth: String,
    pub bridge: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>, // extensible (includes network_type, template, etc.)
}

pub struct LxcPlugin;

impl LxcPlugin {
    pub fn new() -> Self {
        Self
    }

    /// Apply state for a single container
    pub async fn apply_container_state(&self, container: &ContainerInfo) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        // Check if container exists
        let current_containers = self.discover_from_ovs().await?;
        let exists = current_containers.iter().any(|c| c.id == container.id);

        if !exists {
            // Create container
            match Self::create_container(container).await {
                Ok(_) => {
                    changes_applied.push(format!("Created container {}", container.id));
                    
                    // Start it
                    if let Err(e) = Self::start_container(&container.id).await {
                        errors.push(format!("Failed to start container {}: {}", container.id, e));
                    } else {
                        changes_applied.push(format!("Started container {}", container.id));
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to create container {}: {}", container.id, e));
                }
            }
        } else {
            changes_applied.push(format!("Container {} already exists", container.id));
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied,
            errors,
            checkpoint: None,
        })
    }

    fn is_running(ct_id: &str) -> Option<bool> {
        // Proxmox systemd service path: pve-container@{vmid}.service (cgroup v2)
        let path = format!(
            "/sys/fs/cgroup/system.slice/pve-container@{}.service",
            ct_id
        );
        Some(fs::metadata(path).is_ok())
    }

    async fn discover_from_ovs(&self) -> Result<Vec<ContainerInfo>> {
        let client = crate::native::OvsdbClient::new();
        // If OVSDB is not reachable, return empty list
        if client.list_dbs().await.is_err() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let bridges = client.list_bridges().await.unwrap_or_default();
        for br in bridges {
            let ports = client.list_bridge_ports(&br).await.unwrap_or_default();
            for p in ports {
                if let Some(ct_id) = p.strip_prefix("vi") {
                    // ensure ID is numeric-like
                    if ct_id.chars().all(|c| c.is_ascii_digit()) {
                        let running = Self::is_running(ct_id);
                        results.push(ContainerInfo {
                            id: ct_id.to_string(),
                            veth: p.clone(),
                            bridge: br.clone(),
                            running,
                            properties: None,
                        });
                    }
                }
            }
        }
        Ok(results)
    }
}

impl Default for LxcPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlugTree for LxcPlugin {
    fn pluglet_type(&self) -> &str {
        "container"
    }

    fn pluglet_id_field(&self) -> &str {
        "id"
    }

    fn extract_pluglet_id(&self, resource: &Value) -> Result<String> {
        resource
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Container missing 'id' field"))
    }

    async fn apply_pluglet(&self, pluglet_id: &str, desired: &Value) -> Result<ApplyResult> {
        let container: ContainerInfo = serde_json::from_value(desired.clone())?;
        self.apply_container_state(&container).await
    }

    async fn query_pluglet(&self, pluglet_id: &str) -> Result<Option<Value>> {
        let containers = self.discover_from_ovs().await?;
        
        for container in containers {
            if container.id == pluglet_id {
                return Ok(Some(serde_json::to_value(container)?));
            }
        }
        
        Ok(None)
    }

    async fn list_pluglet_ids(&self) -> Result<Vec<String>> {
        let containers = self.discover_from_ovs().await?;
        Ok(containers.into_iter().map(|c| c.id).collect())
    }
}

impl LxcPlugin {
    /// Find container's veth interface name
    async fn find_container_veth(ct_id: &str) -> Result<String> {
        // Container network namespace path
        let _netns_path = format!("/run/netns/ct{}", ct_id);

        // List all interfaces and find veth peer
        let output = tokio::process::Command::new("ip")
            .args([
                "netns",
                "exec",
                &format!("ct{}", ct_id),
                "ip",
                "link",
                "show",
            ])
            .output()
            .await?;

        if !output.status.success() {
            // Try alternative: check host for veth pairs
            let output = tokio::process::Command::new("ip")
                .args(["link", "show", "type", "veth"])
                .output()
                .await?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            // Parse output to find veth matching container
            // Format: "vethXXX@if" - we want the host side
            for line in stdout.lines() {
                if line.contains("@if") && line.contains("veth") {
                    if let Some(name) = line.split(':').nth(1) {
                        return Ok(name.split('@').next().unwrap_or("").trim().to_string());
                    }
                }
            }
            return Err(anyhow::anyhow!(
                "Could not find veth for container {}",
                ct_id
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("eth0") {
                // This is the container's eth0, find its host peer
                // For now, assume naming pattern vethXXX
                return Ok(format!("veth{}", ct_id));
            }
        }

        Err(anyhow::anyhow!(
            "Could not determine veth name for container {}",
            ct_id
        ))
    }

    /// Determine bridge based on network type
    fn get_bridge_for_network_type(container: &ContainerInfo) -> String {
        let network_type = container
            .properties
            .as_ref()
            .and_then(|p| p.get("network_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("bridge");

        match network_type {
            "netmaker" => "mesh".to_string(),     // Netmaker mesh bridge
            "bridge" => container.bridge.clone(), // Traditional bridge (vmbr0)
            _ => container.bridge.clone(),
        }
    }

    /// Create LXC container via pct (Proxmox)
    async fn create_container(container: &ContainerInfo) -> Result<()> {
        log::info!("Creating LXC container {}", container.id);

        // Select bridge based on network type
        let bridge = Self::get_bridge_for_network_type(container);
        log::info!("Container {} will use bridge {}", container.id, bridge);

        // Get template from properties or use default
        let template = container
            .properties
            .as_ref()
            .and_then(|p| p.get("template"))
            .and_then(|v| v.as_str())
            .unwrap_or("local-btrfs:vztmpl/debian-13-netmaker_custom.tar.zst");

        log::info!("Using template: {}", template);

        // Use pct create (Proxmox Container Toolkit)
        let output = tokio::process::Command::new("pct")
            .args([
                "create",
                &container.id,
                template,
                "--hostname",
                &format!("ct{}", container.id),
                "--memory",
                "512",
                "--swap",
                "512",
                "--rootfs",
                "local-btrfs:8",
                "--net0",
                &format!("name=eth0,bridge={},firewall=1", bridge),
                "--unprivileged",
                "1",
                "--features",
                "nesting=1",
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("pct create failed: {}", stderr));
        }

        log::info!(
            "Container {} created successfully on bridge {}",
            container.id,
            bridge
        );

        // Inject netmaker token for first-boot join (if netmaker network type)
        let network_type = container
            .properties
            .as_ref()
            .and_then(|p| p.get("network_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("bridge");

        if network_type == "netmaker" {
            // Read token from host
            if let Ok(token) = tokio::fs::read_to_string("/etc/op-dbus/netmaker.env").await {
                // Parse NETMAKER_TOKEN=xxx from env file
                for line in token.lines() {
                    if let Some(token_value) = line.strip_prefix("NETMAKER_TOKEN=") {
                        let token_clean = token_value.trim_matches('"').trim();
                        
                        // Write token to container's rootfs
                        let rootfs_path = format!("/var/lib/lxc/{}/rootfs/etc/netmaker-token", container.id);
                        if tokio::fs::write(&rootfs_path, token_clean).await.is_ok() {
                            log::info!("Injected netmaker token into container {}", container.id);
                        }
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Start LXC container
    async fn start_container(ct_id: &str) -> Result<()> {
        let output = tokio::process::Command::new("pct")
            .args(["start", ct_id])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("pct start failed: {}", stderr));
        }

        Ok(())
    }
}

#[async_trait]
impl StatePlugin for LxcPlugin {
    fn name(&self) -> &str {
        "lxc"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn query_current_state(&self) -> Result<Value> {
        let containers = self.discover_from_ovs().await?;
        Ok(serde_json::to_value(LxcState { containers })?)
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        // For now, emit a single modify if different; once lifecycle is defined, compute granular actions.
        let actions = if current != desired {
            vec![StateAction::Modify {
                resource: "lxc".into(),
                changes: desired.clone(),
            }]
        } else {
            vec![]
        };
        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: format!("{:x}", md5::compute(serde_json::to_string(current)?)),
                desired_hash: format!("{:x}", md5::compute(serde_json::to_string(desired)?)),
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create {
                    resource: _,
                    config,
                } => {
                    let container: ContainerInfo = serde_json::from_value(config.clone())?;

                    // 1. Create LXC container
                    match Self::create_container(&container).await {
                        Ok(_) => {
                            changes_applied.push(format!("Created container {}", container.id));

                            // 2. Start container to create veth interface
                            if let Err(e) = Self::start_container(&container.id).await {
                                errors.push(format!(
                                    "Failed to start container {}: {}",
                                    container.id, e
                                ));
                                continue;
                            }

                            // Wait for veth to appear
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                            // 3. Find and rename veth
                            let veth_name = format!("vi{}", container.id);
                            match Self::find_container_veth(&container.id).await {
                                Ok(old_veth) => {
                                    log::info!(
                                        "Found veth {} for container {}",
                                        old_veth,
                                        container.id
                                    );

                                    match crate::native::rtnetlink_helpers::link_set_name(
                                        &old_veth, &veth_name,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            changes_applied.push(format!(
                                                "Renamed {} to {}",
                                                old_veth, veth_name
                                            ));

                                            // 4. Network enrollment based on type
                                            let network_type = container
                                                .properties
                                                .as_ref()
                                                .and_then(|p| p.get("network_type"))
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("bridge");

                                            match network_type {
                                                "netmaker" => {
                                                    // Add to mesh (netmaker mesh bridge)
                                                    let client = crate::native::OvsdbClient::new();
                                                    match client.add_port("mesh", &veth_name).await
                                                    {
                                                        Ok(_) => {
                                                            log::info!(
                                                                "Container {} on mesh bridge",
                                                                container.id
                                                            );
                                                            changes_applied.push(format!(
                                                                "Added {} to mesh (netmaker bridge)",
                                                                veth_name
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            errors.push(format!(
                                                                "Failed to add {} to mesh: {}",
                                                                veth_name, e
                                                            ));
                                                        }
                                                    }
                                                }
                                                "bridge" => {
                                                    // Add to traditional bridge (vmbr0)
                                                    let client = crate::native::OvsdbClient::new();
                                                    match client
                                                        .add_port(&container.bridge, &veth_name)
                                                        .await
                                                    {
                                                        Ok(_) => {
                                                            changes_applied.push(format!(
                                                                "Added {} to bridge {}",
                                                                veth_name, container.bridge
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            errors.push(format!(
                                                                "Failed to add port to bridge: {}",
                                                                e
                                                            ));
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    // Default to traditional bridge
                                                    let client = crate::native::OvsdbClient::new();
                                                    match client
                                                        .add_port(&container.bridge, &veth_name)
                                                        .await
                                                    {
                                                        Ok(_) => {
                                                            changes_applied.push(format!(
                                                                "Added {} to bridge {}",
                                                                veth_name, container.bridge
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            errors.push(format!(
                                                                "Failed to add port to bridge: {}",
                                                                e
                                                            ));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            errors.push(format!("Failed to rename veth: {}", e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.push(format!(
                                        "Failed to find veth for container {}: {}",
                                        container.id, e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            errors.push(format!(
                                "Failed to create container {}: {}",
                                container.id, e
                            ));
                        }
                    }
                }
                StateAction::Modify {
                    resource,
                    changes: _,
                } => {
                    // Handle container state changes (start/stop)
                    log::info!(
                        "Modify operation for container {} (not yet implemented)",
                        resource
                    );
                    changes_applied.push(format!("Skipped modify for {}", resource));
                }
                StateAction::Delete { resource } => {
                    // Delete container
                    log::info!("Deleting container {}", resource);
                    let output = tokio::process::Command::new("pct")
                        .args(["destroy", resource])
                        .output()
                        .await;

                    match output {
                        Ok(out) if out.status.success() => {
                            changes_applied.push(format!("Deleted container {}", resource));
                        }
                        _ => {
                            errors.push(format!("Failed to delete container {}", resource));
                        }
                    }
                }
                StateAction::NoOp { .. } => {}
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied,
            errors,
            checkpoint: None,
        })
    }

    async fn verify_state(&self, _desired: &Value) -> Result<bool> {
        Ok(true)
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        Ok(Checkpoint {
            id: format!("lxc-{}", chrono::Utc::now().timestamp()),
            plugin: self.name().into(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: json!({}),
            backend_checkpoint: None,
        })
    }

    async fn rollback(&self, _checkpoint: &Checkpoint) -> Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: false,
            supports_checkpoints: false,
            supports_verification: false,
            atomic_operations: false,
        }
    }
}
