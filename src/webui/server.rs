//! Web server for op-dbus UI

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::state::StateManager;

#[derive(Clone)]
pub struct AppState {
    state_manager: Arc<StateManager>,
}

#[derive(Clone, Debug)]
pub struct WebConfig {
    pub bind_addr: String,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0".to_string(),
            port: 9573, // OPDBUS on phone keypad: 6-7-3-2-8-7 (compressed)
        }
    }
}

/// Start web server
pub async fn start_web_server(
    state_manager: Arc<StateManager>,
    config: WebConfig,
) -> Result<()> {
    let app_state = AppState { state_manager };

    let app = Router::new()
        // API routes
        .route("/api/plugins", get(list_plugins))
        .route("/api/plugins/:plugin", get(query_plugin))
        .route("/api/plugins/:plugin/state", get(query_plugin_state))
        .route("/api/plugins/:plugin/apply", post(apply_plugin_state))
        
        // PlugTree routes (per-resource)
        .route("/api/containers", get(list_containers))
        .route("/api/containers/:id", get(get_container))
        .route("/api/containers/:id", post(apply_container))
        .route("/api/containers/:id", delete(delete_container))
        
        .route("/api/units", get(list_units))
        .route("/api/units/:name", get(get_unit))
        .route("/api/units/:name", post(apply_unit))
        
        // System-wide
        .route("/api/query", get(query_all))
        .route("/api/introspect", get(introspect_databases))
        
        // UI
        .route("/", get(index_handler))
        .route("/containers", get(containers_page))
        .route("/network", get(network_page))
        .route("/systemd", get(systemd_page))
        
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = format!("{}:{}", config.bind_addr, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Web UI available at http://{}", addr);
    tracing::info!("  Dashboard: http://{}/", addr);
    tracing::info!("  API docs: http://{}/api", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}

// API Handlers

async fn list_plugins(State(_state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "plugins": ["net", "systemd", "lxc", "login1"]
    }))
}

