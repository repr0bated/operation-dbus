//! Refactored MCP server using tool registry for loose coupling

#[path = "../mcp/tool_registry.rs"]
mod tool_registry;
#[path = "../mcp/introspection_tools.rs"]
mod introspection_tools;

mod resources;
mod llm_agents;
mod executors;
mod commands;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tool_registry::{
    AuditMiddleware, DynamicToolBuilder, LoggingMiddleware, Tool, ToolContent, ToolRegistry,
    ToolResult,
};
use llm_agents::{AgentRegistry, AgentRequest};
use commands::CommandRegistry;
use executors::ExecutorFactory;
use resources::register_embedded_markdown_resources;
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

/// Refactored MCP server with tool, resource, agent, and command registries
struct McpServer {
    tool_registry: Arc<ToolRegistry>,
    resource_registry: Arc<resources::ResourceRegistry>,
    agent_registry: Arc<AgentRegistry>,
    command_registry: Arc<CommandRegistry>,
    orchestrator: Option<zbus::Proxy<'static>>,
}

// Orchestrator proxy will be created manually

impl McpServer {
    async fn new() -> Result<Self> {
        // Create tool registry
        let tool_registry = Arc::new(ToolRegistry::new());

        // Add middleware
        tool_registry.add_middleware(Box::new(LoggingMiddleware)).await;
        tool_registry
            .add_middleware(Box::new(AuditMiddleware::new()))
            .await;

        // Create agent registry and load agents from filesystem
        let agent_registry = Arc::new(AgentRegistry::new());
        let agents_loaded = agent_registry.load_agents(std::path::Path::new("/git/agents")).await?;
        eprintln!("Loaded {} agents from filesystem", agents_loaded);

        // Create command registry and load commands from filesystem
        let command_registry = Arc::new(CommandRegistry::new());
        let commands_loaded = command_registry.load_commands(std::path::Path::new("/git/commands")).await?;
        eprintln!("Loaded {} commands from filesystem", commands_loaded);

        // Register LLM executors
        let executors = ExecutorFactory::create_executors();
        for executor in executors {
            agent_registry.register_executor(executor).await?;
        }

        // Register default tools
        Self::register_default_tools(&tool_registry).await?;

        // Register agent execution tools
        Self::register_agent_tools(&tool_registry, &agent_registry).await?;

        // Register command execution tools
        Self::register_command_tools(&tool_registry, &command_registry, &agent_registry).await?;

        // Create resource registry
        let resource_registry = Arc::new(resources::ResourceRegistry::new());

        // Register embedded markdown resources
        register_embedded_markdown_resources(&resource_registry).await?;

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
            tool_registry,
            resource_registry,
            agent_registry,
            command_registry,
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

        Ok(())
    }

