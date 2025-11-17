//! op-dbus - Operation D-Bus
//! Declarative system state management via native protocols

mod blockchain;
mod cache;
#[cfg(feature = "ml")]
mod ml;
mod native;
mod nonnet_db;
mod state;
mod webui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use sha2::Digest;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::info;


#[derive(Parser)]
#[command(
    name = "op-dbus",
    version,
    about = "Declarative system state via native protocols"
)]
struct Cli {
    #[arg(short, long)]
    state_file: Option<PathBuf>,

    #[arg(short = 't', long)]
    enable_dhcp_server: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the daemon (default)
    Run {
        #[arg(long)]
        oneshot: bool,
    },

    /// Apply desired state from file
    Apply {
        state_file: PathBuf,
        #[arg(long)]
        dry_run: bool,
        /// Only apply to specific plugin (e.g., lxc, net, systemd)
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Query current system state
    Query {
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Show diff between current and desired state
    Diff {
        state_file: PathBuf,
        #[arg(short, long)]
        plugin: Option<String>,
    },

    /// Verify current state matches last footprint
    Verify {
        #[arg(long)]
        full: bool,
    },

    /// Blockchain operations
    #[command(subcommand)]
    Blockchain(BlockchainCommands),

    /// Container management (LXC)
    #[command(subcommand)]
    Container(ContainerCommands),

    /// Apply state to a specific container only
    ApplyContainer {
        /// Container ID (e.g., 100, 101)
        container_id: String,
        /// Container state file or use main state file
        #[arg(short, long)]
        state_file: Option<PathBuf>,
    },

    /// Initialize configuration file
    Init {
        #[arg(long)]
        introspect: bool,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// System diagnostics
    Doctor,

    /// Show version information
    Version {
        #[arg(long)]
        verbose: bool,
    },

    /// Introspect system databases
    Introspect {
        /// Database to query: ovsdb, nonnet, or all (default: all)
        #[arg(short, long)]
        database: Option<String>,

        /// Pretty print JSON output
        #[arg(short, long)]
        pretty: bool,
    },

    /// Cache management
    #[command(subcommand)]
    Cache(CacheCommands),

    /// Start web UI server
    Serve {
        /// Bind address
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,

        /// Port
        #[arg(short, long, default_value = "9573")]
        port: u16,
    },

    /// D-Bus index management (hierarchical abstraction layer)
    #[command(subcommand)]
    Index(IndexCommands),
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats,

    /// Clear cache
    Clear {
        #[arg(long)]
        embeddings: bool,
        #[arg(long)]
        blocks: bool,
        #[arg(long)]
        all: bool,
    },

    /// Clean old cache entries
    Clean {
        #[arg(long, default_value = "90")]
        older_than_days: i64,
    },

    /// Create cache snapshot
    Snapshot,

    /// List cache snapshots
    Snapshots,

    /// Delete all snapshots
    DeleteSnapshots,
}

#[derive(Subcommand)]
enum IndexCommands {
    /// Build complete D-Bus index (unlimited scan)
    Build {
        /// Output directory (default: /var/lib/op-dbus/@dbus-index)
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        output: PathBuf,
    },

    /// Update existing index (incremental)
    Update {
        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,
    },

    /// Search D-Bus index
    Search {
        /// Search query (service/object/method name)
        query: String,

        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,
    },

    /// Show index statistics
    Stats {
        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,
    },

    /// Diff two D-Bus index snapshots
    Diff {
        /// First index (baseline)
        baseline: PathBuf,
        /// Second index (current)
        current: PathBuf,
    },

    /// List all snapshots
    Snapshots {
        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,
    },

    /// Create a snapshot manually
    Snapshot {
        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,

        /// Snapshot name (optional, defaults to timestamp)
        #[arg(short, long)]
        name: Option<String>,

        /// Tag this snapshot (prevents auto-deletion)
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Clean up old snapshots (apply retention policy)
    Cleanup {
        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,

        /// Force cleanup without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Verify index completeness against live D-Bus
    Verify {
        /// Index directory
        #[arg(short, long, default_value = "/var/lib/op-dbus/@dbus-index")]
        index: PathBuf,

        /// Show detailed missing services
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Subcommand)]
enum BlockchainCommands {
    /// List blockchain blocks
    List {
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Show specific block
    Show { block_id: String },

    /// Export blockchain
    Export {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Verify blockchain integrity
    Verify {
        #[arg(long)]
        full: bool,
    },

    /// Search blockchain for changes
    Search { query: String },
}

#[derive(Subcommand)]
enum ContainerCommands {
    /// List containers
    List {
        #[arg(long)]
        running: bool,
        #[arg(long)]
        stopped: bool,
    },

    /// Show container details
    Show { container_id: String },

    /// Create container
    Create {
        container_id: String,
        #[arg(long, default_value = "bridge")]
        network_type: String,
    },

    /// Start container
    Start { container_id: String },

    /// Stop container
    Stop { container_id: String },

    /// Destroy container
    Destroy { container_id: String },
}

fn init_logging() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::from_default_env().add_directive("op_dbus=info".parse().unwrap());
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;
    Ok(())
}

async fn apply_state_from_file(
    state_manager: &state::StateManager,
    state_file: &std::path::Path,
) -> Result<()> {
    info!("Loading desired state from: {}", state_file.display());
    let desired_state = state_manager.load_desired_state(state_file).await?;
    let report = state_manager.apply_state(desired_state).await?;
    if report.success {
        info!("Successfully applied desired state");
    }
    Ok(())
}

async fn apply_state_from_file_single_plugin(
    state_manager: &state::StateManager,
    state_file: &std::path::Path,
    plugin_name: &str,
) -> Result<()> {
    info!("Loading desired state from: {}", state_file.display());
    let desired_state = state_manager.load_desired_state(state_file).await?;
    let report = state_manager
        .apply_state_single_plugin(desired_state, plugin_name)
        .await?;
    if report.success {
        info!("Successfully applied state for plugin: {}", plugin_name);
    }
    Ok(())
}

async fn setup_dhcp_server() -> Result<()> {
    info!("Setting up DHCP server...");

    // Install dnsmasq (lightweight DHCP and DNS server)
    let output = tokio::process::Command::new("apt")
        .args(["update"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to update package list"));
    }

    let output = tokio::process::Command::new("apt")
        .args(["install", "-y", "dnsmasq"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to install dnsmasq"));
    }

    // Create basic dnsmasq configuration for DHCP server
    let dhcp_config = r#"# DHCP server configuration
interface=ovsbr0
dhcp-range=192.168.1.50,192.168.1.150,12h
dhcp-option=option:router,192.168.1.1
dhcp-option=option:dns-server,8.8.8.8,8.8.4.4
dhcp-authoritative
"#;

    fs::write("/etc/dnsmasq.d/op-dbus-dhcp.conf", dhcp_config).await?;

    // Enable and start dnsmasq
    let output = tokio::process::Command::new("systemctl")
        .args(["enable", "dnsmasq"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to enable dnsmasq"));
    }

    let output = tokio::process::Command::new("systemctl")
        .args(["restart", "dnsmasq"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to start dnsmasq"));
    }

    info!("DHCP server setup complete");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;
    let args = Cli::parse();

    let state_manager = Arc::new(state::StateManager::new());

    // Register core plugins manually
    let plugins = vec![
        ("net", Arc::new(state::plugins::NetStatePlugin::new()) as Arc<dyn crate::state::plugin::StatePlugin>),
        ("systemd", Arc::new(state::plugins::SystemdStatePlugin::new())),
        ("login1", Arc::new(state::plugins::Login1Plugin::new())),
        ("lxc", Arc::new(state::plugins::LxcPlugin::new())),
        ("sessdecl", Arc::new(state::plugins::SessDeclPlugin::new())),
        ("dns", Arc::new(state::plugins::DnsResolverPlugin::new())),
        ("pcidecl", Arc::new(state::plugins::PciDeclPlugin::new())),
        ("packagekit", Arc::new(state::plugins::PackageKitPlugin::new())),
        #[cfg(feature = "openflow")]
        ("privacy", Arc::new(state::plugins::PrivacyPlugin::new(Default::default()))),
        #[cfg(feature = "openflow")]
        ("netmaker", Arc::new(state::plugins::NetmakerPlugin::new(Default::default()))),
    ];

    for (name, plugin) in plugins {
        state_manager.register_plugin(plugin.clone()).await;
        // Also register as workflow node
        state_manager.register_plugin_as_workflow_node(name, plugin);
    }

    // Discover and register auto-generated plugins
    #[cfg(feature = "mcp")]
    {
        if let Err(e) = state_manager.discover_and_register_auto_plugins().await {
            log::warn!("Failed to discover auto plugins: {}", e);
        }
    }

    // Setup default workflows
    if let Err(e) = state_manager.setup_default_workflows().await {
        log::warn!("Failed to setup default workflows: {}", e);
    }

    // Start org.opdbus on system D-Bus to accept ApplyState calls for net plugin
    {
        let sm = Arc::clone(&state_manager);
        tokio::spawn(async move {
            if let Err(e) = crate::state::dbus_server::start_system_bus(sm).await {
                log::warn!("Failed to start org.opdbus service: {}", e);
            } else {
                log::info!("D-Bus service started: org.opdbus at /org/opdbus/state/net");
            }
        });
    }

    // Start non-network JSON-RPC DB (unix socket) for plugin state, OVSDB-like, read-only
    {
        let sm = Arc::clone(&state_manager);
        tokio::spawn(async move {
            if let Err(e) = nonnet_db::run_unix_jsonrpc(sm, "/run/op-dbus/nonnet.db.sock").await {
                info!("nonnet DB server exited: {}", e);
            }
        });
    }

    match args.command.unwrap_or(Commands::Run { oneshot: false }) {
        Commands::Run { oneshot } => {
            // Set up DHCP server if requested
            if args.enable_dhcp_server {
                setup_dhcp_server().await?;
            }

            let state_file = args
                .state_file
                .unwrap_or_else(|| PathBuf::from("/etc/op-dbus/state.json"));
            if state_file.exists() {
                apply_state_from_file(&state_manager, &state_file).await?;
            }

            if oneshot {
                info!("Oneshot mode: exiting after apply");
                return Ok(());
            }

            info!("Daemon running, press Ctrl+C to stop");
            tokio::signal::ctrl_c().await?;
            Ok(())
        }

        Commands::Apply {
            state_file,
            dry_run,
            plugin,
        } => {
            if dry_run {
                info!("DRY RUN: Showing what would be applied");
                let desired = state_manager.load_desired_state(&state_file).await?;
                let diffs = state_manager.show_diff(desired).await?;

                // Filter by plugin if specified
                let filtered_diffs: Vec<_> = if let Some(ref p) = plugin {
                    diffs.into_iter().filter(|d| &d.plugin == p).collect()
                } else {
                    diffs
                };

                println!("{}", serde_json::to_string_pretty(&filtered_diffs)?);
            } else if let Some(plugin_name) = plugin {
                info!("Applying state for plugin: {}", plugin_name);
                apply_state_from_file_single_plugin(&state_manager, &state_file, &plugin_name)
                    .await?;
            } else {
                info!("??  WARNING: Applying state to ALL plugins system-wide");
                info!("??  Consider using --plugin flag to limit scope");
                apply_state_from_file(&state_manager, &state_file).await?;
            }
            Ok(())
        }

        Commands::Query { plugin } => {
            let state = if let Some(p) = plugin {
                state_manager.query_plugin_state(&p).await?
            } else {
                serde_json::to_value(&state_manager.query_current_state().await?)?
            };
            println!("{}", serde_json::to_string_pretty(&state)?);
            Ok(())
        }

        Commands::Diff {
            state_file,
            plugin: _,
        } => {
            let desired = state_manager.load_desired_state(&state_file).await?;
            let diffs = state_manager.show_diff(desired).await?;
            println!("{}", serde_json::to_string_pretty(&diffs)?);
            Ok(())
        }

        Commands::Verify { full } => {
            info!("Verifying state against blockchain footprint");

            let blockchain_path = PathBuf::from("/var/lib/op-dbus/blockchain");

            if !blockchain_path.exists() {
                println!("? No blockchain found - nothing to verify");
                println!("Run 'op-dbus apply' to create initial state footprints");
                return Ok(());
            }
            let timing_path = blockchain_path.join("timing");
            if !timing_path.exists() {
                println!("? No footprints found in blockchain");
                return Ok(());
            }

            println!("=== State Verification ===\n");

            // Query current state
            let current_state = state_manager.query_current_state().await?;
            println!("? Current state queried successfully");

            // Read blockchain footprints
            let mut footprints = Vec::new();
            let mut entries = fs::read_dir(&timing_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(block) = serde_json::from_str::<serde_json::Value>(&content) {
                            footprints.push(block);
                        }
                    }
                }
            }

            println!("? Found {} blockchain footprints", footprints.len());

            if full {
                println!("\n--- Full Verification ---");

                // Verify blockchain integrity (hash chain)
                let mut hash_issues = 0;
                for footprint in &footprints {
                    if let Some(hash) = footprint["hash"].as_str() {
                        let category = footprint["category"].as_str().unwrap_or("");
                        let action = footprint["action"].as_str().unwrap_or("");
                        let timestamp = footprint["timestamp"].as_u64().unwrap_or(0);

                        let content = format!("{}:{}:{}", category, action, timestamp);
                        let calculated = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));

                        if calculated != hash {
                            println!("? Hash mismatch in footprint {}", &hash[..16]);
                            hash_issues += 1;
                        }
                    }
                }

                if hash_issues == 0 {
                    println!("? All footprint hashes are valid");
                } else {
                    println!("? Found {} hash integrity issues", hash_issues);
                }

                // Verify vector data consistency
                let vector_path = blockchain_path.join("vectors");
                let mut vector_issues = 0;

                for footprint in &footprints {
                    if let Some(hash) = footprint["hash"].as_str() {
                        let vec_file = vector_path.join(format!("{}.vec", hash));
                        if !vec_file.exists() {
                            println!("? Missing vector file for footprint {}", &hash[..16]);
                            vector_issues += 1;
                        }
                    }
                }

                if vector_issues == 0 {
                    println!("? All vector files present");
                } else {
                    println!("? Found {} missing vector files", vector_issues);
                }

                // Verify snapshots
                let snapshot_dir = blockchain_path.join("snapshots");
                if snapshot_dir.exists() {
                    let mut snapshot_count = 0;
                    let mut snapshot_entries = fs::read_dir(&snapshot_dir).await?;
                    while let Some(_entry) = snapshot_entries.next_entry().await? {
                        snapshot_count += 1;
                    }
                    println!("? Found {} BTRFS snapshots", snapshot_count);
                } else {
                    println!("? No snapshots directory found");
                }
            } else {
                // Basic verification - just check if state is consistent
                println!("\n--- Basic Verification ---");

                // Verify each plugin's state
                for (plugin_name, _plugin_state) in current_state.plugins.iter() {
                    // Count footprints for this plugin
                    let plugin_footprints: Vec<_> = footprints
                        .iter()
                        .filter(|f| f["category"].as_str() == Some(plugin_name))
                        .collect();

                    if !plugin_footprints.is_empty() {
                        println!(
                            "? Plugin '{}': {} footprints found",
                            plugin_name,
                            plugin_footprints.len()
                        );
                    }
                }
            }

            println!("\n=== Verification Complete ===");
            Ok(())
        }

        Commands::Blockchain(cmd) => handle_blockchain_command(cmd).await,

        Commands::Container(cmd) => handle_container_command(cmd, &state_manager).await,

        Commands::ApplyContainer {
            container_id,
            state_file,
        } => {
            info!("Applying state for container: {}", container_id);

            let state_path = state_file.unwrap_or_else(|| PathBuf::from("/etc/op-dbus/state.json"));
            let desired_state = state_manager.load_desired_state(&state_path).await?;

            // Find the container in the desired state
            if let Some(lxc_state) = desired_state.plugins.get("lxc") {
                let lxc_config: crate::state::plugins::lxc::LxcState =
                    serde_json::from_value(lxc_state.clone())?;

                if let Some(container) = lxc_config.containers.iter().find(|c| c.id == container_id)
                {
                    // Get LXC plugin and apply single container
                    let lxc_plugin = crate::state::plugins::LxcPlugin::new();
                    let result = lxc_plugin.apply_container_state(container).await?;

                    if result.success {
                        println!("? Container {} applied successfully", container_id);
                        for change in &result.changes_applied {
                            println!("  - {}", change);
                        }
                    } else {
                        println!("? Container {} apply failed", container_id);
                        for error in &result.errors {
                            println!("  - ERROR: {}", error);
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "Container {} not found in state file",
                        container_id
                    ));
                }
            } else {
                return Err(anyhow::anyhow!("No LXC plugin configuration in state file"));
            }

            Ok(())
        }

        Commands::Init { introspect, output } => {
            info!("Initializing configuration");
            if introspect {
                // Query current state and wrap with metadata so the result is a valid state file
                let current = state_manager.query_current_state().await?;
                let state_json = serde_json::json!({
                    "version": 1,
                    "plugins": current.plugins,
                });
                let json = serde_json::to_string_pretty(&state_json)?;

                if let Some(out_path) = output {
                    fs::write(&out_path, json).await?;
                    println!("? Configuration written to: {}", out_path.display());
                } else {
                    println!("{}", json);
                }
            } else {
                // Create minimal template
                let template = r#"{
  "version": 1,
  "plugins": {
    "net": {
      "interfaces": []
    },
    "systemd": {
      "units": {}
    }
  }
}"#;
                if let Some(out_path) = output {
                    fs::write(&out_path, template).await?;
                    println!("? Template written to: {}", out_path.display());
                } else {
                    println!("{}", template);
                }
            }
            Ok(())
        }

        Commands::Introspect { database, pretty } => {
            let db_choice = database.as_deref().unwrap_or("all");
            let mut results = serde_json::Map::new();

            // Introspect OVSDB (OVS network state)
            if db_choice == "all" || db_choice == "ovsdb" {
                info!("Introspecting OVSDB (Open vSwitch)...");
                let ovsdb_client = crate::native::OvsdbClient::new();

                match ovsdb_client.list_dbs().await {
                    Ok(dbs) => {
                        let mut ovsdb_data = serde_json::Map::new();
                        ovsdb_data.insert("databases".to_string(), serde_json::json!(dbs));

                        // Get Open_vSwitch database content
                        if dbs.contains(&"Open_vSwitch".to_string()) {
                            if let Ok(bridges) = ovsdb_client.list_bridges().await {
                                let mut bridge_details = Vec::new();
                                for bridge in &bridges {
                                    if let Ok(info) = ovsdb_client.get_bridge_info(bridge).await {
                                        if let Ok(parsed) =
                                            serde_json::from_str::<serde_json::Value>(&info)
                                        {
                                            bridge_details.push(parsed);
                                        }
                                    }
                                }
                                ovsdb_data.insert(
                                    "bridges".to_string(),
                                    serde_json::json!(bridge_details),
                                );
                            }
                        }

                        results.insert("ovsdb".to_string(), serde_json::Value::Object(ovsdb_data));
                    }
                    Err(e) => {
                        results.insert(
                            "ovsdb".to_string(),
                            serde_json::json!({
                                "error": format!("Failed to connect: {}", e)
                            }),
                        );
                    }
                }
            }

            // Introspect NonNet DB (Plugin state: systemd, login1, lxc)
            if db_choice == "all" || db_choice == "nonnet" {
                info!("Introspecting NonNet DB (Plugin state)...");

                // Query all plugin states via state manager
                match state_manager.query_current_state().await {
                    Ok(current) => {
                        let mut nonnet_data = serde_json::Map::new();

                        // Extract non-network plugins
                        for (plugin_name, plugin_state) in current.plugins.iter() {
                            if plugin_name != "net" {
                                nonnet_data.insert(plugin_name.clone(), plugin_state.clone());
                            }
                        }

                        results
                            .insert("nonnet".to_string(), serde_json::Value::Object(nonnet_data));
                    }
                    Err(e) => {
                        results.insert(
                            "nonnet".to_string(),
                            serde_json::json!({
                                "error": format!("Failed to query: {}", e)
                            }),
                        );
                    }
                }
            }

            // Output results
            let output = if pretty {
                serde_json::to_string_pretty(&results)?
            } else {
                serde_json::to_string(&results)?
            };

            println!("{}", output);
            Ok(())
        }

        Commands::Doctor => {
            println!("=== op-dbus System Diagnostics ===\n");

            // Check binary
            println!("? op-dbus binary running");

            // Check OVSDB
            print!("Checking OVSDB connection... ");
            match crate::native::OvsdbClient::new().list_dbs().await {
                Ok(_) => println!("?"),
                Err(e) => println!("? Failed: {}", e),
            }

            // Check D-Bus
            print!("Checking D-Bus connection... ");
            match zbus::Connection::system().await {
                Ok(_) => println!("?"),
                Err(e) => println!("? Failed: {}", e),
            }

            // Check blockchain storage
            print!("Checking blockchain storage... ");
            if std::path::Path::new("/var/lib/op-dbus/blockchain").exists() {
                println!("?");
            } else {
                println!("? Not found");
            }

            // Check state file
            print!("Checking state file... ");
            if std::path::Path::new("/etc/op-dbus/state.json").exists() {
                println!("?");
            } else {
                println!("? Not found (run: op-dbus init --introspect)");
            }

            println!("\n=== Diagnostics Complete ===");
            Ok(())
        }

        Commands::Version { verbose } => {
            println!("op-dbus {}", env!("CARGO_PKG_VERSION"));
            if verbose {
                println!("Build: {}", env!("CARGO_PKG_VERSION"));
                println!("Rust: Pure Rust implementation");
                println!("Protocols: OVSDB, Netlink, D-Bus");
            }
            Ok(())
        }

        Commands::Cache(cmd) => handle_cache_command(cmd).await,

        Commands::Index(cmd) => handle_index_command(cmd).await,

        Commands::Serve { bind, port } => {
            info!("Starting web UI server on {}:{}", bind, port);

            let config = crate::webui::WebConfig {
                bind_addr: bind,
                port,
            };

            crate::webui::start_web_server(state_manager, config).await?;
            Ok(())
        }
    }
}

async fn handle_cache_command(cmd: CacheCommands) -> Result<()> {
    let cache_dir = PathBuf::from(
        std::env::var("OPDBUS_CACHE_DIR").unwrap_or_else(|_| "/var/lib/op-dbus/@cache".to_string()),
    );

    match cmd {
        CacheCommands::Stats => {
            println!("=== BTRFS Cache Statistics ===\n");

            let cache = crate::cache::BtrfsCache::new(cache_dir).await?;
            let stats = cache.stats()?;

            println!("Embeddings:");
            println!("  Total entries:    {}", stats.total_entries);
            println!(
                "  Hot (< 1h):       {} ({:.1}%)",
                stats.hot_entries,
                stats.hot_ratio() * 100.0
            );
            println!("  Average accesses: {:.1}", stats.avg_accesses());
            println!(
                "  Disk usage:       {:.2} MB",
                stats.embeddings_size_bytes as f64 / 1024.0 / 1024.0
            );

            println!("\nBlocks:");
            println!(
                "  Disk usage:       {:.2} MB",
                stats.blocks_size_bytes as f64 / 1024.0 / 1024.0
            );

            println!("\nTotal:");
            println!(
                "  Disk usage:       {:.2} MB (compressed)",
                stats.disk_usage_bytes as f64 / 1024.0 / 1024.0
            );

            // Show snapshots
            let snapshots = cache.list_snapshots().await?;
            println!("\nSnapshots:          {}", snapshots.len());
            if !snapshots.is_empty() {
                if let Some(oldest) = snapshots.first() {
                    println!("  Oldest:           {}", oldest.timestamp_str);
                }
                if let Some(newest) = snapshots.last() {
                    println!("  Newest:           {}", newest.timestamp_str);
                }
            }

            let numa = cache.numa_info();
            println!("\nNUMA:");
            println!("  Nodes:            {}", numa.node_count);
            println!("  CPU affinity:     {:?}", numa.cpu_affinity);
            println!("  Placement:        {:?}", numa.placement_strategy);
            println!("  Memory policy:    {:?}", numa.memory_policy);

            Ok(())
        }

        CacheCommands::Clear {
            embeddings,
            blocks,
            all,
        } => {
            let cache = crate::cache::BtrfsCache::new(cache_dir).await?;

            if all || (!embeddings && !blocks) {
                println!("Clearing all cache...");
                cache.clear()?;
                println!("? All cache cleared");
            } else {
                if embeddings {
                    println!("Clearing embeddings cache...");
                    cache.clear_embeddings()?;
                    println!("? Embeddings cache cleared");
                }
                if blocks {
                    println!("Clearing blocks cache...");
                    cache.clear_blocks()?;
                    println!("? Blocks cache cleared");
                }
            }

            Ok(())
        }

        CacheCommands::Clean { older_than_days } => {
            println!(
                "Cleaning cache entries older than {} days...",
                older_than_days
            );
            let cache = crate::cache::BtrfsCache::new(cache_dir).await?;
            let removed = cache.cleanup_old(older_than_days)?;
            println!("? Cleaned {} old entries", removed);
            Ok(())
        }

        CacheCommands::Snapshot => {
            println!("Creating cache snapshot...");
            let cache = crate::cache::BtrfsCache::new(cache_dir).await?;
            let snapshot_path = cache.create_snapshot().await?;
            println!("? Created snapshot: {}", snapshot_path.display());
            Ok(())
        }

        CacheCommands::Snapshots => {
            let cache = crate::cache::BtrfsCache::new(cache_dir).await?;
            let snapshots = cache.list_snapshots().await?;

            if snapshots.is_empty() {
                println!("No snapshots found");
            } else {
                println!("=== Cache Snapshots ({}) ===\n", snapshots.len());
                for snapshot in snapshots {
                    println!("  {} - {}", snapshot.timestamp_str, snapshot.path.display());
                }
            }

            Ok(())
        }

        CacheCommands::DeleteSnapshots => {
            println!("Deleting all cache snapshots...");
            let cache = crate::cache::BtrfsCache::new(cache_dir).await?;
            let count = cache.delete_all_snapshots().await?;
            println!("? Deleted {} snapshots", count);
            Ok(())
        }
    }
}

async fn handle_index_command(cmd: IndexCommands) -> Result<()> {
    use crate::mcp::dbus_indexer::{DbusIndexer, DbusQueryEngine};
    use crate::snapshot::{SnapshotManager, RetentionPolicy};

    match cmd {
        IndexCommands::Build { output } => {
            println!("🔍 Building complete D-Bus index...");
            println!("   Output: {}", output.display());
            println!();

            let indexer = DbusIndexer::new(&output).await?;
            let index = indexer.build_complete_index().await?;
            indexer.save(&index)?;

            println!();
            println!("✅ Index built successfully!");
            println!("   Location: {}/index.json", output.display());
            println!("   Services: {}", index.statistics.total_services);
            println!("   Objects: {}", index.statistics.total_objects);
            println!("   Methods: {}", index.statistics.total_methods);

            // Auto-create snapshot with rolling-3 retention
            println!();
            println!("📸 Creating snapshot...");
            let snapshots_dir = PathBuf::from("/var/lib/op-dbus/@snapshots/dbus-index");
            let snapshot_mgr = SnapshotManager::with_policy(
                &snapshots_dir,
                RetentionPolicy::Rolling { keep: 3 }
            );

            match snapshot_mgr.create_snapshot(&output, None) {
                Ok(snapshot_path) => {
                    println!("   Snapshot: {}", snapshot_path.display());
                    let snapshots = snapshot_mgr.list_snapshots()?;
                    println!("   Total snapshots: {} (keeping last 3)", snapshots.len());
                }
                Err(e) => {
                    log::warn!("Failed to create snapshot (non-fatal): {}", e);
                    println!("   ⚠️  Snapshot failed (index still saved): {}", e);
                }
            }

            println!();
            println!("💡 Use 'op-dbus index search <query>' to search the index");
            println!("💡 Use 'op-dbus index snapshots' to list all snapshots");

            Ok(())
        }

        IndexCommands::Update { index } => {
            println!("🔄 Updating D-Bus index at {}...", index.display());
            println!("   (Incremental update - only scans changed services)");
            println!();

            let indexer = DbusIndexer::new(&index).await?;
            // TODO: Implement incremental update
            println!("⚠️  Incremental update not yet implemented - use 'build' for now");

            Ok(())
        }

        IndexCommands::Search { query, index } => {
            let indexer = DbusIndexer::new(&index).await?;
            let dbus_index = indexer.load()?;
            let query_engine = DbusQueryEngine::new(dbus_index);

            let results = query_engine.search(&query);

            if results.is_empty() {
                println!("No results found for: {}", query);
            } else {
                println!("Found {} results for '{}':\n", results.len(), query);
                for result in results.iter().take(50) {
                    println!("  {}", result);
                }

                if results.len() > 50 {
                    println!("\n... and {} more results", results.len() - 50);
                }
            }

            Ok(())
        }

        IndexCommands::Stats { index } => {
            let indexer = DbusIndexer::new(&index).await?;
            let dbus_index = indexer.load()?;

            println!("=== D-Bus Index Statistics ===\n");
            println!("Index version:    {}", dbus_index.version);
            println!("Last updated:     {}", chrono::DateTime::from_timestamp(dbus_index.timestamp, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "Unknown".to_string()));
            println!();
            println!("Services:         {}", dbus_index.statistics.total_services);
            println!("Objects:          {}", dbus_index.statistics.total_objects);
            println!("Interfaces:       {}", dbus_index.statistics.total_interfaces);
            println!("Methods:          {}", dbus_index.statistics.total_methods);
            println!("Properties:       {}", dbus_index.statistics.total_properties);
            println!();
            println!("Scan duration:    {:.2}s", dbus_index.statistics.scan_duration_seconds);
            println!("Index location:   {}", index.display());

            Ok(())
        }

        IndexCommands::Diff { baseline, current } => {
            let baseline_indexer = DbusIndexer::new(&baseline).await?;
            let current_indexer = DbusIndexer::new(&current).await?;

            let baseline_index = baseline_indexer.load()?;
            let current_index = current_indexer.load()?;

            println!("=== D-Bus Index Diff ===\n");
            println!("Baseline: {} ({} services)",
                baseline.display(),
                baseline_index.statistics.total_services);
            println!("Current:  {} ({} services)\n",
                current.display(),
                current_index.statistics.total_services);

            // Services added
            let added: Vec<_> = current_index.services.keys()
                .filter(|k| !baseline_index.services.contains_key(*k))
                .collect();
            if !added.is_empty() {
                println!("Services added (+{}):", added.len());
                for service in &added {
                    println!("  + {}", service);
                }
                println!();
            }

            // Services removed
            let removed: Vec<_> = baseline_index.services.keys()
                .filter(|k| !current_index.services.contains_key(*k))
                .collect();
            if !removed.is_empty() {
                println!("Services removed (-{}):", removed.len());
                for service in &removed {
                    println!("  - {}", service);
                }
                println!();
            }

            if added.is_empty() && removed.is_empty() {
                println!("No differences found");
            }

            Ok(())
        }

        IndexCommands::Snapshots { index } => {
            let snapshots_dir = PathBuf::from("/var/lib/op-dbus/@snapshots/dbus-index");
            let snapshot_mgr = SnapshotManager::new(&snapshots_dir);
            let snapshots = snapshot_mgr.list_snapshots()?;

            if snapshots.is_empty() {
                println!("No snapshots found");
                println!("💡 Run 'op-dbus index build' to create the first snapshot");
            } else {
                println!("=== D-Bus Index Snapshots ({}) ===\n", snapshots.len());
                for snapshot in &snapshots {
                    let dt = chrono::DateTime::from_timestamp(snapshot.created, 0)
                        .map(|d| d.to_rfc3339())
                        .unwrap_or_else(|| "Unknown".to_string());

                    let tag_info = if snapshot.tagged {
                        format!(" [TAGGED: {}]", snapshot.tag.as_ref().unwrap_or(&"golden".to_string()))
                    } else {
                        String::new()
                    };

                    println!("  {} - {}{}", snapshot.name, dt, tag_info);
                }

                println!();
                println!("Total: {} snapshots", snapshots.len());
                println!("Location: {}", snapshots_dir.display());
            }

            Ok(())
        }

        IndexCommands::Snapshot { index, name, tag } => {
            println!("📸 Creating manual snapshot...");

            let snapshots_dir = PathBuf::from("/var/lib/op-dbus/@snapshots/dbus-index");
            let snapshot_mgr = SnapshotManager::with_policy(
                &snapshots_dir,
                RetentionPolicy::Rolling { keep: 3 }
            );

            let snapshot_path = snapshot_mgr.create_snapshot(&index, name.as_deref())?;
            println!("   Created: {}", snapshot_path.display());

            // Apply tag if provided
            if let Some(tag_value) = tag {
                let snapshot_name = snapshot_path.file_name().unwrap().to_string_lossy();
                snapshot_mgr.tag_snapshot(&snapshot_name, &tag_value)?;
                println!("   Tagged as: {}", tag_value);
            }

            let snapshots = snapshot_mgr.list_snapshots()?;
            println!("   Total snapshots: {}", snapshots.len());

            Ok(())
        }

        IndexCommands::Cleanup { index, force } => {
            let snapshots_dir = PathBuf::from("/var/lib/op-dbus/@snapshots/dbus-index");
            let snapshot_mgr = SnapshotManager::with_policy(
                &snapshots_dir,
                RetentionPolicy::Rolling { keep: 3 }
            );

            let snapshots = snapshot_mgr.list_snapshots()?;

            if snapshots.len() <= 3 {
                println!("✅ No cleanup needed - only {} snapshot(s) exist", snapshots.len());
                return Ok(());
            }

            println!("🗑️  Cleanup Policy: Keep last 3 snapshots");
            println!("   Current snapshots: {}", snapshots.len());
            println!("   Will delete: {} old snapshot(s)", snapshots.len() - 3);
            println!();

            if !force {
                print!("Continue? [y/N] ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            snapshot_mgr.apply_retention_policy()?;

            let remaining = snapshot_mgr.list_snapshots()?;
            println!("✅ Cleanup complete - {} snapshot(s) remaining", remaining.len());

            Ok(())
        }

        IndexCommands::Verify { index, verbose } => {
            println!("🔍 Verifying D-Bus index completeness...");
            println!();

            let indexer = DbusIndexer::new(&index).await?;
            let result = indexer.verify_completeness().await?;

            println!("=== D-Bus Index Verification ===\n");
            println!("Timestamp:         {}",
                chrono::DateTime::from_timestamp(result.timestamp, 0)
                    .map(|d| d.to_rfc3339())
                    .unwrap_or_else(|| "Unknown".to_string()));
            println!("Live services:     {}", result.live_services);
            println!("Indexed services:  {}", result.index_services);
            println!("Coverage:          {:.1}%", result.coverage_percent);
            println!();

            if result.index_complete {
                println!("✅ Index is COMPLETE - all live services are indexed");
            } else {
                println!("⚠️  Index is INCOMPLETE - {} service(s) missing", result.missing_from_index.len());

                if verbose || result.missing_from_index.len() <= 10 {
                    println!("\nMissing services:");
                    for service in &result.missing_from_index {
                        println!("  - {}", service);
                    }
                } else {
                    println!("\nMissing services (showing first 10 of {}):", result.missing_from_index.len());
                    for service in result.missing_from_index.iter().take(10) {
                        println!("  - {}", service);
                    }
                    println!("\n   Use --verbose to see all missing services");
                }
            }

            if !result.extra_in_index.is_empty() {
                println!("\n📝 Note: {} service(s) in index but not currently running",
                    result.extra_in_index.len());
                println!("   (These were indexed previously but the services have stopped)");

                if verbose && result.extra_in_index.len() <= 20 {
                    println!("\nExtra services:");
                    for service in &result.extra_in_index {
                        println!("  - {}", service);
                    }
                }
            }

            println!();
            if !result.index_complete {
                println!("💡 Run 'op-dbus index build' to update the index");
            }

            Ok(())
        }
    }
}

async fn handle_blockchain_command(cmd: BlockchainCommands) -> Result<()> {
    let blockchain_path = PathBuf::from("/var/lib/op-dbus/blockchain");

    match cmd {
        BlockchainCommands::List { limit } => {
            info!("Listing blockchain blocks");
            if !blockchain_path.exists() {
                println!("No blockchain found. Run 'op-dbus apply' to create genesis block.");
                return Ok(());
            }

            println!("Blockchain list (limit: {})", limit.unwrap_or(10));
            println!("? Not yet fully implemented");
            Ok(())
        }
        BlockchainCommands::Show { block_id } => {
            info!("Showing block: {}", block_id);
            if !blockchain_path.exists() {
                println!("No blockchain found.");
                return Ok(());
            }

            let timing_path = blockchain_path.join("timing");
            let vector_path = blockchain_path.join("vectors");

            // Find the block file (match by prefix since user might not enter full hash)
            let mut block_file = None;
            let mut entries = fs::read_dir(&timing_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if name.starts_with(&block_id) {
                        block_file = Some(path);
                        break;
                    }
                }
            }

            let Some(block_path) = block_file else {
                println!("Block not found: {}", block_id);
                return Ok(());
            };

            // Read and display block data
            let content = fs::read_to_string(&block_path).await?;
            let block: serde_json::Value = serde_json::from_str(&content)?;

            println!("=== Block Details ===\n");
            println!("{}", serde_json::to_string_pretty(&block)?);

            // Also show vector data if available
            if let Some(hash) = block["hash"].as_str() {
                let vec_file = vector_path.join(format!("{}.vec", hash));
                if vec_file.exists() {
                    let vec_content = fs::read_to_string(&vec_file).await?;
                    let vec_data: serde_json::Value = serde_json::from_str(&vec_content)?;

                    println!("\n=== Vector Features ===\n");
                    if let Some(vec) = vec_data["vector"].as_array() {
                        println!("Dimensions: {}", vec.len());
                        println!("First 10 values: {:?}", &vec[..10.min(vec.len())]);
                    }
                }
            }

            Ok(())
        }
        BlockchainCommands::Export { output } => {
            info!("Exporting blockchain");
            if !blockchain_path.exists() {
                println!("No blockchain found.");
                return Ok(());
            }

            let timing_path = blockchain_path.join("timing");

            // Read all blocks
            let mut blocks = Vec::new();
            let mut entries = fs::read_dir(&timing_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(block) = serde_json::from_str::<serde_json::Value>(&content) {
                            blocks.push(block);
                        }
                    }
                }
            }

            // Sort by timestamp
            blocks.sort_by(|a, b| {
                let ts_a = a["timestamp"].as_u64().unwrap_or(0);
                let ts_b = b["timestamp"].as_u64().unwrap_or(0);
                ts_a.cmp(&ts_b)
            });

            let export_data = serde_json::json!({
                "version": 1,
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "total_blocks": blocks.len(),
                "blocks": blocks
            });

            let json_output = serde_json::to_string_pretty(&export_data)?;

            if let Some(output_path) = output {
                fs::write(&output_path, json_output).await?;
                println!("? Blockchain exported to: {}", output_path.display());
            } else {
                println!("{}", json_output);
            }

            Ok(())
        }
        BlockchainCommands::Verify { full } => {
            info!("Verifying blockchain integrity");
            if !blockchain_path.exists() {
                println!("No blockchain found.");
                return Ok(());
            }

            let timing_path = blockchain_path.join("timing");

            // Read all blocks
            let mut blocks = Vec::new();
            let mut entries = fs::read_dir(&timing_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        if let Ok(block) = serde_json::from_str::<serde_json::Value>(&content) {
                            blocks.push(block);
                        }
                    }
                }
            }

            println!("=== Blockchain Verification ===\n");
            println!("Total blocks: {}", blocks.len());

            let mut issues = 0;

            // Verify each block's hash
            for block in &blocks {
                if let Some(hash) = block["hash"].as_str() {
                    if full {
                        // Full verification: recalculate hash
                        let category = block["category"].as_str().unwrap_or("");
                        let action = block["action"].as_str().unwrap_or("");
                        let timestamp = block["timestamp"].as_u64().unwrap_or(0);

                        let content = format!("{}:{}:{}", category, action, timestamp);
                        let calculated_hash =
                            format!("{:x}", sha2::Sha256::digest(content.as_bytes()));

                        if calculated_hash != hash {
                            println!("? Block {} has invalid hash", &hash[..16]);
                            issues += 1;
                        }
                    }
                } else {
                    println!("? Block missing hash field");
                    issues += 1;
                }
            }

            if issues == 0 {
                println!("? All blocks verified successfully");
            } else {
                println!("\n? Found {} issues", issues);
            }

            Ok(())
        }
        BlockchainCommands::Search { query } => {
            info!("Searching blockchain for: {}", query);
            if !blockchain_path.exists() {
                println!("No blockchain found.");
                return Ok(());
            }

            let timing_path = blockchain_path.join("timing");
            let query_lower = query.to_lowercase();

            // Search all blocks
            let mut matches = Vec::new();
            let mut entries = fs::read_dir(&timing_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path).await {
                        // Search in the content
                        if content.to_lowercase().contains(&query_lower) {
                            if let Ok(block) = serde_json::from_str::<serde_json::Value>(&content) {
                                matches.push(block);
                            }
                        }
                    }
                }
            }

            println!("=== Search Results: {} matches ===\n", matches.len());

            for block in matches {
                let timestamp = block["timestamp"].as_u64().unwrap_or(0);
                let hash = block["hash"].as_str().unwrap_or("unknown");
                let category = block["category"].as_str().unwrap_or("unknown");
                let action = block["action"].as_str().unwrap_or("unknown");

                let datetime = chrono::DateTime::from_timestamp(timestamp as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "invalid".to_string());

                println!("Block: {}", &hash[..16]);
                println!("  Time:     {}", datetime);
                println!("  Category: {}", category);
                println!("  Action:   {}", action);
                println!();
            }

            Ok(())
        }
    }
}

