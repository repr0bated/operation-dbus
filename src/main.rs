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
    Run,
    Apply { state_file: PathBuf },
    Query { plugin: Option<String> },
    Diff { state_file: PathBuf },
    // No CLI dump commands; interactions via native protocols
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

    match args.command.unwrap_or(Commands::Run) {
        Commands::Run => {
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
            tokio::signal::ctrl_c().await?;
            Ok(())
        }
        Commands::Apply { state_file } => apply_state_from_file(&state_manager, &state_file).await,
        Commands::Query { plugin } => {
            let state = if let Some(p) = plugin {
                state_manager.query_plugin_state(&p).await?
            } else {
                serde_json::to_value(&state_manager.query_current_state().await?)?
            };
            println!("{}", serde_json::to_string_pretty(&state)?);
            Ok(())
        }
        Commands::Diff { state_file } => {
            let desired = state_manager.load_desired_state(&state_file).await?;
            let diffs = state_manager.show_diff(desired).await?;
            println!("{}", serde_json::to_string_pretty(&diffs)?);
            Ok(())
        }
    }
}
