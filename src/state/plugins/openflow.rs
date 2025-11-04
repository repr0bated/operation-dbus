// OpenFlow Controller Plugin - Flow-based networking for containerless communication
// Manages OpenFlow flows for socket-based container networking without veth interfaces

use crate::state::plugin::{
    ApplyResult, Checkpoint, DiffMetadata, PluginCapabilities, StateAction, StateDiff, StatePlugin,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// OpenFlow controller configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFlowConfig {
    /// Bridges managed by this controller
    pub bridges: Vec<BridgeFlowConfig>,

    /// Controller endpoint (tcp:IP:PORT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller_endpoint: Option<String>,
}

/// Per-bridge flow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeFlowConfig {
    /// Bridge name (e.g., "ovsbr0")
    pub name: String,

    /// OpenFlow flows for this bridge
    pub flows: Vec<FlowEntry>,

    /// Container socket ports (internal OVS ports for containerless networking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socket_ports: Option<Vec<SocketPort>>,
}

/// OpenFlow flow entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowEntry {
    /// Flow table number (0-254)
    pub table: u8,

    /// Flow priority (0-65535, higher = more specific)
    pub priority: u16,

    /// Match criteria (OpenFlow match fields)
    pub match_fields: HashMap<String, String>,

    /// Actions to perform
    pub actions: Vec<FlowAction>,

    /// Cookie for flow identification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookie: Option<u64>,

    /// Idle timeout in seconds (0 = permanent)
    #[serde(default)]
    pub idle_timeout: u16,

    /// Hard timeout in seconds (0 = permanent)
    #[serde(default)]
    pub hard_timeout: u16,
}

/// OpenFlow actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowAction {
    /// Output to port
    Output { port: String },

    /// Load value into register
    LoadRegister { register: u8, value: u64 },

    /// Resubmit to another table
    Resubmit { table: u8 },

    /// Set field value
    SetField { field: String, value: String },

    /// Drop packet
    Drop,

    /// Send to normal L2 switching
    Normal,

    /// Send to controller
    Controller { max_len: Option<u16> },
}

/// Socket port for containerless networking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketPort {
    /// Port name (e.g., "internal_100" for container 100)
    pub name: String,

    /// Container ID this port serves
    pub container_id: String,

    /// OVS port number (assigned by OVS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ofport: Option<u16>,
}

/// OpenFlow plugin implementation
pub struct OpenFlowPlugin {
    /// OVSDB client for OVS operations
    ovsdb_client: Arc<crate::native::ovsdb_jsonrpc::OvsdbClient>,
}

impl OpenFlowPlugin {
    pub fn new() -> Self {
        let ovsdb_client = Arc::new(
            crate::native::ovsdb_jsonrpc::OvsdbClient::new()
        );

        Self { ovsdb_client }
    }

