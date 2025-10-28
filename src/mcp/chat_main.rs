use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{info, warn};

mod agent_registry;
mod tool_registry;
mod chat_server;

use agent_registry::AgentRegistry;
use tool_registry::{ToolRegistry, DynamicToolBuilder, Tool};
use chat_server::{ChatServerState, create_chat_router};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting MCP Chat Server...");
    
    // Initialize registries
    let tool_registry = Arc::new(ToolRegistry::new());
    let agent_registry = Arc::new(AgentRegistry::new());
    
    // Register default tools
    register_default_tools(&tool_registry).await?;
    
    // Load default agent specs
    agent_registry::load_default_specs(&*agent_registry).await?;
    
    // Create chat server state
    let chat_state = ChatServerState::new(
        tool_registry.clone(),
        agent_registry.clone()
    );
    
    // Setup static file serving for the web UI
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("mcp")
        .join("web");
    
    // Create the main router
    let app = Router::new()
        // Chat routes
        .nest("/chat", create_chat_router(chat_state))
        
        // Serve static files
        .route("/", get(|| async { 
            axum::response::Redirect::permanent("/chat.html")
        }))
        .nest_service("/chat.html", ServeFile::new(web_dir.join("chat.html")))
        .nest_service("/chat.js", ServeFile::new(web_dir.join("chat.js")))
        .nest_service("/chat-styles.css", ServeFile::new(web_dir.join("chat-styles.css")))
        
        // Fallback to index for other static assets
        .fallback_service(ServeDir::new(web_dir))
        
        // Add tracing
        .layer(TraceLayer::new_for_http());
    
    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("MCP Chat Server listening on http://{}", addr);
    info!("Open http://{}/chat.html in your browser", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn register_default_tools(registry: &ToolRegistry) -> Result<()> {
    // Systemd tool
    registry.register_tool(Box::new(SystemdTool)).await?;
    
    // File tool
    registry.register_tool(Box::new(FileTool)).await?;
    
    // Network tool
    registry.register_tool(Box::new(NetworkTool)).await?;
    
    // Process tool
    registry.register_tool(Box::new(ProcessTool)).await?;
    
    let tools = registry.list_tools().await;
    info!("Registered {} default tools", tools.len());
    Ok(())
}

// Example tool implementations
struct SystemdTool;

#[async_trait::async_trait]
impl Tool for SystemdTool {
    fn name(&self) -> &str { "systemd" }
    
    fn description(&self) -> &str {
        "Manage systemd services"
    }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "start", "stop", "restart", "enable", "disable"]
                },
                "service": {
                    "type": "string",
                    "description": "Service name (e.g., nginx.service)"
                }
            },
            "required": ["action", "service"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<tool_registry::ToolResult> {
        let action = params["action"].as_str().unwrap_or("status");
        let service = params["service"].as_str().unwrap_or("");
        
        // Simulate systemctl command
        let output = match action {
            "status" => format!("● {} - Active: running", service),
            "start" => format!("Started {}", service),
            "stop" => format!("Stopped {}", service),
            "restart" => format!("Restarted {}", service),
            _ => format!("Action {} on {}", action, service)
        };
        
        Ok(tool_registry::ToolResult {
            content: vec![tool_registry::ToolContent::text(output)],
            metadata: None,
        })
    }
}

struct FileTool;

#[async_trait::async_trait]
impl Tool for FileTool {
    fn name(&self) -> &str { "file" }
    
    fn description(&self) -> &str {
        "File operations (read, write, delete)"
    }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read", "write", "delete"]
                },
                "path": {
                    "type": "string",
                    "description": "File path"
                },
                "content": {
                    "type": "string",
                    "description": "Content for write operations"
                }
            },
            "required": ["action", "path"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<tool_registry::ToolResult> {
        let action = params["action"].as_str().unwrap_or("read");
        let path = params["path"].as_str().unwrap_or("");
        
        let output = match action {
            "read" => format!("Contents of {}: [file content]", path),
            "write" => format!("Wrote to {}", path),
            "delete" => format!("Deleted {}", path),
            _ => format!("Unknown action: {}", action)
        };
        
        Ok(tool_registry::ToolResult {
            content: vec![tool_registry::ToolContent::text(output)],
            metadata: None,
        })
    }
}

struct NetworkTool;

#[async_trait::async_trait]
impl Tool for NetworkTool {
    fn name(&self) -> &str { "network" }
    
    fn description(&self) -> &str {
        "Network interface management"
    }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "up", "down", "configure"]
                },
                "interface": {
                    "type": "string",
                    "description": "Network interface name"
                }
            },
            "required": ["action"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<tool_registry::ToolResult> {
        let action = params["action"].as_str().unwrap_or("list");
        
        let output = match action {
            "list" => "eth0: UP, 192.168.1.100/24\nlo: UP, 127.0.0.1/8".to_string(),
            _ => format!("Performed {} on network", action)
        };
        
        Ok(tool_registry::ToolResult {
            content: vec![tool_registry::ToolContent::text(output)],
            metadata: None,
        })
    }
}

struct ProcessTool;

#[async_trait::async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str { "process" }
    
    fn description(&self) -> &str {
        "Process management and monitoring"
    }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "kill", "info"]
                },
                "pid": {
                    "type": "number",
                    "description": "Process ID"
                }
            },
            "required": ["action"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<tool_registry::ToolResult> {
        let action = params["action"].as_str().unwrap_or("list");
        
        let output = match action {
            "list" => "PID   CMD\n1234  systemd\n5678  nginx".to_string(),
            _ => format!("Performed {} on process", action)
        };
        
        Ok(tool_registry::ToolResult {
            content: vec![tool_registry::ToolContent::text(output)],
            metadata: None,
        })
    }
}