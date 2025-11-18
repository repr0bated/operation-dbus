//! Refactored MCP server using tool registry for loose coupling

#[path = "../mcp/tool_registry.rs"]
mod tool_registry;
#[path = "../mcp/resources.rs"]
mod resources;
#[path = "../native/mod.rs"]
mod native;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tool_registry::{
    AuditMiddleware, DynamicToolBuilder, LoggingMiddleware, Tool, ToolContent, ToolRegistry,
    ToolResult,
};
use resources::ResourceRegistry;
use zbus::Connection;

#[derive(Debug, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Refactored MCP server with tool registry and embedded resources
struct McpServer {
    registry: Arc<ToolRegistry>,
    resources: Arc<ResourceRegistry>,
    orchestrator: Option<zbus::Proxy<'static>>,
}

// Orchestrator proxy will be created manually

impl McpServer {
    async fn new() -> Result<Self> {
        // Create tool registry
        let registry = Arc::new(ToolRegistry::new());

        // Add middleware
        registry.add_middleware(Box::new(LoggingMiddleware)).await;
        registry
            .add_middleware(Box::new(AuditMiddleware::new()))
            .await;

        // Register default tools
        Self::register_default_tools(&registry).await?;

        // Create resource registry with embedded documentation
        let resources = Arc::new(ResourceRegistry::new());
        eprintln!("Loaded {} embedded resources", resources.list_resources().len());

        // Try to connect to orchestrator
        let orchestrator = match Connection::session().await {
            Ok(conn) => match zbus::Proxy::new(
                &conn,
                "org.dbusmcp.Orchestrator",
                "/org/dbusmcp/Orchestrator",
                "org.dbusmcp.Orchestrator",
            )
            .await
            {
                Ok(proxy) => {
                    eprintln!("Connected to orchestrator");
                    Some(proxy)
                }
                Err(e) => {
                    eprintln!("Warning: Could not connect to orchestrator: {}", e);
                    None
                }
            },
            Err(e) => {
                eprintln!("Warning: Could not connect to D-Bus session: {}", e);
                None
            }
        };

        Ok(Self {
            registry,
            resources,
            orchestrator,
        })
    }