    /// Install a flow via ovs-ofctl (temporary until native OpenFlow implementation)
    async fn install_flow(&self, bridge: &str, flow: &FlowEntry) -> Result<()> {
        let flow_str = self.flow_to_string(flow);

        log::info!("Installing flow on {}: {}", bridge, flow_str);

        let output = tokio::process::Command::new("ovs-ofctl")
            .args(&["add-flow", bridge, &flow_str])
            .output()
            .await
            .context("Failed to execute ovs-ofctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to install flow: {}", stderr));
        }

        Ok(())
    }

    /// Delete flows via ovs-ofctl
    async fn delete_flow(&self, bridge: &str, flow: &FlowEntry) -> Result<()> {
        let match_str = self.match_to_string(&flow.match_fields);

        log::info!("Deleting flow on {}: table={}, {}", bridge, flow.table, match_str);

        let output = tokio::process::Command::new("ovs-ofctl")
            .args(&[
                "del-flows",
                bridge,
                &format!("table={},{}", flow.table, match_str),
            ])
            .output()
            .await
            .context("Failed to execute ovs-ofctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to delete flow: {}", stderr));
        }

        Ok(())
    }

    /// Query current flows via ovs-ofctl dump-flows
    async fn query_flows(&self, bridge: &str) -> Result<Vec<FlowEntry>> {
        let output = tokio::process::Command::new("ovs-ofctl")
            .args(&["dump-flows", bridge, "--no-stats"])
            .output()
            .await
            .context("Failed to execute ovs-ofctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to query flows: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_flows(&stdout)
    }

    /// Parse ovs-ofctl dump-flows output
    fn parse_flows(&self, output: &str) -> Result<Vec<FlowEntry>> {
        let mut flows = Vec::new();

        for line in output.lines() {
            // Skip header and empty lines
            if line.starts_with("NXST_FLOW") || line.trim().is_empty() {
                continue;
            }

            // Parse flow line
            // Format: cookie=0x0, duration=123s, table=0, n_packets=0, priority=100, in_port=1, actions=output:2
            if let Some(flow) = self.parse_flow_line(line) {
                flows.push(flow);
            }
        }

        Ok(flows)
    }

    /// Parse a single flow line
    fn parse_flow_line(&self, line: &str) -> Option<FlowEntry> {
        let mut table = 0u8;
        let mut priority = 0u16;
        let mut cookie = None;
        let mut match_fields = HashMap::new();
        let mut actions = Vec::new();

        // Split by comma and parse fields
        for part in line.split(',') {
            let part = part.trim();

            if let Some((key, value)) = part.split_once('=') {
                match key.trim() {
                    "table" => table = value.parse().ok()?,
                    "priority" => priority = value.parse().ok()?,
                    "cookie" => cookie = Some(u64::from_str_radix(value.trim_start_matches("0x"), 16).ok()?),
                    "actions" => actions = self.parse_actions(value),
                    _ => {
                        // Match field
                        if !key.contains("duration") && !key.contains("n_packets") && !key.contains("n_bytes") {
                            match_fields.insert(key.to_string(), value.to_string());
                        }
                    }
                }
            }
        }

        Some(FlowEntry {
            table,
            priority,
            match_fields,
            actions,
            cookie,
            idle_timeout: 0,
            hard_timeout: 0,
        })
    }

    /// Parse actions string
    fn parse_actions(&self, actions_str: &str) -> Vec<FlowAction> {
        let mut actions = Vec::new();

        for action in actions_str.split(',') {
            let action = action.trim();

            if action == "NORMAL" {
                actions.push(FlowAction::Normal);
            } else if action == "drop" {
                actions.push(FlowAction::Drop);
            } else if let Some(port) = action.strip_prefix("output:") {
                actions.push(FlowAction::Output {
                    port: port.to_string(),
                });
            } else if let Some(rest) = action.strip_prefix("resubmit(,") {
                if let Some(table_str) = rest.strip_suffix(')') {
                    if let Ok(table) = table_str.parse() {
                        actions.push(FlowAction::Resubmit { table });
                    }
                }
            }
        }

        actions
    }

    /// Convert flow to ovs-ofctl string format
    fn flow_to_string(&self, flow: &FlowEntry) -> String {
        let mut parts = Vec::new();

        // Table
        parts.push(format!("table={}", flow.table));

        // Priority
        parts.push(format!("priority={}", flow.priority));

        // Match fields
        for (key, value) in &flow.match_fields {
            parts.push(format!("{}={}", key, value));
        }

        // Actions
        let actions_str = flow
            .actions
            .iter()
            .map(|a| self.action_to_string(a))
            .collect::<Vec<_>>()
            .join(",");

        format!("{},actions={}", parts.join(","), actions_str)
    }

    /// Convert match fields to string
    fn match_to_string(&self, match_fields: &HashMap<String, String>) -> String {
        match_fields
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Convert action to string
    fn action_to_string(&self, action: &FlowAction) -> String {
        match action {
            FlowAction::Output { port } => format!("output:{}", port),
            FlowAction::LoadRegister { register, value } => {
                format!("load:{}->NXM_NX_REG{}[]", value, register)
            }
            FlowAction::Resubmit { table } => format!("resubmit(,{})", table),
            FlowAction::SetField { field, value } => format!("set_field:{}={}", value, field),
            FlowAction::Drop => "drop".to_string(),
            FlowAction::Normal => "NORMAL".to_string(),
            FlowAction::Controller { max_len } => {
                if let Some(len) = max_len {
                    format!("CONTROLLER:{}", len)
                } else {
                    "CONTROLLER".to_string()
                }
            }
        }
    }

    /// Create OVS internal port for socket networking
    async fn create_socket_port(&self, bridge: &str, port: &SocketPort) -> Result<()> {
        log::info!(
            "Creating socket port {} on {} for container {}",
            port.name,
            bridge,
            port.container_id
        );

        // Add internal port to OVS bridge
        self.ovsdb_client.add_port(bridge, &port.name).await?;

        // Set port type to internal
        let output = tokio::process::Command::new("ovs-vsctl")
            .args(&["set", "interface", &port.name, "type=internal"])
            .output()
            .await
            .context("Failed to set port type")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to set port type: {}", stderr));
        }

        Ok(())
    }

    /// Delete socket port
    async fn delete_socket_port(&self, bridge: &str, port_name: &str) -> Result<()> {
        log::info!("Deleting socket port {} from {}", port_name, bridge);

        // Use OVSDB transact to delete port
        let port_uuid = self.find_port_uuid(bridge, port_name).await?;
        let bridge_uuid = self.find_bridge_uuid_by_name(bridge).await?;

        let operations = serde_json::json!([
            {
                "op": "mutate",
                "table": "Bridge",
                "where": [["_uuid", "==", ["uuid", &bridge_uuid]]],
                "mutations": [
                    ["ports", "delete", ["uuid", &port_uuid]]
                ]
            },
            {
                "op": "delete",
                "table": "Port",
                "where": [["_uuid", "==", ["uuid", &port_uuid]]]
            }
        ]);

        self.ovsdb_client.transact(operations).await?;
        Ok(())
    }

    /// Find port UUID by name on a bridge
    async fn find_port_uuid(&self, _bridge: &str, port_name: &str) -> Result<String> {
        let operations = serde_json::json!([{
            "op": "select",
            "table": "Port",
            "where": [["name", "==", port_name]],
            "columns": ["_uuid"]
        }]);

        let result = self.ovsdb_client.transact(operations).await?;

        if let Some(rows) = result[0]["rows"].as_array() {
            if let Some(first_row) = rows.first() {
                if let Some(uuid_array) = first_row["_uuid"].as_array() {
                    if uuid_array.len() == 2 && uuid_array[0] == "uuid" {
                        return Ok(uuid_array[1].as_str().unwrap().to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Port '{}' not found", port_name))
    }

    /// Find bridge UUID by name
    async fn find_bridge_uuid_by_name(&self, bridge_name: &str) -> Result<String> {
        let operations = serde_json::json!([{
            "op": "select",
            "table": "Bridge",
            "where": [["name", "==", bridge_name]],
            "columns": ["_uuid"]
        }]);

        let result = self.ovsdb_client.transact(operations).await?;

        if let Some(rows) = result[0]["rows"].as_array() {
            if let Some(first_row) = rows.first() {
                if let Some(uuid_array) = first_row["_uuid"].as_array() {
                    if uuid_array.len() == 2 && uuid_array[0] == "uuid" {
                        return Ok(uuid_array[1].as_str().unwrap().to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Bridge '{}' not found", bridge_name))
    }

    /// Compute SHA-256 hash of state
    fn compute_state_hash(&self, state: &Value) -> String {
        use sha2::{Digest, Sha256};
        let json_str = serde_json::to_string(state).unwrap_or_default();
        format!("{:x}", Sha256::digest(json_str.as_bytes()))
    }
}

#[async_trait]
impl StatePlugin for OpenFlowPlugin {
    fn name(&self) -> &str {
        "openflow"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn query_current_state(&self) -> Result<Value> {
        log::info!("Querying current OpenFlow state");

        let bridges = self.ovsdb_client.list_bridges().await?;
        let mut bridge_configs = Vec::new();

        for bridge in bridges {
            let flows = self.query_flows(&bridge).await.unwrap_or_default();

            bridge_configs.push(BridgeFlowConfig {
                name: bridge,
                flows,
                socket_ports: None, // TODO: Query socket ports
            });
        }

        let config = OpenFlowConfig {
            bridges: bridge_configs,
            controller_endpoint: None,
        };

        Ok(serde_json::to_value(config)?)
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        log::info!("Calculating OpenFlow diff");

        let current_config: OpenFlowConfig = serde_json::from_value(current.clone())?;
        let desired_config: OpenFlowConfig = serde_json::from_value(desired.clone())?;

        let mut actions = Vec::new();

        // Compare bridges
        for desired_bridge in &desired_config.bridges {
            let current_bridge = current_config
                .bridges
                .iter()
                .find(|b| b.name == desired_bridge.name);

            if let Some(current_bridge) = current_bridge {
                // Compare flows
                for desired_flow in &desired_bridge.flows {
                    let flow_exists = current_bridge.flows.iter().any(|f| f == desired_flow);

                    if !flow_exists {
                        actions.push(StateAction::Create {
                            resource: format!("{}/flow/{}", desired_bridge.name, desired_flow.table),
                            config: serde_json::to_value(desired_flow)?,
                        });
                    }
                }

                // Check for flows to delete
                for current_flow in &current_bridge.flows {
                    let flow_desired = desired_bridge.flows.iter().any(|f| f == current_flow);

                    if !flow_desired {
                        actions.push(StateAction::Delete {
                            resource: format!("{}/flow/{}", desired_bridge.name, current_flow.table),
                        });
                    }
                }

                // Compare socket ports
                if let (Some(desired_ports), Some(current_ports)) =
                    (&desired_bridge.socket_ports, &current_bridge.socket_ports)
                {
                    for desired_port in desired_ports {
                        let port_exists = current_ports.iter().any(|p| p.name == desired_port.name);

                        if !port_exists {
                            actions.push(StateAction::Create {
                                resource: format!("{}/port/{}", desired_bridge.name, desired_port.name),
                                config: serde_json::to_value(desired_port)?,
                            });
                        }
                    }
                }
            }
        }

        let current_state = self.query_current_state().await?;
        let current_hash = self.compute_state_hash(&current_state);
        let desired_hash = self.compute_state_hash(&serde_json::to_value(desired)?);

        Ok(StateDiff {
            plugin: "openflow".to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash,
                desired_hash,
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        log::info!("Applying OpenFlow state changes: {} actions", diff.actions.len());

        let mut changes = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    if resource.contains("/flow/") {
                        // Install flow
                        let parts: Vec<&str> = resource.split('/').collect();
                        let bridge = parts[0];
                        let flow: FlowEntry = serde_json::from_value(config.clone())?;

                        match self.install_flow(bridge, &flow).await {
                            Ok(_) => changes.push(format!("Installed flow on {}", bridge)),
                            Err(e) => errors.push(format!("Failed to install flow: {}", e)),
                        }
                    } else if resource.contains("/port/") {
                        // Create socket port
                        let parts: Vec<&str> = resource.split('/').collect();
                        let bridge = parts[0];
                        let port: SocketPort = serde_json::from_value(config.clone())?;

                        match self.create_socket_port(bridge, &port).await {
                            Ok(_) => changes.push(format!("Created socket port {}", port.name)),
                            Err(e) => errors.push(format!("Failed to create port: {}", e)),
                        }
                    }
                }
                StateAction::Delete { resource } => {
                    if resource.contains("/flow/") {
                        // Delete flow - parse flow info from resource string
                        let parts: Vec<&str> = resource.split('/').collect();
                        let _bridge = parts[0];
                        // For deletion, we need to query current flows and match by table
                        let _table_str = parts.get(2).unwrap_or(&"0");

                        errors.push(format!("Flow deletion by resource path needs implementation: {}", resource));
                    } else if resource.contains("/port/") {
                        // Delete socket port
                        let parts: Vec<&str> = resource.split('/').collect();
                        let bridge = parts[0];
                        let port_name = parts[2];

                        match self.delete_socket_port(bridge, port_name).await {
                            Ok(_) => changes.push(format!("Deleted socket port {}", port_name)),
                            Err(e) => errors.push(format!("Failed to delete port: {}", e)),
                        }
                    }
                }
                StateAction::Modify { .. } => {
                    // Flow modification = delete + create
                    errors.push("Flow modification not yet implemented".to_string());
                }
                StateAction::NoOp { .. } => {
                    // No operation needed
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

    async fn verify_state(&self, desired: &Value) -> Result<bool> {
        let current = self.query_current_state().await?;
        Ok(current == *desired)
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        let current_state = self.query_current_state().await?;

        Ok(Checkpoint {
            id: format!("openflow_{}", chrono::Utc::now().timestamp()),
            plugin: "openflow".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: current_state,
            backend_checkpoint: None,
        })
    }

    async fn rollback(&self, checkpoint: &Checkpoint) -> Result<()> {
        log::info!("Rolling back OpenFlow to checkpoint from {}", checkpoint.timestamp);

        let current = self.query_current_state().await?;
        let diff = self
            .calculate_diff(&current, &checkpoint.state_snapshot)
            .await?;

        self.apply_state(&diff).await?;

        Ok(())
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: true,
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: false, // Flows installed one by one
        }
    }
}
