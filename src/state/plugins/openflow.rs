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

/// OpenFlow controller configuration - Policy-based, not interface-based
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFlowConfig {
    /// Bridges managed by this controller
    pub bridges: Vec<BridgeFlowConfig>,

    /// Controller endpoint (tcp:IP:PORT)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller_endpoint: Option<String>,

    /// Flow policies to apply (discovered containers get flows based on policies)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_policies: Option<Vec<FlowPolicy>>,

    /// Enable automatic container discovery and flow generation
    #[serde(default = "default_auto_discover")]
    pub auto_discover_containers: bool,
}

fn default_auto_discover() -> bool {
    true
}

/// Flow policy - Applied to discovered containers/ports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowPolicy {
    /// Policy name
    pub name: String,

    /// Match selector (e.g., "container:*", "container:100-199", "port:internal_*")
    pub selector: String,

    /// Flow template to generate
    pub template: FlowTemplate,
}

/// Flow template for policy-based generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTemplate {
    /// Table to insert flow
    pub table: u8,

    /// Priority
    pub priority: u16,

    /// Actions to perform (can use variables like {container_id}, {port_name})
    pub actions: Vec<FlowAction>,

    /// Additional match fields (beyond the auto-generated in_port match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_matches: Option<HashMap<String, String>>,
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

/// Discovered container from OVSDB introspection
#[derive(Debug, Clone)]
struct DiscoveredContainer {
    /// Container ID
    id: String,

    /// Port name in OVS
    port_name: String,

    /// Bridge this container is attached to
    bridge: String,

    /// OpenFlow port number
    ofport: Option<u16>,
}

/// OpenFlow plugin implementation
pub struct OpenFlowPlugin {
    /// OVSDB client for OVS operations
    ovsdb_client: Arc<crate::native::ovsdb_jsonrpc::OvsdbClient>,
}

impl OpenFlowPlugin {
    pub fn new() -> Self {
        let ovsdb_client = Arc::new(crate::native::ovsdb_jsonrpc::OvsdbClient::new());

        Self { ovsdb_client }
    }

    /// Discover containers from LXC plugin via OVSDB introspection
    async fn discover_containers(&self) -> Result<Vec<DiscoveredContainer>> {
        let mut containers = Vec::new();

        // Get all bridges
        let bridges = self.ovsdb_client.list_bridges().await?;

        for bridge in bridges {
            // Get ports on this bridge
            let ports = self.ovsdb_client.list_bridge_ports(&bridge).await?;

            for port in ports {
                // Check if port matches container pattern (vi{VMID} or internal_{VMID})
                if let Some(container_id) = Self::extract_container_id(&port) {
                    containers.push(DiscoveredContainer {
                        id: container_id.clone(),
                        port_name: port.clone(),
                        bridge: bridge.clone(),
                        ofport: self.get_port_ofport(&port).await.ok(),
                    });
                }
            }
        }

        log::info!(
            "Discovered {} containers from OVS introspection",
            containers.len()
        );
        Ok(containers)
    }

