//! OVS Tools - OVSDB JSON-RPC based

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use op_network::OvsdbClient;

use op_tools::Tool;
use op_tools::{ToolResult, ToolContext}; // Add missing imports for op-dbus Tool trait
use op_execution_tracker::{ExecutionRecord, ExecutionTiming}; // Add for ExecutionRecord

pub struct OvsTool {
    name: String,
    description: String,
}

impl OvsTool {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

#[async_trait]
impl Tool for OvsTool {
    fn name(&self) -> &'static str {
        // Leak the name to return &'static str if needed, or change implementation.
        // But Tool trait demands &'static str.
        // This dynamic struct approach fails if trait demands static.
        // I will implement concrete structs instead, like builtin.rs does.
        // OR use Box::leak?
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn description(&self) -> &'static str {
        Box::leak(self.description.clone().into_boxed_str())
    }

    fn input_schema(&self) -> Value {
        match self.name.as_str() {
            "ovs_list_bridges" | "network.ovs.bridge.list" => json!({
                "type": "object",
                "properties": {}
            }),
            "ovs_create_bridge" | "network.ovs.bridge.create" => json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Bridge name"}
                },
                "required": ["name"]
            }),
            "ovs_delete_bridge" | "network.ovs.bridge.delete" => json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Bridge name"}
                },
                "required": ["name"]
            }),
            "ovs_add_port" | "network.ovs.port.add" => json!({
                "type": "object",
                "properties": {
                    "bridge": {"type": "string", "description": "Bridge name"},
                    "port": {"type": "string", "description": "Port name"}
                },
                "required": ["bridge", "port"]
            }),
            "ovs_list_ports" | "network.ovs.port.list" => json!({
                "type": "object",
                "properties": {
                    "bridge": {"type": "string", "description": "Bridge name"}
                },
                "required": ["bridge"]
            }),
            _ => json!({"type": "object", "properties": {}})
        }
    }

    fn output_schema(&self) -> Value {
        json!({"type": "object", "properties": {"result": {"type": "any"}}})
    }

    async fn execute(&self, input: Value, ctx: &ToolContext) -> crate::error::Result<ToolResult> {
        let (start, timing) = ExecutionTiming::capture_start();
        
        let client = OvsdbClient::new();
        
        // Helper to convert any error (including Anyhow) to OpDbusError
        let map_err = |e: anyhow::Error| crate::error::OpDbusError::ToolExecution(format!("{}: {}", self.name(), e));
        
        // Map old names to new names if needed, but self.name is what we registered.
        let result_val = match self.name.as_str() {
            "ovs_list_bridges" | "network.ovs.bridge.list" => {
                match client.list_bridges().await {
                    Ok(bridges) => Ok(json!({"bridges": bridges})),
                    Err(e) => Err(map_err(e))
                }
            }
            "ovs_create_bridge" | "network.ovs.bridge.create" => {
                let name = input.get("name").and_then(|n| n.as_str())
                    .ok_or_else(|| crate::error::OpDbusError::ToolExecution(format!("{}: Missing bridge name", self.name())))?;
                match client.create_bridge(name).await {
                    Ok(_) => Ok(json!({"created": name})),
                    Err(e) => Err(map_err(e))
                }
            }
            "ovs_delete_bridge" | "network.ovs.bridge.delete" => {
                let name = input.get("name").and_then(|n| n.as_str())
                    .ok_or_else(|| crate::error::OpDbusError::ToolExecution(format!("{}: Missing bridge name", self.name())))?;
                match client.delete_bridge(name).await {
                    Ok(_) => Ok(json!({"deleted": name})),
                    Err(e) => Err(map_err(e))
                }
            }
            "ovs_list_ports" | "network.ovs.port.list" => {
                let bridge = input.get("bridge").and_then(|b| b.as_str())
                    .ok_or_else(|| crate::error::OpDbusError::ToolExecution(format!("{}: Missing bridge name", self.name())))?;
                match client.list_bridge_ports(bridge).await {
                    Ok(ports) => Ok(json!({"bridge": bridge, "ports": ports})),
                    Err(e) => Err(map_err(e))
                }
            }
            "ovs_add_port" | "network.ovs.port.add" => {
                let bridge = input.get("bridge").and_then(|b| b.as_str())
                    .ok_or_else(|| crate::error::OpDbusError::ToolExecution(format!("{}: Missing bridge name", self.name())))?;
                let port = input.get("port").and_then(|p| p.as_str())
                    .ok_or_else(|| crate::error::OpDbusError::ToolExecution(format!("{}: Missing port name", self.name())))?;
                match client.add_port(bridge, port).await {
                    Ok(_) => Ok(json!({"bridge": bridge, "port_added": port})),
                    Err(e) => Err(map_err(e))
                }
            }
            _ => Ok(json!({"error": "Not implemented"}))
        }?;
        
        let timing = timing.complete(start);

        let record = ExecutionRecord::builder(self.name())
            .input(input)
            .output(result_val.clone())
            .policy_id(&ctx.policy_id)
            .plugin_core_hash(&ctx.plugin.core_hash)
            .tunable_hash(&ctx.plugin.tunable_hash)
            .timing(timing)
            .prev_hash(&ctx.prev_hash)
            .build();

        Ok(ToolResult {
            output: result_val,
            execution_record: record,
        })
    }
}
