use anyhow::Result;
use async_trait::async_trait;
use op_network::OvsdbClient;
use op_state::{
    ApplyResult, Checkpoint, DiffMetadata, PluginCapabilities, StateAction, StateDiff, StatePlugin,
};
use serde::{Deserialize, Serialize};
use simd_json::prelude::*;
use simd_json::OwnedValue as Value;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvsBridgeState {
    pub bridges: Vec<BridgeConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub name: String,
    #[serde(default)]
    pub ports: Vec<String>,
}

pub struct OvsBridgePlugin {
    ovsdb: Arc<OvsdbClient>,
}

impl Default for OvsBridgePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl OvsBridgePlugin {
    pub fn new() -> Self {
        Self {
            ovsdb: Arc::new(OvsdbClient::new()),
        }
    }
}

#[async_trait]
impl StatePlugin for OvsBridgePlugin {
    fn name(&self) -> &str {
        "ovsdb_bridge"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn is_available(&self) -> bool {
        std::path::Path::new("/var/run/openvswitch/db.sock").exists()
    }

    fn unavailable_reason(&self) -> String {
        "OVSDB socket not found at /var/run/openvswitch/db.sock".to_string()
    }

    async fn query_current_state(&self) -> Result<Value> {
        let bridge_names = self.ovsdb.list_bridges().await.unwrap_or_default();

        let mut bridges = Vec::new();
        for name in bridge_names {
            let ports = self.ovsdb.list_bridge_ports(&name).await.unwrap_or_default();
            bridges.push(BridgeConfig { name, ports });
        }

        Ok(simd_json::serde::to_owned_value(OvsBridgeState { bridges })?)
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        let current_state: OvsBridgeState =
            simd_json::serde::from_owned_value(current.clone()).unwrap_or(OvsBridgeState {
                bridges: vec![],
            });
        let desired_state: OvsBridgeState =
            simd_json::serde::from_owned_value(desired.clone()).unwrap_or(OvsBridgeState {
                bridges: vec![],
            });

        let mut actions = Vec::new();

        let current_names: HashSet<&str> =
            current_state.bridges.iter().map(|b| b.name.as_str()).collect();
        let desired_names: HashSet<&str> =
            desired_state.bridges.iter().map(|b| b.name.as_str()).collect();

        // Bridges to create
        for desired_bridge in &desired_state.bridges {
            if !current_names.contains(desired_bridge.name.as_str()) {
                actions.push(StateAction::Create {
                    resource: format!("bridge/{}", desired_bridge.name),
                    config: simd_json::serde::to_owned_value(desired_bridge.clone())?,
                });
            }
        }

        // Bridges to delete
        for current_bridge in &current_state.bridges {
            if !desired_names.contains(current_bridge.name.as_str()) {
                actions.push(StateAction::Delete {
                    resource: format!("bridge/{}", current_bridge.name),
                });
            }
        }

        // Ports to add/remove on existing bridges
        for desired_bridge in &desired_state.bridges {
            if let Some(current_bridge) = current_state
                .bridges
                .iter()
                .find(|b| b.name == desired_bridge.name)
            {
                let current_ports: HashSet<&str> =
                    current_bridge.ports.iter().map(|p| p.as_str()).collect();
                let desired_ports: HashSet<&str> =
                    desired_bridge.ports.iter().map(|p| p.as_str()).collect();

                for port in &desired_ports {
                    if !current_ports.contains(port) {
                        actions.push(StateAction::Create {
                            resource: format!("bridge/{}/port/{}", desired_bridge.name, port),
                            config: simd_json::json!({"bridge": desired_bridge.name, "port": port}),
                        });
                    }
                }

                for port in &current_ports {
                    // Don't delete the bridge's own internal port
                    if !desired_ports.contains(port) && *port != desired_bridge.name.as_str() {
                        actions.push(StateAction::Delete {
                            resource: format!("bridge/{}/port/{}", desired_bridge.name, port),
                        });
                    }
                }
            }
        }

        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: format!("{:x}", md5::compute(simd_json::to_string(current)?)),
                desired_hash: format!("{:x}", md5::compute(simd_json::to_string(desired)?)),
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    if resource.starts_with("bridge/") && !resource.contains("/port/") {
                        // Create bridge
                        let name = config
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        match self.ovsdb.create_bridge(name).await {
                            Ok(_) => {
                                changes_applied.push(format!("Created bridge '{}'", name));
                                // Add ports specified in config
                                if let Some(ports) = config.get("ports").and_then(|v| v.as_array())
                                {
                                    for port_val in ports {
                                        if let Some(port) = port_val.as_str() {
                                            // Skip the internal port (same name as bridge)
                                            if port == name {
                                                continue;
                                            }
                                            match self.ovsdb.add_port(name, port).await {
                                                Ok(_) => changes_applied.push(format!(
                                                    "Added port '{}' to '{}'",
                                                    port, name
                                                )),
                                                Err(e) => errors.push(format!(
                                                    "Failed to add port '{}': {}",
                                                    port, e
                                                )),
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                errors.push(format!("Failed to create bridge '{}': {}", name, e))
                            }
                        }
                    } else if resource.contains("/port/") {
                        // Add port to existing bridge
                        let bridge = config
                            .get("bridge")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let port = config.get("port").and_then(|v| v.as_str()).unwrap_or("");
                        match self.ovsdb.add_port(bridge, port).await {
                            Ok(_) => changes_applied
                                .push(format!("Added port '{}' to '{}'", port, bridge)),
                            Err(e) => errors
                                .push(format!("Failed to add port '{}' to '{}': {}", port, bridge, e)),
                        }
                    }
                }
                StateAction::Delete { resource } => {
                    if resource.starts_with("bridge/") && !resource.contains("/port/") {
                        let name = resource.strip_prefix("bridge/").unwrap_or(resource);
                        match self.ovsdb.delete_bridge(name).await {
                            Ok(_) => changes_applied.push(format!("Deleted bridge '{}'", name)),
                            Err(e) => {
                                errors.push(format!("Failed to delete bridge '{}': {}", name, e))
                            }
                        }
                    }
                    // Port deletion would require an OVSDB delete_port method — skip for now
                }
                _ => {}
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied,
            errors,
            checkpoint: None,
        })
    }

    async fn verify_state(&self, desired: &Value) -> Result<bool> {
        let current = self.query_current_state().await?;
        let diff = self.calculate_diff(&current, desired).await?;
        Ok(diff.actions.is_empty())
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        let state = self.query_current_state().await?;
        Ok(Checkpoint {
            id: uuid::Uuid::new_v4().to_string(),
            plugin: self.name().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: state,
            backend_checkpoint: None,
        })
    }

    async fn rollback(&self, _checkpoint: &Checkpoint) -> Result<()> {
        Ok(())
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: false,
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: true,
        }
    }
}
