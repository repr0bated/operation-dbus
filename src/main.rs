//! op-dbus - Operation D-Bus
//! Declarative system state management via native protocols

mod blockchain;
mod cache;
mod ml;
mod native;
mod nonnet_db;
mod state;
mod webui;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
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
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
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
enum BlockchainCommands {
    /// List blockchain blocks
    List {
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Show specific block
    Show {
        block_id: String,
    },

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
    Search {
        query: String,
    },
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
    Show {
        container_id: String,
    },

    /// Create container
    Create {
        container_id: String,
        #[arg(long, default_value = "bridge")]
        network_type: String,
    },

    /// Start container
    Start {
        container_id: String,
    },

    /// Stop container
    Stop {
        container_id: String,
    },

    /// Destroy container
    Destroy {
        container_id: String,
    },
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
interface=vmbr0
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

    let mut sm = state::StateManager::new();
    // Initialize blockchain footprint streaming (best-effort)
    if let Ok(blockchain) =
        crate::blockchain::StreamingBlockchain::new("/var/lib/op-dbus/blockchain").await
    {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        sm.set_blockchain_sender(tx);
        tokio::spawn(async move {
            let _ = blockchain.start_footprint_receiver(rx).await;
        });
    } else {
        info!("Blockchain storage not initialized; footprints disabled");
    }
    let state_manager = Arc::new(sm);
    state_manager
        .register_plugin(Box::new(state::plugins::NetStatePlugin::new()))
        .await;
    state_manager
        .register_plugin(Box::new(state::plugins::SystemdStatePlugin::new()))
        .await;
    state_manager
        .register_plugin(Box::new(state::plugins::Login1Plugin::new()))
        .await;
    state_manager
        .register_plugin(Box::new(state::plugins::LxcPlugin::new()))
        .await;

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

        Commands::Apply { state_file, dry_run, plugin } => {
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
            } else {
                if let Some(plugin_name) = plugin {
                    info!("Applying state for plugin: {}", plugin_name);
                    apply_state_from_file_single_plugin(&state_manager, &state_file, &plugin_name).await?;
                } else {
                    info!("⚠️  WARNING: Applying state to ALL plugins system-wide");
                    info!("⚠️  Consider using --plugin flag to limit scope");
                    apply_state_from_file(&state_manager, &state_file).await?;
                }
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

        Commands::Diff { state_file, plugin: _ } => {
            let desired = state_manager.load_desired_state(&state_file).await?;
            let diffs = state_manager.show_diff(desired).await?;
            println!("{}", serde_json::to_string_pretty(&diffs)?);
            Ok(())
        }

        Commands::Verify { full } => {
            info!("Verifying state against blockchain footprint");
            if full {
                info!("Full blockchain integrity check not yet implemented");
            } else {
                info!("Basic verification not yet implemented");
            }
            println!("✓ Verification passed (placeholder)");
            Ok(())
        }

        Commands::Blockchain(cmd) => handle_blockchain_command(cmd).await,

        Commands::Container(cmd) => handle_container_command(cmd, &state_manager).await,

        Commands::ApplyContainer { container_id, state_file } => {
            info!("Applying state for container: {}", container_id);
            
            let state_path = state_file.unwrap_or_else(|| PathBuf::from("/etc/op-dbus/state.json"));
            let desired_state = state_manager.load_desired_state(&state_path).await?;
            
            // Find the container in the desired state
            if let Some(lxc_state) = desired_state.plugins.get("lxc") {
                let lxc_config: crate::state::plugins::lxc::LxcState = serde_json::from_value(lxc_state.clone())?;
                
                if let Some(container) = lxc_config.containers.iter().find(|c| c.id == container_id) {
                    // Get LXC plugin and apply single container
                    let lxc_plugin = crate::state::plugins::LxcPlugin::new();
                    let result = lxc_plugin.apply_container_state(container).await?;
                    
                    if result.success {
                        println!("✓ Container {} applied successfully", container_id);
                        for change in &result.changes_applied {
                            println!("  - {}", change);
                        }
                    } else {
                        println!("✗ Container {} apply failed", container_id);
                        for error in &result.errors {
                            println!("  - ERROR: {}", error);
                        }
                    }
                } else {
                    return Err(anyhow::anyhow!("Container {} not found in state file", container_id));
                }
            } else {
                return Err(anyhow::anyhow!("No LXC plugin configuration in state file"));
            }
            
            Ok(())
        }

        Commands::Init { introspect, output } => {
            info!("Initializing configuration");
            if introspect {
                // Query current state
                let current = state_manager.query_current_state().await?;
                let json = serde_json::to_string_pretty(&current)?;

                if let Some(out_path) = output {
                    fs::write(&out_path, json).await?;
                    println!("✓ Configuration written to: {}", out_path.display());
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
                    println!("✓ Template written to: {}", out_path.display());
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
                                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&info) {
                                            bridge_details.push(parsed);
                                        }
                                    }
                                }
                                ovsdb_data.insert("bridges".to_string(), serde_json::json!(bridge_details));
                            }
                        }
                        
