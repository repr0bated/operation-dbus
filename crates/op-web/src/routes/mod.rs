//! API routes and route handlers

use axum::{
    extract::Extension,
    response::Html,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::mcp;
use crate::mcp_agents;
use crate::mcp_discovery;
use crate::groups_admin;
use crate::middleware::security;
use crate::sse;
use crate::state::AppState;
use crate::websocket;

#[allow(dead_code)]
pub mod chat;
#[allow(dead_code)]
pub mod llm;
pub mod admin;

// UI is embedded via rust-embed in embedded_ui.rs from ui/dist/
use crate::embedded_ui::UiAssets;

async fn index_handler() -> Html<String> {
    // Serve embedded UI from ui/dist/index.html
    if let Some(content) = UiAssets::get("index.html") {
        return Html(String::from_utf8_lossy(&content.data).to_string());
    }
    Html("<h1>UI not found</h1>".to_string())
}

/// Create the complete router with all routes
pub fn create_router(state: Arc<AppState>) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes
    let api_routes = Router::new()
        // Health & Status
        .route("/health", get(handlers::health::health_handler))
        .route("/status", get(handlers::status::status_handler))
        // Chat endpoints
        .route("/chat", post(handlers::chat::chat_handler))
        .route("/chat/stream", post(handlers::chat::chat_stream_handler))
        .route("/chat/history/:session_id", get(handlers::chat::get_history_handler))
        .route("/chat/transcript", post(handlers::chat::save_transcript_handler))
        // Tool endpoints
        .route("/tools", get(handlers::tools::list_tools_handler))
        .route("/tools/:name", get(handlers::tools::get_tool_handler))
        .route("/tool", post(handlers::tools::execute_tool_handler))
        .route("/tools/:name/execute", post(handlers::tools::execute_named_tool_handler))
        // Agent endpoints
        .route("/agents", get(handlers::agents::list_agents_handler))
        .route("/agents", post(handlers::agents::spawn_agent_handler))
        .route("/agents/types", get(handlers::agents::list_agent_types_handler))
        .route("/agents/:id", get(handlers::agents::get_agent_handler))
        .route(
            "/agents/:id",
            axum::routing::delete(handlers::agents::kill_agent_handler),
        )
        // LLM endpoints
        .route("/llm/status", get(handlers::llm::llm_status_handler))
        .route("/llm/providers", get(handlers::llm::list_providers_handler))
        .route("/llm/models", get(handlers::llm::list_models_handler))
        .route("/llm/models/:provider", get(handlers::llm::list_models_for_provider_handler))
        .route("/llm/provider", post(handlers::llm::switch_provider_handler))
        .route("/llm/model", post(handlers::llm::switch_model_handler))
        // MCP discovery endpoints
        .route("/mcp/_config", get(mcp::config_handler))
        // SSE events
        .route("/events", get(sse::sse_handler))
        // Privacy router endpoints
        .route("/privacy/signup", post(handlers::privacy::signup))
        .route("/privacy/verify", get(handlers::privacy::verify))
        .route("/privacy/config/:user_id", get(handlers::privacy::get_config))
        .route("/privacy/status", get(handlers::privacy::status))
        .route("/privacy/credentials", post(handlers::privacy::set_credentials))
        // Google OAuth endpoints
        .route("/privacy/google/auth", get(handlers::privacy::google_auth))
        .route("/privacy/google/callback", get(handlers::privacy::google_callback));

    // MCP JSON-RPC endpoints (profile-based and legacy)
    let mcp_route = mcp::create_mcp_router();

    // Critical Agents MCP endpoint (SSE-based, direct tool access)
    // These are added separately to avoid state conflicts
    let agents_mcp_route = Router::new()
        .route("/mcp/agents", get(mcp_agents::mcp_agents_sse_handler_stateless))
        .route("/mcp/agents/message", post(mcp_agents::mcp_agents_message_handler_stateless));

    // WebSocket route
    let ws_route = Router::new()
        .route("/ws", get(websocket::websocket_handler));

    // Main router - agents_mcp_route FIRST so it takes precedence
    let mut router = Router::new()
        .nest("/api", api_routes)
        // JSON-RPC compatibility aliases (mirror /mcp)
        .route("/jsonrpc", post(mcp::jsonrpc_handler))
        .route("/rpc", post(mcp::jsonrpc_handler))
        .merge(agents_mcp_route)  // Agents first (more specific)
        .nest("/mcp", mcp_route)  // Nest MCP routes under /mcp (not root)
        .merge(ws_route)
        // Well-known discovery endpoint for auto-configuration
        .route("/.well-known/mcp.json", get(mcp_discovery::mcp_discovery_handler))
        .nest("/groups-admin", groups_admin::create_groups_admin_router())
        .nest("/admin", admin::admin_routes());

    // Use embedded UI for all non-API routes
    router = router.fallback(crate::embedded_ui::serve_embedded_ui);

    router
        .layer(Extension(state))
        .layer(axum::middleware::from_fn(security::ip_security_middleware))
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}