    /// Extract container ID from port name (vi100 -> 100, internal_100 -> 100)
    fn extract_container_id(port_name: &str) -> Option<String> {
        if port_name.starts_with("vi") {
            // Proxmox veth pattern: vi100, vi200
            port_name.strip_prefix("vi").map(|s| s.to_string())
        } else if port_name.starts_with("internal_") {
            // Socket networking pattern: internal_100
            port_name.strip_prefix("internal_").map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Get OpenFlow port number for a port name
    async fn get_port_ofport(&self, port_name: &str) -> Result<u16> {
        let operations = serde_json::json!([{
            "op": "select",
            "table": "Interface",
            "where": [["name", "==", port_name]],
            "columns": ["ofport"]
        }]);

        let result = self.ovsdb_client.transact(operations).await?;

        if let Some(rows) = result[0]["rows"].as_array() {
            if let Some(first_row) = rows.first() {
                if let Some(ofport) = first_row["ofport"].as_i64() {
                    return Ok(ofport as u16);
                }
            }
        }

        Err(anyhow!("Could not find ofport for {}", port_name))
    }

    /// Apply flow policies to discovered containers
    async fn apply_flow_policies(
        &self,
        bridge: &str,
        containers: &[DiscoveredContainer],
        policies: &[FlowPolicy],
    ) -> Result<Vec<FlowEntry>> {
        let mut generated_flows = Vec::new();

        for container in containers {
            for policy in policies {
                if Self::policy_matches(policy, container) {
                    let flow = Self::generate_flow_from_policy(policy, container)?;
                    generated_flows.push(flow);
                    log::debug!(
                        "Generated flow for container {} from policy '{}'",
                        container.id,
                        policy.name
                    );
                }
            }
        }

        log::info!(
            "Generated {} flows for {} containers on bridge {}",
            generated_flows.len(),
            containers.len(),
            bridge
        );

        Ok(generated_flows)
    }

    /// Check if policy selector matches container
    fn policy_matches(policy: &FlowPolicy, container: &DiscoveredContainer) -> bool {
        let selector = &policy.selector;

        if selector.starts_with("container:") {
            let pattern = selector.strip_prefix("container:").unwrap();
            return Self::container_id_matches(pattern, &container.id);
        } else if selector.starts_with("port:") {
            let pattern = selector.strip_prefix("port:").unwrap();
            return Self::port_name_matches(pattern, &container.port_name);
        }

        false
    }

    /// Check if container ID matches pattern (*, 100, 100-199)
    fn container_id_matches(pattern: &str, container_id: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern == container_id {
            return true;
        }

        // Range pattern: 100-199
        if pattern.contains('-') {
            if let Some((start, end)) = pattern.split_once('-') {
                if let (Ok(start_num), Ok(end_num), Ok(id_num)) = (
                    start.parse::<u32>(),
                    end.parse::<u32>(),
                    container_id.parse::<u32>(),
                ) {
                    return id_num >= start_num && id_num <= end_num;
                }
            }
        }

        false
    }

    /// Check if port name matches pattern (internal_*, vi*)
    fn port_name_matches(pattern: &str, port_name: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.ends_with('*') {
            let prefix = pattern.trim_end_matches('*');
            return port_name.starts_with(prefix);
        }

        pattern == port_name
    }

    /// Generate flow from policy template, substituting variables
    fn generate_flow_from_policy(
        policy: &FlowPolicy,
        container: &DiscoveredContainer,
    ) -> Result<FlowEntry> {
        let template = &policy.template;

        // Build match fields
        let mut match_fields = HashMap::new();
        match_fields.insert("in_port".to_string(), container.port_name.clone());

        if let Some(additional) = &template.additional_matches {
            for (k, v) in additional {
                let value = Self::substitute_variables(v, container);
                match_fields.insert(k.clone(), value);
            }
        }

        // Substitute variables in actions
        let actions: Vec<FlowAction> = template
            .actions
            .iter()
            .map(|action| Self::substitute_action_variables(action, container))
            .collect();

        Ok(FlowEntry {
            table: template.table,
            priority: template.priority,
            match_fields,
            actions,
            cookie: Some(container.id.parse::<u64>().unwrap_or(0)),
            idle_timeout: 0,
            hard_timeout: 0,
        })
    }

    /// Substitute variables in string ({container_id}, {port_name}, {bridge})
    fn substitute_variables(text: &str, container: &DiscoveredContainer) -> String {
        text.replace("{container_id}", &container.id)
            .replace("{port_name}", &container.port_name)
            .replace("{bridge}", &container.bridge)
    }

    /// Substitute variables in flow action
    fn substitute_action_variables(
        action: &FlowAction,
        container: &DiscoveredContainer,
    ) -> FlowAction {
        match action {
            FlowAction::Output { port } => FlowAction::Output {
                port: Self::substitute_variables(port, container),
            },
            FlowAction::SetField { field, value } => FlowAction::SetField {
                field: field.clone(),
                value: Self::substitute_variables(value, container),
            },
            FlowAction::LoadRegister { register, value } => {
                // Try to parse {container_id} as numeric value
                let substituted = Self::substitute_variables(&value.to_string(), container);
                let numeric_value = substituted.parse::<u64>().unwrap_or(*value);
                FlowAction::LoadRegister {
                    register: *register,
                    value: numeric_value,
                }
            }
            _ => action.clone(),
        }
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
                    "cookie" => {
                        cookie = Some(u64::from_str_radix(value.trim_start_matches("0x"), 16).ok()?)
                    }
                    "actions" => actions = self.parse_actions(value),
                    _ => {
                        // Match field
                        if !key.contains("duration")
                            && !key.contains("n_packets")
                            && !key.contains("n_bytes")
                        {
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
        log::info!("Querying current OpenFlow state with container discovery");

        // Discover containers via OVSDB introspection
        let discovered_containers = self.discover_containers().await.unwrap_or_default();

        let bridges = self.ovsdb_client.list_bridges().await?;
        let mut bridge_configs = Vec::new();

        for bridge in bridges {
            let flows = self.query_flows(&bridge).await.unwrap_or_default();

            // Convert discovered containers on this bridge to SocketPorts
            let socket_ports: Vec<SocketPort> = discovered_containers
                .iter()
                .filter(|c| c.bridge == bridge)
                .map(|c| SocketPort {
                    name: c.port_name.clone(),
                    container_id: c.id.clone(),
                    ofport: c.ofport.map(|n| n as u16),
                })
                .collect();

            bridge_configs.push(BridgeFlowConfig {
                name: bridge,
                flows,
                socket_ports: if socket_ports.is_empty() {
                    None
                } else {
                    Some(socket_ports)
                },
            });
        }

        let config = OpenFlowConfig {
            bridges: bridge_configs,
            controller_endpoint: None,
            flow_policies: None,
            auto_discover_containers: false,
        };

        Ok(serde_json::to_value(config)?)
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        log::info!("Calculating OpenFlow diff with policy-based flow generation");

        let current_config: OpenFlowConfig = serde_json::from_value(current.clone())?;
        let mut desired_config: OpenFlowConfig = serde_json::from_value(desired.clone())?;

        // If auto-discovery is enabled and policies are defined, generate flows
        if desired_config.auto_discover_containers {
            if let Some(policies) = &desired_config.flow_policies {
                log::info!("Auto-discovery enabled, generating flows from policies");
                let discovered_containers = self.discover_containers().await.unwrap_or_default();

                for bridge_config in &mut desired_config.bridges {
                    // Filter containers for this bridge
                    let bridge_containers: Vec<DiscoveredContainer> = discovered_containers
                        .iter()
                        .filter(|c| c.bridge == bridge_config.name)
                        .cloned()
                        .collect();

                    // Generate flows from policies
                    let policy_flows = self
                        .apply_flow_policies(&bridge_config.name, &bridge_containers, policies)
                        .await?;

                    let policy_count = policy_flows.len();
                    let static_count = bridge_config.flows.len();

                    // Merge policy-generated flows with static flows
                    bridge_config.flows.extend(policy_flows);

                    log::info!(
                        "Bridge {}: {} static flows + {} policy-generated flows",
                        bridge_config.name,
                        static_count,
                        policy_count
                    );
                }
            }
        }

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
                            resource: format!(
                                "{}/flow/{}",
                                desired_bridge.name, desired_flow.table
                            ),
                            config: serde_json::to_value(desired_flow)?,
                        });
                    }
                }

                // Check for flows to delete
                for current_flow in &current_bridge.flows {
                    let flow_desired = desired_bridge.flows.iter().any(|f| f == current_flow);

                    if !flow_desired {
                        actions.push(StateAction::Delete {
                            resource: format!(
                                "{}/flow/{}",
                                desired_bridge.name, current_flow.table
                            ),
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
                                resource: format!(
                                    "{}/port/{}",
                                    desired_bridge.name, desired_port.name
                                ),
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
        log::info!(
            "Applying OpenFlow state changes: {} actions",
            diff.actions.len()
        );

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

                        errors.push(format!(
                            "Flow deletion by resource path needs implementation: {}",
                            resource
                        ));
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
        log::info!(
            "Rolling back OpenFlow to checkpoint from {}",
            checkpoint.timestamp
        );

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