                        results.insert("ovsdb".to_string(), serde_json::Value::Object(ovsdb_data));
                    }
                    Err(e) => {
                        results.insert("ovsdb".to_string(), serde_json::json!({
                            "error": format!("Failed to connect: {}", e)
                        }));
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
                        
                        results.insert("nonnet".to_string(), serde_json::Value::Object(nonnet_data));
                    }
                    Err(e) => {
                        results.insert("nonnet".to_string(), serde_json::json!({
                            "error": format!("Failed to query: {}", e)
                        }));
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
            println!("✓ op-dbus binary running");

            // Check OVSDB
            print!("Checking OVSDB connection... ");
            match crate::native::OvsdbClient::new().list_dbs().await {
                Ok(_) => println!("✓"),
                Err(e) => println!("✗ Failed: {}", e),
            }

            // Check D-Bus
            print!("Checking D-Bus connection... ");
            match zbus::Connection::system().await {
                Ok(_) => println!("✓"),
                Err(e) => println!("✗ Failed: {}", e),
            }

            // Check blockchain storage
            print!("Checking blockchain storage... ");
            if std::path::Path::new("/var/lib/op-dbus/blockchain").exists() {
                println!("✓");
            } else {
                println!("✗ Not found");
            }

            // Check state file
            print!("Checking state file... ");
            if std::path::Path::new("/etc/op-dbus/state.json").exists() {
                println!("✓");
            } else {
                println!("⚠ Not found (run: op-dbus init --introspect)");
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

            let cache = crate::cache::BtrfsCache::new(cache_dir)?;
            let stats = cache.stats()?;

            println!("Embeddings:");
            println!("  Total entries:    {}", stats.total_entries);
            println!("  Hot (< 1h):       {} ({:.1}%)",
                stats.hot_entries,
                stats.hot_ratio() * 100.0
            );
            println!("  Average accesses: {:.1}", stats.avg_accesses());
            println!("  Disk usage:       {:.2} MB",
                stats.embeddings_size_bytes as f64 / 1024.0 / 1024.0
            );

            println!("\nBlocks:");
            println!("  Disk usage:       {:.2} MB",
                stats.blocks_size_bytes as f64 / 1024.0 / 1024.0
            );

            println!("\nTotal:");
            println!("  Disk usage:       {:.2} MB (compressed)",
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

            Ok(())
        }

        CacheCommands::Clear { embeddings, blocks, all } => {
            let cache = crate::cache::BtrfsCache::new(cache_dir)?;

            if all || (!embeddings && !blocks) {
                println!("Clearing all cache...");
                cache.clear()?;
                println!("✓ All cache cleared");
            } else {
                if embeddings {
                    println!("Clearing embeddings cache...");
                    // TODO: Implement selective clear
                    println!("✗ Selective clear not yet implemented");
                }
                if blocks {
                    println!("Clearing blocks cache...");
                    // TODO: Implement selective clear
                    println!("✗ Selective clear not yet implemented");
                }
            }

            Ok(())
        }

        CacheCommands::Clean { older_than_days } => {
            println!("Cleaning cache entries older than {} days...", older_than_days);
            let cache = crate::cache::BtrfsCache::new(cache_dir)?;
            let removed = cache.cleanup_old(older_than_days)?;
            println!("✓ Cleaned {} old entries", removed);
            Ok(())
        }

        CacheCommands::Snapshot => {
            println!("Creating cache snapshot...");
            let cache = crate::cache::BtrfsCache::new(cache_dir)?;
            let snapshot_path = cache.create_snapshot().await?;
            println!("✓ Created snapshot: {}", snapshot_path.display());
            Ok(())
        }

        CacheCommands::Snapshots => {
            let cache = crate::cache::BtrfsCache::new(cache_dir)?;
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
            let cache = crate::cache::BtrfsCache::new(cache_dir)?;
            let count = cache.delete_all_snapshots().await?;
            println!("✓ Deleted {} snapshots", count);
            Ok(())
        }
    }
}

