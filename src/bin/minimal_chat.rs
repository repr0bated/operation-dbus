// Minimal AI chat server - bypasses broken mcp modules
// Run with: cargo run --bin minimal_chat --features web

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Minimal AI Chat Server...");

    // Check for API key
    if std::env::var("OLLAMA_API_KEY").is_err() {
        println!("⚠️  OLLAMA_API_KEY not set!");
        println!("   Set it with: export OLLAMA_API_KEY=your-key");
        println!("   Or source ~/.bashrc");
    } else {
        println!("✅ OLLAMA_API_KEY is configured");
    }

    // Web directory
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("mcp")
        .join("web");

    println!("📁 Web directory: {}", web_dir.display());

    // Simple router - just serve static files
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                axum::response::Redirect::permanent("/index.html")
            }),
        )
        .nest_service("/", ServeDir::new(&web_dir));

    // Bind to Netmaker IP
    let netmaker_ip = "100.104.70.1";
    let addr: SocketAddr = format!("{}:8080", netmaker_ip).parse()?;

    println!("\n✅ Server starting on:");
    println!("   http://{}:8080", netmaker_ip);
    println!("   http://localhost:8080");
    println!("\n📝 Note: WebSocket chat features require fixing the mcp compilation errors");
    println!("   See BROKEN_CODE_ANALYSIS.md for details");
    println!("\nPress Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