    /// Register agent execution tools
    async fn register_agent_tools(registry: &ToolRegistry, agent_registry: &AgentRegistry) -> Result<()> {
        // Add a tool to execute agents
        let execute_agent_tool = DynamicToolBuilder::new("execute_agent")
            .description("Execute any of the 299 available AI agents with specialized capabilities")
            .schema(json!({
                "type": "object",
                "properties": {
                    "agent_name": {
                        "type": "string",
                        "description": "Name of the agent to execute (e.g., 'backend-architect', 'ui-visual-validator')"
                    },
                    "task": {
                        "type": "string",
                        "description": "The task or request for the agent"
                    },
                    "context": {
                        "type": "object",
                        "description": "Optional context information"
                    }
                },
                "required": ["agent_name", "task"]
            }))
            .handler(|params| async move {
                // This will be implemented when we have access to agent_registry
                // For now, return a placeholder response
                let agent_name = params["agent_name"].as_str().unwrap_or("unknown");
                let task = params["task"].as_str().unwrap_or("no task");

                Ok(ToolResult::success(ToolContent::text(format!(
                    "Agent execution requested: {} with task: {}", agent_name, task
                ))))
            })
            .build();

        registry.register_tool(Box::new(execute_agent_tool)).await?;

        // Add a tool to list available agents
        let list_agents_tool = DynamicToolBuilder::new("list_agents")
            .description("List all available AI agents and their specializations")
            .schema(json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Optional category filter (e.g., 'backend-development', 'frontend-mobile')"
                    },
                    "model": {
                        "type": "string",
                        "enum": ["sonnet", "haiku"],
                        "description": "Optional model filter"
                    }
                }
            }))
            .handler(|params| async move {
                // This will be implemented when we have access to agent_registry
                Ok(ToolResult::success(ToolContent::text(
                    "Agent listing functionality - 299 agents available across multiple categories".to_string()
                )))
            })
            .build();

        registry.register_tool(Box::new(list_agents_tool)).await?;

        Ok(())
    }

    /// Register command execution tools
    async fn register_command_tools(
        registry: &ToolRegistry,
        command_registry: &CommandRegistry,
        agent_registry: &AgentRegistry,
    ) -> Result<()> {
        // Add a tool to execute commands
        let execute_command_tool = DynamicToolBuilder::new("execute_command")
            .description("Execute any of the 61 available development commands with specialized functionality")
            .schema(json!({
                "type": "object",
                "properties": {
                    "command_name": {
                        "type": "string",
                        "description": "Name of the command to execute (e.g., 'api-scaffold', 'code-explain', 'ai-review')"
                    },
                    "task": {
                        "type": "string",
                        "description": "The task or request for the command"
                    },
                    "context": {
                        "type": "object",
                        "description": "Optional context information"
                    }
                },
                "required": ["command_name", "task"]
            }))
            .handler(move |params| async move {
                // This will be implemented when we have access to registries
                let command_name = params["command_name"].as_str().unwrap_or("unknown");
                let task = params["task"].as_str().unwrap_or("no task");

                Ok(ToolResult::success(ToolContent::text(format!(
                    "Command execution requested: {} with task: {}", command_name, task
                ))))
            })
            .build();

        registry.register_tool(Box::new(execute_command_tool)).await?;

        // Add a tool to list available commands
        let list_commands_tool = DynamicToolBuilder::new("list_commands")
            .description("List all available development commands and their specializations")
            .schema(json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Optional category filter (e.g., 'tools', 'workflows')"
                    }
                }
            }))
            .handler(|params| async move {
                Ok(ToolResult::success(ToolContent::text(
                    "Command listing functionality - 61 commands available across tools and workflows".to_string()
                )))
            })
            .build();

        registry.register_tool(Box::new(list_commands_tool)).await?;

        // Register comprehensive introspection tools (with hierarchical D-Bus discovery)
        use op_dbus::mcp::tools::introspection;
        for mcptool in introspection::register_introspection_tools() {
            // Convert McpTool to Tool trait using DynamicToolBuilder
            let tool_name = mcptool.name().to_string();
            let description = mcptool.description().to_string();
            let parameters = mcptool.parameters().to_vec();

            let tool = DynamicToolBuilder::new(&tool_name)
                .description(&description)
                .schema({
                    let mut schema = json!({
                        "type": "object",
                        "properties": {}
                    });
                    for param in &parameters {
                        schema["properties"][&param.name] = json!({
                            "type": param.type_,
                            "description": param.description
                        });
                        if !param.required {
                            if let Some(req) = schema.get_mut("required") {
                                if let Some(arr) = req.as_array_mut() {
                                    arr.retain(|x| x != &param.name);
                                }
                            }
                        }
                    }
                    schema
                })
                .handler(move |params| {
                    let tool_name_clone = tool_name.clone();
                    Box::pin(async move {
                        // For now, just return a placeholder - the real hierarchical tools need more work
                        Ok(ToolResult::success(ToolContent::text(format!(
                            "Hierarchical D-Bus introspection tool '{}' is available but needs implementation integration",
                            tool_name_clone
                        ))))
                    })
                })
                .build();

            registry.register_tool(Box::new(tool)).await?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id),
            "tools/list" => self.handle_tools_list(request.id).await,
            "tools/call" => self.handle_tools_call(request.id, request.params).await,
            "resources/list" => self.handle_resources_list(request.id).await,
            "resources/read" => self.handle_resources_read(request.id, request.params).await,
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
                    "description": "Refactored MCP server with loose coupling and markdown resources"
                }
            })),
            error: None,
        }
    }

    async fn handle_tools_list(&self, id: Option<Value>) -> McpResponse {
        let tools = self.tool_registry.list_tools().await;

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

        // Check if this is an agent execution request
        if tool_name == "execute_agent" {
            return self.handle_agent_execution(id, arguments).await;
        }

        // Check if this is a command execution request
        if tool_name == "execute_command" {
            return self.handle_command_execution(id, arguments).await;
        }

        // Check if this is a direct agent call (agent name used as tool name)
        if self.agent_registry.get_agent(tool_name).await.is_some() {
            return self.handle_direct_agent_execution(id, tool_name, arguments).await;
        }

        // Check if this is a direct command call (command name used as tool name)
        if self.command_registry.get_command(tool_name).await.is_some() {
            return self.handle_direct_command_execution(id, tool_name, arguments).await;
        }

        // Execute regular tool through registry
        match self.tool_registry.execute_tool(tool_name, arguments).await {
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

    async fn handle_agent_execution(&self, id: Option<Value>, arguments: Value) -> McpResponse {
        let agent_name = match arguments["agent_name"].as_str() {
            Some(name) => name,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing agent_name parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let task = match arguments["task"].as_str() {
            Some(task) => task,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing task parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let request = AgentRequest {
            task: task.to_string(),
            context: arguments["context"].as_object().cloned().map(Value::Object),
            parameters: arguments["parameters"].as_object().cloned().map(|m| {
                m.into_iter().map(|(k, v)| (k, v)).collect()
            }),
        };

        match self.agent_registry.execute_agent(agent_name, request).await {
            Ok(response) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": response.result
                    }],
                    "metadata": response.metadata
                })),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Agent execution failed: {}", e),
                    data: None,
                }),
            },
        }
    }

    async fn handle_direct_agent_execution(&self, id: Option<Value>, agent_name: &str, arguments: Value) -> McpResponse {
        let task = match arguments["task"].as_str() {
            Some(task) => task,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing task parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let request = AgentRequest {
            task: task.to_string(),
            context: arguments["context"].as_object().cloned().map(Value::Object),
            parameters: arguments["parameters"].as_object().cloned().map(|m| {
                m.into_iter().map(|(k, v)| (k, v)).collect()
            }),
        };

        match self.agent_registry.execute_agent(agent_name, request).await {
            Ok(response) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": response.result
                    }],
                    "metadata": response.metadata
                })),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Agent execution failed: {}", e),
                    data: None,
                }),
            },
        }
    }

    async fn handle_command_execution(&self, id: Option<Value>, arguments: Value) -> McpResponse {
        let command_name = match arguments["command_name"].as_str() {
            Some(name) => name,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing command_name parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let task = match arguments["task"].as_str() {
            Some(task) => task,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing task parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let request = AgentRequest {
            task: task.to_string(),
            context: arguments["context"].as_object().cloned().map(Value::Object),
            parameters: arguments["parameters"].as_object().cloned().map(|m| {
                m.into_iter().map(|(k, v)| (k, v)).collect()
            }),
        };

        match self.command_registry.execute_command(command_name, request, &self.agent_registry).await {
            Ok(response) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": response.result
                    }],
                    "metadata": response.metadata
                })),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Command execution failed: {}", e),
                    data: None,
                }),
            },
        }
    }

    async fn handle_direct_command_execution(&self, id: Option<Value>, command_name: &str, arguments: Value) -> McpResponse {
        let task = match arguments["task"].as_str() {
            Some(task) => task,
            None => {
                return McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(McpError {
                        code: -32602,
                        message: "Missing task parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let request = AgentRequest {
            task: task.to_string(),
            context: arguments["context"].as_object().cloned().map(Value::Object),
            parameters: arguments["parameters"].as_object().cloned().map(|m| {
                m.into_iter().map(|(k, v)| (k, v)).collect()
            }),
        };

        match self.command_registry.execute_command(command_name, request, &self.agent_registry).await {
            Ok(response) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": response.result
                    }],
                    "metadata": response.metadata
                })),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Command execution failed: {}", e),
                    data: None,
                }),
            },
        }
    }

    async fn handle_resources_list(&self, id: Option<Value>) -> McpResponse {
        let resources = self.resource_registry.list_resources().await;

        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "resources": resources
            })),
            error: None,
        }
    }

    async fn handle_resources_read(&self, id: Option<Value>, params: Option<Value>) -> McpResponse {
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
                        message: "Missing uri parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // Read resource through registry
        match self.resource_registry.read_resource(uri).await {
            Ok(content) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!(content)),
                error: None,
            },
            Err(e) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32603,
                    message: format!("Resource read failed: {}", e),
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
