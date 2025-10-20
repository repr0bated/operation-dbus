//! LXC plugin - read-only introspection of LXC containers on the host.
//!
//! Design
//! - Discovers LXC containers by correlating OVS ports (vi{VMID}) with OVS bridges
//!   and known cgroup paths on Proxmox (pve-container@{VMID}.service) for running state.
//! - No CLI; native OVSDB JSON-RPC and filesystem reads only.
//! - Read-only for now (apply is a no-op). StateManager will still produce footprints on apply.

use crate::state::plugin::{ApplyResult, Checkpoint, DiffMetadata, PluginCapabilities, StateAction, StateDiff, StatePlugin};
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
    pub properties: Option<HashMap<String, Value>>, // extensible
}

pub struct LxcPlugin;

impl LxcPlugin {
    pub fn new() -> Self { Self }

    fn is_running(ct_id: &str) -> Option<bool> {
        // Proxmox systemd service path: pve-container@{vmid}.service (cgroup v2)
        let path = format!("/sys/fs/cgroup/system.slice/pve-container@{}.service", ct_id);
        Some(fs::metadata(path).is_ok())
    }

    async fn discover_from_ovs(&self) -> Result<Vec<ContainerInfo>> {
        let client = crate::native::OvsdbClient::new();
        // If OVSDB is not reachable, return empty list
        if client.list_dbs().await.is_err() { return Ok(Vec::new()); }

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

impl Default for LxcPlugin { fn default() -> Self { Self::new() } }

#[async_trait]
impl StatePlugin for LxcPlugin {
    fn name(&self) -> &str { "lxc" }
    fn version(&self) -> &str { "1.0.0" }

    async fn query_current_state(&self) -> Result<Value> {
        let containers = self.discover_from_ovs().await?;
        Ok(serde_json::to_value(LxcState { containers })?)
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        // For now, emit a single modify if different; once lifecycle is defined, compute granular actions.
        let actions = if current != desired {
            vec![StateAction::Modify { resource: "lxc".into(), changes: desired.clone() }]
        } else { vec![] };
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

    async fn apply_state(&self, _diff: &StateDiff) -> Result<ApplyResult> {
        // Read-only placeholder; lifecycle ops (create/start/stop) can be added later using native APIs.
        Ok(ApplyResult { success: true, changes_applied: vec!["read-only".into()], errors: vec![], checkpoint: None })
    }

    async fn verify_state(&self, _desired: &Value) -> Result<bool> { Ok(true) }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        Ok(Checkpoint { id: format!("lxc-{}", chrono::Utc::now().timestamp()), plugin: self.name().into(), timestamp: chrono::Utc::now().timestamp(), state_snapshot: json!({}), backend_checkpoint: None })
    }

    async fn rollback(&self, _checkpoint: &Checkpoint) -> Result<()> { Ok(()) }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities { supports_rollback: false, supports_checkpoints: false, supports_verification: false, atomic_operations: false }
    }
}

