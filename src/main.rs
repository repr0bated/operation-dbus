//! OP-DBUS: Native, Deterministic Control Plane for Linux Systems
//! 
//! Production entry point with all components wired together.

use std::sync::Arc;
use std::path::PathBuf;
use std::net::SocketAddr;
use parking_lot::RwLock;
use simd_json::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Crate imports (Authoritative logic)
use op_core::types::BusType;
use op_state_store::{StateStore, SqliteStore};
use op_tools::{ToolRegistry, register_builtin_tools};
use op_plugins::registry::PluginRegistry;
use op_plugins::plugin::{PluginMetadata as PluginCore, PluginTunables};
use op_workflows::orchestrator::{Orchestrator, OrchestratorConfig};
use op_introspection::projection::DbusProjection;
use op_blockchain::StreamingBlockchain;

// Internal modules (Glue logic)
use op_dbus::{
    cache::BtrfsCache,
    chatbot::{Chatbot, ChatbotConfig},
    constants,
    dependency::DependencyManager,
    disaster_recovery::DisasterRecovery,
    error::Result,
    inspector_gadget::{InspectorGadget, InspectorConfig},
    json_rpc::{JsonRpcRequest, JsonRpcResponse, JsonRpcError},
    mcp::{McpCompactDispatcher, McpRequest, McpResponse, McpError},
    mcp_live::McpLiveDispatcher,
    numa_cache::NumaOptimizer,
    policy::PolicyEngine,
    vectorization::FootprintGenerator,
};
use op_dbus_model;

use op_web::{routes, AppState};
#[cfg(feature = "grpc")]
use op_state_store::ChainConfig;
#[cfg(feature = "grpc")]
use op_grpc_bridge::{SyncEngine, DbusWatcher, WatchConfig, run_grpc_server, PluginSchemaProvider};
#[cfg(feature = "grpc")]
use op_grpc_bridge::proto::PluginInfo;
#[cfg(feature = "grpc")]
use serde_json::Value as JsonValue;

#[cfg(feature = "dev-antigravity")]
use op_dbus::antigravity::{
    AntigravityTunnel, AntigravityConfig,
    transport::{TunnelTransport, TransportConfig, TransportType},
};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone)]
struct Config {
    database_url: String,
    cache_dir: String,
    enable_dbus: bool,
    dbus_connection: BusType,
    enable_web: bool,
    web_host: String,
    web_port: u16,
    listen: String,
    #[cfg(feature = "dev-antigravity")]
    enable_antigravity: bool,
    #[cfg(feature = "dev-antigravity")]
    antigravity_listen: String,
    #[cfg(feature = "dev-antigravity")]
    antigravity_transport: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: std::env::var("OP_DBUS_DATABASE_URL")
                .unwrap_or_else(|_| format!("sqlite://{}", constants::STATE_DB_PATH)),
            cache_dir: std::env::var("OP_DBUS_CACHE_DIR")
                .unwrap_or_else(|_| constants::BTRFS_CACHE_SUBVOL_PREFIX.to_string()),
            enable_dbus: std::env::var("OP_DBUS_ENABLE_DBUS")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(true),
            dbus_connection: if std::env::var("OP_DBUS_SESSION_BUS").is_ok() {
                BusType::Session
            } else {
                BusType::System
            },
            enable_web: std::env::var("OP_DBUS_ENABLE_WEB")
                .map(|v| v != "0" && v.to_lowercase() != "false")
                .unwrap_or(true),
            web_host: std::env::var("OP_DBUS_WEB_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            web_port: std::env::var("OP_DBUS_WEB_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(constants::WEB_DEFAULT_PORT),
            listen: std::env::var("OP_DBUS_LISTEN")
                .unwrap_or_else(|_| "none".to_string()),
            #[cfg(feature = "dev-antigravity")]
            enable_antigravity: std::env::var("OP_DBUS_ENABLE_ANTIGRAVITY")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            #[cfg(feature = "dev-antigravity")]
            antigravity_listen: std::env::var("OP_DBUS_ANTIGRAVITY_LISTEN")
                .unwrap_or_else(|_| format!("127.0.0.1:{}", constants::ANTIGRAVITY_DEFAULT_PORT)),
            #[cfg(feature = "dev-antigravity")]
            antigravity_transport: std::env::var("OP_DBUS_ANTIGRAVITY_TRANSPORT")
                .unwrap_or_else(|_| "tcp".to_string()),
        }
    }
}

