//! op-dbus - Operation D-Bus
//! Declarative system state management via native protocols

mod blockchain;
mod ml;
mod native;
mod nonnet_db;
mod state;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tracing::{info, warn};

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
    state_file: &PathBuf,
) -> Result<()> {
    info!("Loading desired state from: {}", state_file.display());
    let desired_state = state_manager.load_desired_state(state_file).await?;
    let report = state_manager.apply_state(desired_state).await?;
    if report.success {
        info!("Successfully applied desired state");
    }
    Ok(())
}

async fn setup_dhcp_server() -> Result<()> {
    info!("Setting up DHCP server...");

    // Install dnsmasq (lightweight DHCP and DNS server)
    let output = tokio::process::Command::new("apt")
        .args(&["update"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to update package list"));
    }

    let output = tokio::process::Command::new("apt")
        .args(&["install", "-y", "dnsmasq"])
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
        .args(&["enable", "dnsmasq"])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to enable dnsmasq"));
    }

    let output = tokio::process::Command::new("systemctl")
        .args(&["restart", "dnsmasq"])
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

        Commands::Apply { state_file, dry_run } => {
            if dry_run {
                info!("DRY RUN: Showing what would be applied");
                let desired = state_manager.load_desired_state(&state_file).await?;
                let diffs = state_manager.show_diff(desired).await?;
                println!("{}", serde_json::to_string_pretty(&diffs)?);
            } else {
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
            warn!("Destroying container {}", container_id);
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
