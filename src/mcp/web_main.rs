use op_dbus::mcp::web_bridge;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Wayfire MCP Web Interface...");
    web_bridge::run_web_server().await
}