#[cfg(feature = "grpc")]
struct OpdbusPluginProvider;

#[cfg(feature = "grpc")]
impl PluginSchemaProvider for OpdbusPluginProvider {
    fn list_plugins(&self) -> Vec<PluginInfo> {
        let mut plugins = Vec::new();
        for plugin in op_dbus::plugins::plugin_definitions() {
            let mut description = String::new();
            let mut dbus_path = String::new();
            let mut interfaces = Vec::new();

            if let Ok(value) = serde_json::from_str::<JsonValue>(plugin.schema_json) {
                if let Some(desc) = value.get("description").and_then(|v| v.as_str()) {
                    description = desc.to_string();
                }
                if let Some(object_types) = value.get("object_types").and_then(|v| v.as_object()) {
                    for (_name, entry) in object_types {
                        if let Some(path) = entry.get("base_path").and_then(|v| v.as_str()) {
                            if dbus_path.is_empty() {
                                dbus_path = path.to_string();
                            }
                        }
                        if let Some(interface) = entry.get("interface").and_then(|v| v.as_str()) {
                            interfaces.push(interface.to_string());
                        }
                    }
                }
            }

            plugins.push(PluginInfo {
                id: plugin.name.to_string(),
                name: plugin.name.to_string(),
                version: "v1".to_string(),
                description,
                dbus_path,
                interfaces,
                tags: Vec::new(),
            });
        }
        plugins
    }

