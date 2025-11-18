//! CLI tool to introspect all D-Bus services and output JSON
//! This populates the SQLite cache and outputs comprehensive introspection data

use anyhow::Result;
use op_dbus::mcp::system_introspection::SystemIntrospector;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize simple logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    eprintln!("🔍 Starting D-Bus system introspection...");

    let introspector = SystemIntrospector::new().await?;

    eprintln!("📡 Discovering and introspecting all D-Bus services...");
    let system_introspection = introspector.introspect_all_services().await?;

    eprintln!(
        "✅ Introspection complete: {} services, {} interfaces, {} methods",
        system_introspection.total_services,
        system_introspection.total_interfaces,
        system_introspection.total_methods
    );

    // Output JSON to stdout
    let json = serde_json::to_string_pretty(&system_introspection)?;
    println!("{}", json);

    Ok(())
}
