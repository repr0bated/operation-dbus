//! MCP Proxy – thin shim with optional direct-to-subscription mode.

use op_cache::proto::{mcp_service_client::McpServiceClient, McpRequest};
use std::io::{BufRead, Write};
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::info;

mod cloudaicompanion;
mod direct_llm;
mod gcloud_auth;
mod session;

use direct_llm::DirectLLM;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    // If DIRECT_MODE is set we handle LLM requests ourselves.
    let direct_mode = std::env::var("DIRECT_MODE").is_ok();
    let direct_llm = if direct_mode {
        info!("Running in DIRECT_MODE – LLM calls go to cloudaicompanion.googleapis.com");
        Some(Arc::new(DirectLLM::new().await?))
    } else {
        None
    };

    let daemon_addr =
        std::env::var("OP_DBUS_ADDR").unwrap_or_else(|_| "http://[::1]:50051".to_string());

    let channel = Channel::from_shared(daemon_addr)?.connect().await?;
    let mut client = McpServiceClient::new(channel);

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: simd_json::OwnedValue = simd_json::from_str(&line)?;
        let method = req["method"].as_str().unwrap_or("");

        // Route LLM methods directly if in direct mode
        if let Some(ref llm) = direct_llm {
            if matches!(
                method,
                "completion/complete" | "sampling/createMessage" | "generate"
            ) {
                let resp = llm.handle(&req).await;
                writeln!(stdout, "{}", simd_json::to_string(&resp)?)?;
                stdout.flush()?;
                continue;
            }
        }

        // Otherwise forward to op-dbus daemon (original behaviour)
        let grpc_req = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: req["method"].as_str().unwrap_or("").to_string(),
            id: req["id"].as_str().unwrap_or("null").to_string(),
            params: serde_json::to_vec(&req["params"]).unwrap_or_default(),
        };
        let grpc_resp = client.handle_request(grpc_req).await?.into_inner();
        let json_resp = if let Some(err) = grpc_resp.error {
            simd_json::json!({
                "jsonrpc": "2.0",
                "id": grpc_resp.id,
                "error": { "code": err.code, "message": err.message }
            })
        } else {
            simd_json::json!({
                "jsonrpc": "2.0",
                "id": grpc_resp.id,
                "result": serde_json::from_slice::<simd_json::OwnedValue>(&grpc_resp.result).unwrap_or(simd_json::OwnedValue::Null)
            })
        };
        writeln!(stdout, "{}", simd_json::to_string(&json_resp)?)?;
        stdout.flush()?;
    }
    Ok(())
}