    fn get_schema(&self, plugin_id: &str) -> Option<(String, String, String)> {
        let schema = op_dbus::plugins::get_plugin_schema_json(plugin_id)?;
        Some((schema.to_string(), "json-schema-2020-12".to_string(), "v1".to_string()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "op_dbus=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::default();

    tracing::info!("======================================");
    tracing::info!("OP-DBUS: Native Deterministic Control Plane");
    tracing::info!("======================================");
    tracing::info!("Database: {}", config.database_url);
    tracing::info!("Cache: {}", config.cache_dir);
    tracing::info!("Web: {}:{}", config.web_host, config.web_port);

    #[cfg(feature = "dev-antigravity")]
    if config.enable_antigravity {
        tracing::warn!("============================================");
        tracing::warn!("DEVELOPMENT BUILD: Antigravity tunnel enabled");
        tracing::warn!("This feature is REMOVED in production builds");
        tracing::warn!("============================================");
    }

    // Initialize state store (authoritative database)
    let sqlite_store = SqliteStore::new(&config.database_url).await?;
    let pool = sqlite_store.pool().clone();
    let state_store: Arc<dyn StateStore> = Arc::new(sqlite_store);

    op_dbus_model::create_schema(&pool).await?;
    op_dbus::plugins::insert_plugins(&pool).await?;
    op_dbus::pre_canned::create_pre_canned_schemas(&pool).await?;
    op_dbus::plugins::validate_plugin_schemas_from_repo()?;

    // Initialize NUMA optimizer
    let numa_optimizer = NumaOptimizer::from_env();
    if numa_optimizer.is_available() {
        tracing::info!("NUMA optimization enabled");
    }

    // Initialize vectorization
    let footprint_generator = FootprintGenerator::from_env();
    tracing::info!("Vectorization level: {:?}", footprint_generator);

    // Initialize tool registry and register built-in tools
    let tool_registry = Arc::new(ToolRegistry::new());
    register_builtin_tools(&tool_registry).await?;
    tracing::info!("Registered {} tools", tool_registry.len().await);

    // Initialize plugin registry
    let plugin_dir = PathBuf::from(&config.cache_dir).join("plugins");
    let plugin_registry = Arc::new(PluginRegistry::new(&plugin_dir));
    
    // Register system plugin metadata (placeholder until full plugin loading is restored)
    let _system_plugin = PluginCore {
        name: "system".to_string(),
        version: "1.0.0".to_string(),
        description: "Core system plugin".to_string(),
        ..Default::default()
    };
    
    // The new registry requires a BoxedPlugin trait object, not just metadata.
    // For now, we skip manual registration of "system" plugin as tools are registered directly.
    // plugin_registry.register_core(system_plugin);
    // plugin_registry.register_tunables("system", TunableScope::Global, PluginTunables::default());

    // Initialize blockchain (StreamingBlockchain)
    let blockchain_path = PathBuf::from(&config.cache_dir).join("blockchain");
    let blockchain_stream = StreamingBlockchain::new(blockchain_path).await?;
    // DbusProjection expects Arc<parking_lot::RwLock<StreamingBlockchain>>
    let blockchain = Arc::new(parking_lot::RwLock::new(blockchain_stream));

    // Initialize cache
    let cache = Arc::new(BtrfsCache::new(PathBuf::from(&config.cache_dir)).await?);

    // Create orchestrator
    let orchestrator = Arc::new(Orchestrator::new(
        OrchestratorConfig::default(),
        tool_registry.clone(),
        plugin_registry.clone(),
    ));

    // Create MCP dispatchers
    let mcp_compact = Arc::new(McpCompactDispatcher::new(
        tool_registry.clone(),
    ));

    let mcp_live = Arc::new(McpLiveDispatcher::new(
        tool_registry.clone(),
    ));

    // Create policy engine
    let policy_engine = Arc::new(PolicyEngine::new(state_store.clone()));
    policy_engine.load_policies().await?;
    tracing::info!("Policy engine initialized");

    // Create Inspector Gadget (one-shot only)
    let inspector = Arc::new(InspectorGadget::new(
        InspectorConfig::default(),
        state_store.clone(),
        plugin_registry.clone(),
        tool_registry.clone(),
    ));
    tracing::info!("Inspector Gadget ready (one-shot discovery only)");

    // Create disaster recovery
    let disaster_recovery = Arc::new(DisasterRecovery::new(state_store.clone()));
    tracing::info!("Disaster recovery ready");

    // Create dependency manager
    let mut dependency_manager = DependencyManager::new(state_store.clone());
    dependency_manager.init().await?;
    let dependency_manager = Arc::new(dependency_manager);
    tracing::info!("Dependency manager initialized");

    // Create chatbot (cognitive brain - reasons but never executes directly)
    let _chatbot = Arc::new(
        Chatbot::new(
            ChatbotConfig::default(),
            mcp_compact.clone(),
            mcp_live.clone(),
            state_store.clone(),
        )
        .with_policy_engine(policy_engine.clone())
        .with_inspector(inspector.clone())
        .with_disaster_recovery(disaster_recovery.clone())
        .with_dependency_manager(dependency_manager.clone())
    );
    tracing::info!("Chatbot initialized (reasoning only, no direct execution)");

    // D-Bus projection - READ from database OR discover if needed
    // WRITE only on: onboarding, upgrade, migration, chatbot changes
    let _dbus_projection = if config.enable_dbus {
        let projection = DbusProjection::new()
            .with_blockchain(blockchain.clone());

        // Check if we need to discover (onboarding/upgrade/migration)
        let force_discovery = std::env::var("OP_DBUS_FORCE_DISCOVERY")
            .map(|v| v == "1" || v.to_lowercase() == "true")
            .unwrap_or(false);

        let needs_discovery = force_discovery; // TODO: Check if tools empty via state_store

        if needs_discovery {
            tracing::info!("🔍 D-Bus projection: discovering tools (onboarding/upgrade/force)...");

            use op_introspection::IntrospectionService;
            let introspection = Arc::new(IntrospectionService::new());
            let engine = op_dbus::projection::ProjectionEngine::new(introspection);

            // Discover session bus tools (blocks until complete)
            match engine.discover_all(&tool_registry, op_core::BusType::Session).await {
                Ok(count) => tracing::info!("✅ D-Bus projection (session): {} tools", count),
                Err(e) => tracing::error!("❌ D-Bus projection (session) failed: {}", e),
            }

            // Discover system bus tools (blocks until complete)
            match engine.discover_all(&tool_registry, op_core::BusType::System).await {
                Ok(count) => tracing::info!("✅ D-Bus projection (system): {} tools", count),
                Err(e) => tracing::error!("❌ D-Bus projection (system) failed: {}", e),
            }

            let final_count = tool_registry.len().await;
            tracing::info!("🎯 Total tools discovered: {}", final_count);

            // TODO: Save to database for next startup
            // state_store.save_tools(tool_registry.export()).await
        } else {
            tracing::info!("📖 D-Bus projection: reading tools from database (instant startup)");
            // TODO: Load from database
            // tool_registry.import(state_store.load_tools().await)
        }

        Some(projection)
    } else {
        None
    };

    // Start Antigravity tunnel if enabled (DEVELOPMENT ONLY)
    #[cfg(feature = "dev-antigravity")]
    let _antigravity_handle = if config.enable_antigravity {
        let transport_type = match config.antigravity_transport.to_lowercase().as_str() {
            "stdio" => TransportType::Stdio,
            "tcp" => TransportType::Tcp,
            "websocket" | "ws" => TransportType::WebSocket,
            _ => TransportType::Tcp,
        };

        let antigravity_config = AntigravityConfig {
            enabled: true,
            transport: TransportConfig {
                transport_type,
                listen_addr: config.antigravity_listen.clone(),
                tls: false,
            },
            session_timeout_secs: constants::ANTIGRAVITY_SESSION_TIMEOUT_SECS,
            track_billing: true,
            allowed_ides: vec![],
            max_sessions: 100,
        };

        let tunnel = Arc::new(AntigravityTunnel::new(
            antigravity_config,
            mcp_compact.clone(),
            orchestrator.clone(),
        ));

        let transport = TunnelTransport::new(TransportConfig {
            transport_type,
            listen_addr: config.antigravity_listen.clone(),
            tls: false,
        });

        let tunnel_clone = tunnel.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = transport.start(tunnel_clone).await {
                tracing::error!("Antigravity tunnel error: {}", e);
            }
        });

        tracing::info!("Antigravity tunnel started at {}", config.antigravity_listen);
        Some(handle)
    } else {
        None
    };

    // Start web server if enabled
    if config.enable_web {
        tracing::info!("Starting web interface at http://{}:{}", config.web_host, config.web_port);
        let addr: SocketAddr = format!("{}:{}", config.web_host, config.web_port)
            .parse()
            .map_err(|e| op_dbus::error::OpDbusError::ConfigError(format!("Invalid OP_DBUS_WEB_HOST/PORT: {}", e)))?;

        // Share the tool_registry with web server (avoids duplicating 16k+ D-Bus tools)
        let web_state = Arc::new(AppState::new_with_registry(Some(tool_registry.clone())).await?);
        let app = routes::create_router(web_state);

        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(e) => {
                    tracing::error!("Web server bind error: {}", e);
                    return;
                }
            };
            if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await {
                tracing::error!("Web server error: {}", e);
            }
        });
    }

    // TODO: Start gRPC server (disabled due to op-mcp compilation errors)
    // Will be enabled after fixing simd-json API issues in op-mcp
    /*
    #[cfg(feature = "grpc")]
    if std::env::var("OP_DBUS_ENABLE_GRPC").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false) {
        tracing::info!("gRPC server support temporarily disabled");
    }
    */
    #[cfg(feature = "grpc")]
    if std::env::var("OP_DBUS_ENABLE_GRPC").map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false) {
        let addr = std::env::var("OP_DBUS_GRPC_ADDR").unwrap_or_else(|_| "0.0.0.0:50051".to_string());
        let socket_addr: std::net::SocketAddr = addr.parse().map_err(|e| {
            op_dbus::error::OpDbusError::ConfigError(format!("Invalid OP_DBUS_GRPC_ADDR: {}", e))
        })?;

        let chain = Arc::new(tokio::sync::RwLock::new(op_state_store::EventChain::new(ChainConfig::default())));
        let sync_engine = Arc::new(SyncEngine::new(chain));

        // Start D-Bus watcher to push property changes into the sync engine.
        let mut watcher = DbusWatcher::new(WatchConfig::default(), sync_engine.clone());
        if let Err(e) = watcher.connect().await {
            tracing::warn!("D-Bus watcher connect failed: {}", e);
        } else if let Err(e) = watcher.start().await {
            tracing::warn!("D-Bus watcher start failed: {}", e);
        } else {
            let watcher = Arc::new(watcher);
            // Register plugin base paths for routing.
            for plugin in op_dbus::plugins::plugin_definitions() {
                let mut schema = plugin.schema_json.to_string();
                if let Ok(schema_value) = unsafe { simd_json::from_str::<simd_json::OwnedValue>(&mut schema) } {
                    if let Some(object_types) = schema_value.as_object().and_then(|o| o.get("object_types")).and_then(|v| v.as_object()) {
                        for (_name, entry_value) in object_types {
                            if let Some(path) = entry_value.as_object().and_then(|o| o.get("base_path")).and_then(|v| v.as_str()) {
                                watcher.register_path(path.to_string(), plugin.name.to_string()).await;
                            }
                        }
                    }
                }
            }
            watcher.spawn();
        }

        let plugin_provider = Arc::new(OpdbusPluginProvider);
        tokio::spawn(async move {
            tracing::info!("Starting gRPC server at {}", socket_addr);
            if let Err(e) = run_grpc_server(socket_addr, sync_engine, Some(plugin_provider)).await {
                tracing::error!("gRPC server error: {}", e);
            }
        });
    }

    // Run JSON-RPC server (blocking)
    match config.listen.as_str() {
        "stdio" => run_stdio_server(mcp_compact).await?,
        listen if listen.starts_with("tcp:") => {
            let addr = listen.strip_prefix("tcp:").unwrap();
            run_tcp_server(addr, mcp_compact).await?;
        }
        listen if listen.starts_with("unix:") => {
            let path = listen.strip_prefix("unix:").unwrap();
            run_unix_server(path, mcp_compact).await?;
        }
        "none" => {
            tracing::info!("Running in web-only mode. Press Ctrl+C to stop.");
            tokio::signal::ctrl_c().await?;
        }
        _ => {
            tracing::error!("Unknown listen address: {}", config.listen);
        }
    }

    tracing::info!("OP-DBUS shutdown complete");
    Ok(())
}

