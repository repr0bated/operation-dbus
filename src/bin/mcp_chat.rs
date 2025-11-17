//! MCP Chat Server - DeepSeek Integration
//! Uses the existing chat_server.rs infrastructure
//! 
//! Run with: OLLAMA_API_KEY=your-key cargo run --bin mcp_chat

use axum::{response::Redirect, routing::get, Router};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;

// Import from the existing MCP modules
use op_dbus::mcp::agent_registry::AgentRegistry;
use op_dbus::mcp::chat_server::{create_chat_router, ChatServerState};
use op_dbus::mcp::ollama::OllamaClient;
use op_dbus::mcp::tool_registry::ToolRegistry;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🚀 Starting MCP Chat Server with DeepSeek Integration...\n");

    // Get API key from environment
    let api_key = std::env::var("OLLAMA_API_KEY")
        .expect("OLLAMA_API_KEY must be set. Run: export OLLAMA_API_KEY=your-key");

    println!("✅ OLLAMA_API_KEY loaded");

    // Initialize Ollama client for DeepSeek Cloud
    println!("🔌 Connecting to Ollama/DeepSeek...");
    let ollama_client = OllamaClient::cloud(api_key);

    // Test connection
    match ollama_client.health_check().await {
        Ok(true) => println!("✅ Connected to Ollama"),
        Ok(false) => println!("⚠️  Using cloud API (no local Ollama)"),
        Err(e) => println!("⚠️  Health check failed: {} (will use cloud API)", e),
    }

    // Initialize tool and agent registries
    println!("📦 Initializing MCP components...");
    let tool_registry = Arc::new(ToolRegistry::new());
    let agent_registry = Arc::new(AgentRegistry::new());

    // Register introspection tools
    op_dbus::mcp::introspection_tools::register_introspection_tools(&tool_registry)
        .await
        .expect("Failed to register introspection tools");

    println!("✅ {} tools registered", tool_registry.list_tools().await.len());
    println!("✅ {} agent types available", agent_registry.list_agent_types().await.len());

    // Create chat server state with Ollama client
    let chat_state = ChatServerState::new(tool_registry, agent_registry)
        .with_ollama_client(ollama_client);

    println!("✅ DeepSeek AI integrated");

    // Web directory for UI
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("mcp")
        .join("web");

    println!("📁 Web directory: {}", web_dir.display());

    // Build router combining chat API + static files
    let chat_router = create_chat_router(chat_state);

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/index.html") }))
        .merge(chat_router)
        .nest_service("/", ServeDir::new(&web_dir));

    // Bind to all interfaces (accessible remotely)
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;

    println!("\n✅ MCP Chat Server ready!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🌐 Web UI:");
    println!("   http://localhost:8080");
    println!("   http://100.104.70.1:8080 (Netmaker)");
    println!("   http://80.209.240.244:8080 (Public)");
    println!("\n📡 API Endpoints:");
    println!("   WebSocket: ws://server:8080/ws");
    println!("   POST /api/suggestions");
    println!("   POST /api/history");
    println!("\n🤖 DeepSeek Model: deepseek-v3.1:671b-cloud");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\nFeatures:");
    println!("  ✓ Natural language command processing");
    println!("  ✓ DeepSeek AI with full system context");
    println!("  ✓ Hardware introspection (CPU, BIOS, IOMMU)");
    println!("  ✓ ISP/Provider analysis");
    println!("  ✓ MCP tool execution");
    println!("  ✓ Agent orchestration");
    println!("\nPress Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