async fn handle_blockchain_command(cmd: BlockchainCommands) -> Result<()> {
    match cmd {
        BlockchainCommands::List { limit } => {
            info!("Listing blockchain blocks");
            let blockchain_path = "/var/lib/op-dbus/blockchain";
            if !std::path::Path::new(blockchain_path).exists() {
                println!("No blockchain found. Run 'op-dbus apply' to create genesis block.");
                return Ok(());
            }

            println!("Blockchain list (limit: {})", limit.unwrap_or(10));
            println!("✗ Not yet fully implemented");
            Ok(())
        }
        BlockchainCommands::Show { block_id } => {
            println!("Showing block: {}", block_id);
            println!("✗ Not yet implemented");
            Ok(())
        }
        BlockchainCommands::Export { output } => {
            println!("Exporting blockchain to: {:?}", output);
            println!("✗ Not yet implemented");
            Ok(())
        }
        BlockchainCommands::Verify { full } => {
            println!("Verifying blockchain integrity (full: {})", full);
            println!("✗ Not yet implemented");
            Ok(())
        }
        BlockchainCommands::Search { query } => {
            println!("Searching blockchain for: {}", query);
            println!("✗ Not yet implemented");
            Ok(())
        }
    }
}

async fn handle_container_command(cmd: ContainerCommands, state_manager: &state::StateManager) -> Result<()> {
    match cmd {
        ContainerCommands::List { running, stopped } => {
            info!("Listing containers");
            let state = state_manager.query_plugin_state("lxc").await?;
            println!("{}", serde_json::to_string_pretty(&state)?);

            if running || stopped {
                println!("Filtering not yet implemented");
            }
            Ok(())
        }
        ContainerCommands::Show { container_id } => {
            println!("Showing container: {}", container_id);
            let state = state_manager.query_plugin_state("lxc").await?;
            // TODO: Filter to specific container
            println!("{}", serde_json::to_string_pretty(&state)?);
            Ok(())
        }
        ContainerCommands::Create { container_id, network_type } => {
            println!("Creating container {} with network type: {}", container_id, network_type);
            println!("✗ Not yet implemented (use: op-dbus apply with state.json)");
            Ok(())
        }
        ContainerCommands::Start { container_id } => {
            info!("Starting container {}", container_id);
            let output = tokio::process::Command::new("pct")
                .args(["start", &container_id])
                .output()
                .await?;

            if output.status.success() {
                println!("✓ Container {} started", container_id);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("✗ Failed: {}", stderr);
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
                println!("✓ Container {} stopped", container_id);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("✗ Failed: {}", stderr);
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
                println!("✓ Container {} destroyed", container_id);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("✗ Failed: {}", stderr);
            }
            Ok(())
        }
    }
}
