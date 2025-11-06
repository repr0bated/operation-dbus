//! Refactored MCP server using tool registry for loose coupling

use op_dbus::mcp::tool_registry;
use op_dbus::mcp::introspection_tools;

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
        // Register introspection tools (real implementations)
        introspection_tools::register_introspection_tools(registry).await?;

        // Keep a few basic tools for development
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
                Box::pin(async move {
                    let service = params["service"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing service parameter"))?;

                    // Query systemd via systemctl
                    let output = tokio::process::Command::new("systemctl")
                        .arg("status")
                        .arg(service)
                        .output()
                        .await?;

                    let status = String::from_utf8_lossy(&output.stdout);
                    Ok(ToolResult {
                        content: vec![ToolContent::text(status.to_string())],
                        metadata: None,
                    })
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
                Box::pin(async move {
                    let path = params["path"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;

                    // Read file with tokio
                    let contents = tokio::fs::read_to_string(path).await?;
                    Ok(ToolResult {
                        content: vec![ToolContent::text(contents)],
                        metadata: None,
                    })
                })
            })
            .build();

        registry.register_tool(Box::new(file_read)).await?;

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