async fn run_stdio_server(dispatcher: Arc<McpCompactDispatcher>) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    tracing::info!("JSON-RPC server listening on stdio");

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let response = process_request(&dispatcher, &line).await;
        let response_json = simd_json::to_string(&response).unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"Serialization error"}}"#.to_string()
        });

        stdout.write_all(response_json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn run_tcp_server(addr: &str, dispatcher: Arc<McpCompactDispatcher>) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("JSON-RPC server listening on tcp://{}", addr);

    loop {
        let (socket, peer) = listener.accept().await?;
        let dispatcher = dispatcher.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_tcp_connection(socket, dispatcher).await {
                tracing::error!("Connection error from {}: {}", peer, e);
            }
        });
    }
}

async fn handle_tcp_connection(
    socket: tokio::net::TcpStream,
    dispatcher: Arc<McpCompactDispatcher>,
) -> Result<()> {
    let (reader, mut writer) = socket.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let response = process_request(&dispatcher, &line).await;
        let response_json = simd_json::to_string(&response).unwrap_or_default();

        writer.write_all(response_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
    }

    Ok(())
}

#[cfg(unix)]
async fn run_unix_server(path: &str, dispatcher: Arc<McpCompactDispatcher>) -> Result<()> {
    let _ = std::fs::remove_file(path);
    let listener = tokio::net::UnixListener::bind(path)?;
    tracing::info!("JSON-RPC server listening on unix://{}", path);

    loop {
        let (socket, _) = listener.accept().await?;
        let dispatcher = dispatcher.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let reader = BufReader::new(reader);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await.unwrap_or(None) {
                if line.trim().is_empty() {
                    continue;
                }

                let response = process_request(&dispatcher, &line).await;
                let response_json = simd_json::to_string(&response).unwrap_or_default();

                let _ = writer.write_all(response_json.as_bytes()).await;
                let _ = writer.write_all(b"\n").await;
            }
        });
    }
}