async fn query_plugin(
    State(state): State<AppState>,
    Path(plugin): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.state_manager.query_plugin_state(&plugin).await {
        Ok(plugin_state) => Ok(Json(plugin_state)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn query_plugin_state(
    State(state): State<AppState>,
    Path(plugin): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    match state.state_manager.query_plugin_state(&plugin).await {
        Ok(plugin_state) => Ok(Json(plugin_state)),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Deserialize)]
struct ApplyRequest {
    state: Value,
}

async fn apply_plugin_state(
    State(_state): State<AppState>,
    Path(_plugin): Path<String>,
    Json(_req): Json<ApplyRequest>,
) -> impl IntoResponse {
    // TODO: Implement apply
    StatusCode::NOT_IMPLEMENTED
}

async fn query_all(State(state): State<AppState>) -> Result<Json<Value>, StatusCode> {
    match state.state_manager.query_current_state().await {
        Ok(current) => Ok(Json(serde_json::to_value(current).unwrap())),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Container (PlugTree) handlers

async fn list_containers(State(_state): State<AppState>) -> impl IntoResponse {
    // TODO: Query LXC plugin for all containers
    Json(serde_json::json!({"containers": []}))
}

async fn get_container(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Query specific container
    StatusCode::NOT_IMPLEMENTED
}

async fn apply_container(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    Json(_req): Json<Value>,
) -> impl IntoResponse {
    // TODO: Apply container state
    StatusCode::NOT_IMPLEMENTED
}

async fn delete_container(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> impl IntoResponse {
    // TODO: Delete container
    StatusCode::NOT_IMPLEMENTED
}

// Unit (PlugTree) handlers

async fn list_units(State(_state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({"units": []}))
}

async fn get_unit(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn apply_unit(
    State(_state): State<AppState>,
    Path(_name): Path<String>,
    Json(_req): Json<Value>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn introspect_databases(State(_state): State<AppState>) -> impl IntoResponse {
    // TODO: Run introspection on both databases
    Json(serde_json::json!({
        "ovsdb": {},
        "nonnet": {}
    }))
}

// HTML Pages

async fn index_handler() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>op-dbus Control Panel</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #1a1a1a;
            color: #e0e0e0;
        }
        h1 { color: #4fc3f7; }
        .card {
            background: #2a2a2a;
            border-radius: 8px;
            padding: 20px;
            margin: 10px 0;
            border-left: 4px solid #4fc3f7;
        }
        .card h2 { margin-top: 0; color: #81c784; }
        a {
            color: #4fc3f7;
            text-decoration: none;
            display: inline-block;
            margin: 5px 10px 5px 0;
        }
        a:hover { text-decoration: underline; }
        code {
            background: #1a1a1a;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
        }
    </style>
</head>
<body>
    <h1>🔧 op-dbus Control Panel</h1>
    <p>Declarative system state management via native protocols</p>
    <p style="color: #888; font-size: 0.9em;">Port 9573 = OPDBUS on phone keypad</p>

    <div class="card">
        <h2>Plugins</h2>
        <a href="/containers">📦 Containers (LXC)</a>
        <a href="/network">🌐 Network (OVS)</a>
        <a href="/systemd">⚙️ Systemd Units</a>
        <a href="/api/query">🔍 Query All State</a>
    </div>

    <div class="card">
        <h2>API Endpoints</h2>
        <p><code>GET /api/plugins</code> - List all plugins</p>
        <p><code>GET /api/plugins/:plugin</code> - Query plugin state</p>
        <p><code>GET /api/containers</code> - List containers</p>
        <p><code>GET /api/containers/:id</code> - Get container details</p>
        <p><code>POST /api/containers/:id</code> - Apply container state</p>
        <p><code>GET /api/units</code> - List systemd units</p>
        <p><code>POST /api/units/:name</code> - Control unit (start/stop/enable/disable/mask)</p>
        <p><code>GET /api/introspect</code> - Introspect OVSDB + NonNet databases</p>
    </div>

    <div class="card">
        <h2>Documentation</h2>
        <p>Repository: <a href="https://github.com/repr0bated/operation-dbus" target="_blank">github.com/repr0bated/operation-dbus</a></p>
    </div>
</body>
</html>
    "#)
}

async fn containers_page() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Containers - op-dbus</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
            background: #1a1a1a;
            color: #e0e0e0;
        }
        h1 { color: #4fc3f7; }
        .container-list {
            display: grid;
            gap: 10px;
        }
        .container-card {
            background: #2a2a2a;
            padding: 15px;
            border-radius: 8px;
            border-left: 4px solid #81c784;
        }
        .status-running { color: #81c784; }
        .status-stopped { color: #e57373; }
        button {
            background: #4fc3f7;
            color: #1a1a1a;
            border: none;
            padding: 8px 16px;
            border-radius: 4px;
            cursor: pointer;
            margin: 5px;
        }
        button:hover { background: #81c784; }
    </style>
</head>
<body>
    <h1>📦 LXC Containers</h1>
    <p><a href="/">← Back to Dashboard</a></p>
    
    <div id="containers" class="container-list">
        <p>Loading containers...</p>
    </div>

    <script>
        async function loadContainers() {
            const res = await fetch('/api/containers');
            const data = await res.json();
            const container_html = data.containers.map(c => `
                <div class="container-card">
                    <h3>Container ${c.id}</h3>
                    <p>Bridge: ${c.bridge}</p>
                    <p>Network: ${c.properties?.network_type || 'bridge'}</p>
                    <p>Status: <span class="status-${c.running ? 'running' : 'stopped'}">${c.running ? 'Running' : 'Stopped'}</span></p>
                    <button onclick="startContainer('${c.id}')">Start</button>
                    <button onclick="stopContainer('${c.id}')">Stop</button>
                </div>
            `).join('');
            document.getElementById('containers').innerHTML = container_html || '<p>No containers found</p>';
        }

        async function startContainer(id) {
            console.log('Starting container:', id);
            // TODO: Implement
        }

        async function stopContainer(id) {
            console.log('Stopping container:', id);
            // TODO: Implement
        }

        loadContainers();
    </script>
</body>
</html>
    "#)
}

async fn network_page() -> Html<&'static str> {
    Html("<h1>Network Configuration</h1><p>Coming soon</p>")
}

async fn systemd_page() -> Html<&'static str> {
    Html("<h1>Systemd Units</h1><p>Coming soon</p>")
}

