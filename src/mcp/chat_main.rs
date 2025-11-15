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
mod chat_server;
mod tool_registry;

use agent_registry::AgentRegistry;
use chat_server::{create_chat_router, ChatServerState};
use tool_registry::{DynamicToolBuilder, Tool, ToolRegistry};
use op_dbus::blockchain::StreamingBlockchain;

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
    let chat_state = ChatServerState::new(tool_registry.clone(), agent_registry.clone());

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
        .route(
            "/",
            get(|| async { axum::response::Redirect::permanent("/chat.html") }),
        )
        .nest_service("/chat.html", ServeFile::new(web_dir.join("chat.html")))
        .nest_service("/chat.js", ServeFile::new(web_dir.join("chat.js")))
        .nest_service(
            "/chat-styles.css",
            ServeFile::new(web_dir.join("chat-styles.css")),
        )
        // Fallback to index for other static assets
        .fallback_service(ServeDir::new(web_dir))
        // Add tracing
        .layer(TraceLayer::new_for_http());

    // Start the server
    // Bind to 0.0.0.0 to accept connections from any network interface
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("MCP Chat Server listening on http://{}", addr);
    info!("Server accessible at http://<your-ip>:8080/chat.html");

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

    // Blockchain snapshot management
    registry.register_tool(Box::new(BlockchainSnapshotTool)).await?;

    // Blockchain retention policy
    registry.register_tool(Box::new(BlockchainRetentionTool)).await?;

    let tools = registry.list_tools().await;
    info!("Registered {} default tools", tools.len());
    Ok(())
}

// Example tool implementations
struct SystemdTool;

#[async_trait::async_trait]
impl Tool for SystemdTool {
    fn name(&self) -> &str {
        "systemd"
    }

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
            _ => format!("Action {} on {}", action, service),
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
    fn name(&self) -> &str {
        "file"
    }

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
            _ => format!("Unknown action: {}", action),
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
    fn name(&self) -> &str {
        "network"
    }

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
            _ => format!("Performed {} on network", action),
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
    fn name(&self) -> &str {
        "process"
    }

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
            _ => format!("Performed {} on process", action),
        };

        Ok(tool_registry::ToolResult {
            content: vec![tool_registry::ToolContent::text(output)],
            metadata: None,
        })
    }
}

struct BlockchainSnapshotTool;

#[async_trait::async_trait]
impl Tool for BlockchainSnapshotTool {
    fn name(&self) -> &str {
        "blockchain_snapshot"
    }