async fn handle_container_command(
    cmd: ContainerCommands,
    state_manager: &state::StateManager,
) -> Result<()> {
    match cmd {
        ContainerCommands::List { running, stopped } => {
            info!("Listing containers");
            let state = state_manager.query_plugin_state("lxc").await?;
            let lxc_state: crate::state::plugins::lxc::LxcState = serde_json::from_value(state)?;

            let containers: Vec<_> = if running {
                lxc_state
                    .containers
                    .into_iter()
                    .filter(|c| c.running == Some(true))
                    .collect()
            } else if stopped {
                lxc_state
                    .containers
                    .into_iter()
                    .filter(|c| c.running == Some(false))
                    .collect()
            } else {
                lxc_state.containers
            };

            if containers.is_empty() {
                println!("No containers found");
            } else {
                println!("=== Containers ({}) ===\n", containers.len());
                for container in containers {
                    let status = match container.running {
                        Some(true) => "RUNNING",
                        Some(false) => "STOPPED",
                        None => "UNKNOWN",
                    };
                    println!("Container {}", container.id);
                    println!("  Status:  {}", status);
                    println!("  Veth:    {}", container.veth);
                    println!("  Bridge:  {}", container.bridge);
                    if let Some(props) = &container.properties {
                        if let Some(net_type) = props.get("network_type") {
                            println!("  Network: {}", net_type.as_str().unwrap_or("unknown"));
                        }
                    }
                    println!();
                }
            }
            Ok(())
        }
        ContainerCommands::Show { container_id } => {
            info!("Showing container: {}", container_id);
            let state = state_manager.query_plugin_state("lxc").await?;
            let lxc_state: crate::state::plugins::lxc::LxcState = serde_json::from_value(state)?;

            let container = lxc_state.containers.iter().find(|c| c.id == container_id);

            match container {
                Some(c) => {
                    println!("=== Container {} ===\n", container_id);
                    println!("{}", serde_json::to_string_pretty(&c)?);
                }
                None => {
                    println!("Container {} not found", container_id);
                }
            }
            Ok(())
        }
        ContainerCommands::Create {
            container_id,
            network_type,
        } => {
            info!(
                "Creating container {} with network type: {}",
                container_id, network_type
            );
            println!("? Not yet implemented (use: op-dbus apply with state.json)");

            // Create container config
            let mut properties = std::collections::HashMap::new();
            properties.insert(
                "network_type".to_string(),
                serde_json::Value::String(network_type.clone()),
            );

            let container = crate::state::plugins::lxc::ContainerInfo {
                id: container_id.clone(),
                veth: format!("vi{}", container_id),
                bridge: "vmbr0".to_string(), // default bridge, may be changed by plugin
                running: None,
                properties: Some(properties),
            };

            // Use LXC plugin to create the container
            let lxc_plugin = crate::state::plugins::LxcPlugin::new();
            let result = lxc_plugin.apply_container_state(&container).await?;

            if result.success {
                println!("? Container {} created successfully", container_id);
                for change in &result.changes_applied {
                    println!("  - {}", change);
                }
            } else {
                println!("? Container {} creation failed", container_id);
                for error in &result.errors {
                    println!("  - ERROR: {}", error);
                }
            }

            Ok(())
        }
        ContainerCommands::Start { container_id } => {
            info!("Starting container {}", container_id);
            let output = tokio::process::Command::new("pct")
                .args(["start", &container_id])
                .output()
                .await?;

            if output.status.success() {
                println!("? Container {} started", container_id);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("? Failed: {}", stderr);
            }
            Ok(())
        }
        ContainerCommands::Stop { container_id } => {
            info!("Stopping container {}", container_id);
            let output = tokio::process::Command::new("pct")
                .args(["stop", &container_id])
                .output()
                .await?;

            if output.status.success() {
                println!("? Container {} stopped", container_id);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("? Failed: {}", stderr);
            }
            Ok(())
        }
        ContainerCommands::Destroy { container_id } => {
            tracing::warn!("Destroying container {}", container_id);
            let output = tokio::process::Command::new("pct")
                .args(["destroy", &container_id])
                .output()
                .await?;

            if output.status.success() {
                println!("? Container {} destroyed", container_id);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("? Failed: {}", stderr);
            }
            Ok(())
        }
    }
}
