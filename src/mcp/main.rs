//! Refactored MCP server using tool registry for loose coupling

#[path = "../mcp/tool_registry.rs"]
mod tool_registry;
use tool_registry::{
    AuditMiddleware, DynamicToolBuilder, LoggingMiddleware, Tool, ToolContent, ToolRegistry,
    ToolResult,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
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

/// Refactored MCP server with tool registry
struct McpServer {
    registry: Arc<ToolRegistry>,
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

        // Try to connect to orchestrator
        let orchestrator = match Connection::session().await {
            Ok(conn) => match zbus::Proxy::new(
                &conn,
                "org.dbusmcp.Orchestrator",
                "/org/dbusmcp/Orchestrator",
                "org.dbusmcp.Orchestrator",
            ).await {
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
            .handler(|params| {
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
            .handler(|params| {
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
            .handler(|_params| {
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
            .handler(|params| {
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
            .handler(|params| {
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

        Ok(())
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id),
            "tools/list" => self.handle_tools_list(request.id).await,
            "tools/call" => self.handle_tools_call(request.id, request.params).await,
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
                    }
                },
                "serverInfo": {
                    "name": "dbus-mcp-refactored",
                    "version": "2.0.0",
                    "description": "Refactored MCP server with loose coupling"
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