    fn description(&self) -> &str {
        "Manage blockchain state snapshots for disaster recovery (list, show, rollback)"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "show", "rollback"],
                    "description": "Operation to perform"
                },
                "snapshot_name": {
                    "type": "string",
                    "description": "Snapshot name (required for show/rollback)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<tool_registry::ToolResult> {
        let action = params["action"].as_str().unwrap_or("list");
        let blockchain_path = std::env::var("OPDBUS_BLOCKCHAIN_PATH")
            .unwrap_or_else(|_| "/var/lib/op-dbus/blockchain".to_string());

        match action {
            "list" => {
                // List all snapshots
                let blockchain = StreamingBlockchain::new(&blockchain_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to initialize blockchain: {}", e))?;

                let snapshots = blockchain.list_state_snapshots().await?;

                if snapshots.is_empty() {
                    return Ok(tool_registry::ToolResult {
                        content: vec![tool_registry::ToolContent::text(
                            "No snapshots found. Snapshots are created automatically when you apply state changes."
                        )],
                        metadata: None,
                    });
                }

                let mut output = format!("Found {} state snapshots:\n\n", snapshots.len());
                for (name, timestamp) in &snapshots {
                    output.push_str(&format!("• {} ({})\n", name, timestamp));
                }
                output.push_str("\nUse action='show' with snapshot_name to view details.");

                Ok(tool_registry::ToolResult {
                    content: vec![tool_registry::ToolContent::text(output)],
                    metadata: Some(serde_json::json!({
                        "snapshot_count": snapshots.len()
                    })),
                })
            }
            "show" => {
                let snapshot_name = params["snapshot_name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("snapshot_name required for show action"))?;

                let blockchain = StreamingBlockchain::new(&blockchain_path).await?;
                let state_file = blockchain.rollback_to_snapshot(snapshot_name).await?;

                let state_content = tokio::fs::read_to_string(&state_file).await?;
                let state: serde_json::Value = serde_json::from_str(&state_content)?;

                Ok(tool_registry::ToolResult {
                    content: vec![
                        tool_registry::ToolContent::text(format!("Snapshot: {}", snapshot_name)),
                        tool_registry::ToolContent::json(state),
                    ],
                    metadata: Some(serde_json::json!({
                        "snapshot_name": snapshot_name,
                        "state_file": state_file.display().to_string()
                    })),
                })
            }
            "rollback" => {
                Ok(tool_registry::ToolResult {
                    content: vec![tool_registry::ToolContent::text(
                        "Rollback functionality coming soon. For now, use 'show' to view snapshot contents."
                    )],
                    metadata: None,
                })
            }
            _ => Ok(tool_registry::ToolResult {
                content: vec![tool_registry::ToolContent::error(format!("Unknown action: {}", action))],
                metadata: None,
            }),
        }
    }
}

struct BlockchainRetentionTool;

#[async_trait::async_trait]
impl Tool for BlockchainRetentionTool {
    fn name(&self) -> &str {
        "blockchain_retention"
    }

    fn description(&self) -> &str {
        "View and update blockchain snapshot retention policy (hourly, daily, weekly, quarterly)"
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["show", "update"],
                    "description": "Show current policy or update it"
                },
                "hourly": {
                    "type": "number",
                    "description": "Number of hourly snapshots to keep"
                },
                "daily": {
                    "type": "number",
                    "description": "Number of daily snapshots to keep"
                },
                "weekly": {
                    "type": "number",
                    "description": "Number of weekly snapshots to keep"
                },
                "quarterly": {
                    "type": "number",
                    "description": "Number of quarterly snapshots to keep"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<tool_registry::ToolResult> {
        let action = params["action"].as_str().unwrap_or("show");
        let blockchain_path = std::env::var("OPDBUS_BLOCKCHAIN_PATH")
            .unwrap_or_else(|_| "/var/lib/op-dbus/blockchain".to_string());

        let mut blockchain = StreamingBlockchain::new(&blockchain_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize blockchain: {}", e))?;

        match action {
            "show" => {
                let policy = blockchain.retention_policy();

                let output = format!(
                    "Snapshot Retention Policy:\n\n\
                     • Hourly:    {} snapshots (last 24 hours)\n\
                     • Daily:     {} snapshots (last 30 days)\n\
                     • Weekly:    {} snapshots (last 12 weeks)\n\
                     • Quarterly: {} snapshots (long-term)\n\n\
                     Old snapshots are automatically pruned based on this policy.",
                    policy.hourly, policy.daily, policy.weekly, policy.quarterly
                );

                Ok(tool_registry::ToolResult {
                    content: vec![tool_registry::ToolContent::text(output)],
                    metadata: Some(serde_json::json!({
                        "hourly": policy.hourly,
                        "daily": policy.daily,
                        "weekly": policy.weekly,
                        "quarterly": policy.quarterly
                    })),
                })
            }
            "update" => {
                let mut policy = blockchain.retention_policy();
                let mut changed = false;

                if let Some(hourly) = params["hourly"].as_u64() {
                    policy.set_hourly(hourly as usize);
                    changed = true;
                }
                if let Some(daily) = params["daily"].as_u64() {
                    policy.set_daily(daily as usize);
                    changed = true;
                }
                if let Some(weekly) = params["weekly"].as_u64() {
                    policy.set_weekly(weekly as usize);
                    changed = true;
                }
                if let Some(quarterly) = params["quarterly"].as_u64() {
                    policy.set_quarterly(quarterly as usize);
                    changed = true;
                }

                if !changed {
                    return Ok(tool_registry::ToolResult {
                        content: vec![tool_registry::ToolContent::error(
                            "No changes specified. Provide hourly, daily, weekly, or quarterly parameters."
                        )],
                        metadata: None,
                    });
                }

                blockchain.set_retention_policy(policy);

                let output = format!(
                    "Updated Retention Policy:\n\n\
                     • Hourly:    {} snapshots\n\
                     • Daily:     {} snapshots\n\
                     • Weekly:    {} snapshots\n\
                     • Quarterly: {} snapshots\n\n\
                     Note: This is a runtime change. Set environment variables for persistence.",
                    policy.hourly, policy.daily, policy.weekly, policy.quarterly
                );

                Ok(tool_registry::ToolResult {
                    content: vec![tool_registry::ToolContent::text(output)],
                    metadata: Some(serde_json::json!({
                        "hourly": policy.hourly,
                        "daily": policy.daily,
                        "weekly": policy.weekly,
                        "quarterly": policy.quarterly,
                        "updated": true
                    })),
                })
            }
            _ => Ok(tool_registry::ToolResult {
                content: vec![tool_registry::ToolContent::error(format!("Unknown action: {}", action))],
                metadata: None,
            }),
        }
    }
}