#[cfg(not(unix))] 
async fn run_unix_server(_path: &str, _dispatcher: Arc<McpCompactDispatcher>) -> Result<()> {
    Err(op_dbus::error::OpDbusError::ConfigError(
        "Unix sockets not supported on this platform".into(),
    ))
}

async fn process_request(dispatcher: &McpCompactDispatcher, input: &str) -> JsonRpcResponse {
    let mut input_mut = input.to_string();
    let request: JsonRpcRequest = match unsafe { simd_json::from_str(&mut input_mut) } {
        Ok(req) => req,
        Err(e) => {
            return JsonRpcResponse::error_with_code(
                simd_json::OwnedValue::from(()),
                op_dbus::json_rpc::error_codes::PARSE_ERROR,
                format!("Parse error: {}", e),
            );
        }
    };

    let id = request.id.clone();
    let mcp_request = McpRequest::from(request);
    
    let mcp_response = dispatcher.handle_request(mcp_request).await;

    // Convert McpResponse back to JsonRpcResponse
    if let Some(error) = mcp_response.error {
        JsonRpcResponse::error(
            id,
            JsonRpcError {
                code: error.code,
                message: error.message,
                data: error.data,
            },
        )
    } else {
        JsonRpcResponse::success(
            id,
            mcp_response.result.unwrap_or(simd_json::OwnedValue::from(())),
        )
    }
}