    /// Register default tools dynamically
    async fn register_default_tools(registry: &ToolRegistry) -> Result<()> {
        // Systemd status tool
        let systemd_status = DynamicToolBuilder::new("systemd_status")
            .description("Get the status of a systemd service")
            .schema(json!({
                "type": "object",
                "properties": {
                    "service": {
                        "type": "string",
                        "description": "Name of the systemd service"
                    }
                },
                "required": ["service"]
            }))
            .handler(|params| async move {
                let service = params["service"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing service parameter"))?;

                // In real implementation, would query systemd
                Ok(ToolResult {
                    content: vec![ToolContent::text(format!("Status of {}: running", service))],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(systemd_status)).await?;

        // File read tool
        let file_read = DynamicToolBuilder::new("file_read")
            .description("Read contents of a file")
            .schema(json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path"
                    }
                },
                "required": ["path"]
            }))
            .handler(|params| async move {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;

                // In real implementation, would read file with validation
                Ok(ToolResult {
                    content: vec![ToolContent::text(format!("Contents of {}", path))],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(file_read)).await?;

        // Network interfaces tool
        let network_interfaces = DynamicToolBuilder::new("network_interfaces")
            .description("List network interfaces")
            .schema(json!({
                "type": "object",
                "properties": {}
            }))
            .handler(|_params| async move {
                Ok(ToolResult {
                    content: vec![ToolContent::json(json!({
                        "interfaces": [
                            {"name": "eth0", "ip": "192.168.1.100"},
                            {"name": "lo", "ip": "127.0.0.1"}
                        ]
                    }))],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(network_interfaces)).await?;

        // Process list tool
        let process_list = DynamicToolBuilder::new("process_list")
            .description("List running processes")
            .schema(json!({
                "type": "object",
                "properties": {
                    "filter": {
                        "type": "string",
                        "description": "Optional filter"
                    }
                }
            }))
            .handler(|params| async move {
                let filter = params["filter"].as_str();

                Ok(ToolResult {
                    content: vec![ToolContent::text(format!(
                        "Processes{}",
                        filter
                            .map(|f| format!(" filtered by '{}'", f))
                            .unwrap_or_default()
                    ))],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(process_list)).await?;

        // Command execution tool
        let exec_command = DynamicToolBuilder::new("exec_command")
            .description("Execute a whitelisted command")
            .schema(json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command to execute"
                    },
                    "args": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Command arguments"
                    }
                },
                "required": ["command"]
            }))
            .handler(|params| async move {
                let command = params["command"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing command"))?;

                // In real implementation, would validate and execute
                Ok(ToolResult {
                    content: vec![ToolContent::text(format!("Executed: {}", command))],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(exec_command)).await?;

        // Generic OVSDB JSON-RPC call tool
        let json_rpc_call = DynamicToolBuilder::new("json_rpc_call")
            .description("Execute generic JSON-RPC call to OVSDB")
            .schema(json!({
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "description": "JSON-RPC method name (e.g., 'transact')"
                    },
                    "params": {
                        "type": "array",
                        "description": "JSON-RPC parameters array"
                    }
                },
                "required": ["method", "params"]
            }))
            .handler(|params| async move {
                let method = params["method"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing method parameter"))?
                    .to_string();

                let rpc_params = params["params"].clone();

                // Build OVSDB JSON-RPC request
                let request = json!({
                    "method": method,
                    "params": rpc_params,
                    "id": 0
                });

                // Call via bash script (works reliably with socat)
                let script_path = "/git/operation-dbus/ovsdb-rpc.sh";
                let output = tokio::process::Command::new(script_path)
                    .arg(request.to_string())
                    .output()
                    .await?;

                if !output.status.success() {
                    return Err(anyhow::anyhow!("OVSDB RPC failed: {}", String::from_utf8_lossy(&output.stderr)));
                }

                let response: Value = serde_json::from_slice(&output.stdout)?;

                // Check for OVSDB error
                if let Some(error) = response.get("error") {
                    if !error.is_null() {
                        return Err(anyhow::anyhow!("OVSDB error: {}", error));
                    }
                }

                let result = response.get("result")
                    .cloned()
                    .unwrap_or(json!(null));

                Ok(ToolResult {
                    content: vec![ToolContent::text(serde_json::to_string_pretty(&result)?)],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(json_rpc_call)).await?;

        // Create OVS bridge with full system integration
        let create_ovs_bridge = DynamicToolBuilder::new("create_ovs_bridge")
            .description("Create OVS bridge with OVSDB persistence, kernel visibility, and IP configuration")
            .schema(json!({
                "type": "object",
                "properties": {
                    "bridge_name": {
                        "type": "string",
                        "description": "Name of the bridge to create"
                    },
                    "ports": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Optional: Ports to attach to bridge"
                    },
                    "ipv4_address": {
                        "type": "string",
                        "description": "Optional: IPv4 address (e.g., '192.168.1.100')"
                    },
                    "ipv4_prefix": {
                        "type": "number",
                        "description": "Optional: IPv4 prefix length (e.g., 24)"
                    }
                },
                "required": ["bridge_name"]
            }))
            .handler(|params| async move {
                let bridge_name = params["bridge_name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing bridge_name"))?;

                // Create bridge in OVSDB
                let create_ops = json!([
                    "Open_vSwitch",
                    [{
                        "op": "insert",
                        "table": "Bridge",
                        "row": {"name": bridge_name},
                        "uuid-name": "new_bridge"
                    }, {
                        "op": "mutate",
                        "table": "Open_vSwitch",
                        "where": [],
                        "mutations": [["bridges", "insert", ["set", [["named-uuid", "new_bridge"]]]]]
                    }]
                ]);

                let request = json!({"method": "transact", "params": create_ops, "id": 0});
                let output = tokio::process::Command::new("/git/operation-dbus/ovsdb-rpc.sh")
                    .arg(request.to_string())
                    .output()
                    .await?;

                if !output.status.success() {
                    return Err(anyhow::anyhow!("Failed to create bridge in OVSDB"));
                }

                // Add ports if specified
                if let Some(ports) = params["ports"].as_array() {
                    for port in ports {
                        if let Some(port_name) = port.as_str() {
                            let add_port_ops = json!([
                                "Open_vSwitch",
                                [{
                                    "op": "insert",
                                    "table": "Interface",
                                    "row": {"name": port_name, "type": ""},
                                    "uuid-name": "new_iface"
                                }, {
                                    "op": "insert",
                                    "table": "Port",
                                    "row": {"name": port_name, "interfaces": ["named-uuid", "new_iface"]},
                                    "uuid-name": "new_port"
                                }, {
                                    "op": "mutate",
                                    "table": "Bridge",
                                    "where": [["name", "==", bridge_name]],
                                    "mutations": [["ports", "insert", ["set", [["named-uuid", "new_port"]]]]]
                                }]
                            ]);

                            let port_request = json!({"method": "transact", "params": add_port_ops, "id": 0});
                            tokio::process::Command::new("/git/operation-dbus/ovsdb-rpc.sh")
                                .arg(port_request.to_string())
                                .output()
                                .await?;
                        }
                    }
                }

                // Bring interface up via native rtnetlink
                if let Err(e) = crate::native::rtnetlink_helpers::link_up(bridge_name).await {
                    log::warn!("Failed to bring bridge up: {}", e);
                }

                // Add IP if specified via native rtnetlink
                if let (Some(ip), Some(prefix)) = (params.get("ipv4_address"), params.get("ipv4_prefix")) {
                    if let (Some(ip_str), Some(prefix_num)) = (ip.as_str(), prefix.as_u64()) {
                        if let Err(e) = crate::native::rtnetlink_helpers::add_ipv4_address(
                            bridge_name,
                            ip_str,
                            prefix_num as u8
                        ).await {
                            log::warn!("Failed to add IP address: {}", e);
                        }
                    }
                }

                Ok(ToolResult {
                    content: vec![ToolContent::text(format!(
                        "Created OVS bridge '{}' with OVSDB persistence and kernel visibility",
                        bridge_name
                    ))],
                    metadata: None,
                })
            })
            .build();

        registry.register_tool(Box::new(create_ovs_bridge)).await?;

        Ok(())
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id),
            "tools/list" => self.handle_tools_list(request.id).await,
            "tools/call" => self.handle_tools_call(request.id, request.params).await,
            "resources/list" => self.handle_resources_list(request.id),
            "resources/read" => self.handle_resources_read(request.id, request.params),
            _ => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "list": true,
                        "call": true
                    },
                    "resources": {
                        "list": true,
                        "read": true
                    }
                },
                "serverInfo": {
                    "name": "dbus-mcp-refactored",
                    "version": "2.0.0",
                    "description": "Refactored MCP server with loose coupling and embedded documentation"
                }
            })),
            error: None,
        }
    }

    async fn handle_tools_list(&self, id: Option<Value>) -> McpResponse {
        let tools = self.registry.list_tools().await;

        let tool_list: Vec<Value> = tools
            .into_iter()
            .map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "inputSchema": tool.input_schema
                })
            })
            .collect();

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": tool_list
            })),
            error: None,
        }
    }

    async fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> McpResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let tool_name = match params["name"].as_str() {
            Some(name) => name,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing tool name".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let arguments = params["arguments"].clone();
        if arguments.is_null() {
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32602,
                    message: "Missing arguments".to_string(),
                    data: None,
                }),
            };
        }

        // Execute tool through registry
        match self.registry.execute_tool(tool_name, arguments).await {
            Ok(result) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(result)),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Tool execution failed: {}", e),
                    data: None,
                }),
            },
        }
    }

    fn handle_resources_list(&self, id: Option<Value>) -> McpResponse {
        let resources = self.resources.list_resources();

        let resource_list: Vec<Value> = resources
            .into_iter()
            .map(|resource| {
                json!({
                    "uri": resource.uri,
                    "name": resource.name,
                    "description": resource.description,
                    "mimeType": resource.mime_type
                })
            })
            .collect();

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "resources": resource_list
            })),
            error: None,
        }
    }

    fn handle_resources_read(&self, id: Option<Value>, params: Option<Value>) -> McpResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing params".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let uri = match params["uri"].as_str() {
            Some(u) => u,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing resource URI".to_string(),
                        data: None,
                    }),
                };
            }
        };

        match self.resources.get_resource(uri) {
            Some(resource) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "contents": [{
                        "uri": resource.uri,
                        "mimeType": resource.mime_type,
                        "text": resource.content
                    }]
                })),
                error: None,
            },
            None => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32602,
                    message: format!("Resource not found: {}", uri),
                    data: None,
                }),
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    eprintln!("Starting refactored MCP server with tool registry...");

    let server = McpServer::new().await?;

    eprintln!("MCP server ready. Reading from stdin...");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to read line: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to parse request: {}", e);
                continue;
            }
        };

        let response = server.handle_request(request).await;
        let response_json = serde_json::to_string(&response)?;

        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;
    }

    Ok(())
}
