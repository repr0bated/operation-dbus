//! op-dbus - Operation D-Bus
//! Declarative system state management via native protocols

mod blockchain;
mod native;
mod state;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "op-dbus", version, about = "Declarative system state via native protocols")]
struct Cli {
    #[arg(short, long)]
    state_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Run,
    Apply { state_file: PathBuf },
    Query { plugin: Option<String> },
    Diff { state_file: PathBuf },
}

fn init_logging() -> Result<()> {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::from_default_env().add_directive("op_dbus=info".parse().unwrap());
    let subscriber = fmt::Subscriber::builder().with_env_filter(filter).with_target(false).finish();
    tracing::subscriber::set_global_default(subscriber).context("Failed to set tracing subscriber")?;
    Ok(())
}

async fn apply_state_from_file(state_manager: &state::StateManager, state_file: &PathBuf) -> Result<()> {
    info!("Loading desired state from: {}", state_file.display());
    let desired_state = state_manager.load_desired_state(state_file).await?;
    let report = state_manager.apply_state(desired_state).await?;
    if report.success {
        info!("Successfully applied desired state");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;
    let args = Cli::parse();
    
    let state_manager = Arc::new(state::StateManager::new());
    state_manager.register_plugin(Box::new(state::plugins::NetStatePlugin::new())).await;
    state_manager.register_plugin(Box::new(state::plugins::SystemdStatePlugin::new())).await;
    
    match args.command.unwrap_or(Commands::Run) {
        Commands::Run => {
            let state_file = args.state_file.unwrap_or_else(|| PathBuf::from("/etc/op-dbus/state.json"));
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
