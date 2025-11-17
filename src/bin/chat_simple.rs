//! Simple AI Chat Server - Only uses working code
//! No broken zbus dependencies
//! Run with: OLLAMA_API_KEY=your-key OLLAMA_DEFAULT_MODEL=model-name cargo run --bin chat_simple --release
//! Example models: deepseek-v3.1:671b-cloud, llama2, mistral, etc.

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir};

// These modules compile fine - no zbus
mod ollama_mod {
    include!("../mcp/ollama.rs");
}

mod ai_context_mod {
    include!("../mcp/ai_context_provider.rs");
}

use ollama_mod::{ChatMessage, OllamaClient};
use ai_context_mod::{AiContextProvider, SystemContext};

#[derive(Clone)]
struct AppState {
    ollama: Arc<OllamaClient>,
    context_provider: Arc<AiContextProvider>,
    system_context: Arc<RwLock<Option<SystemContext>>>,
    history: Arc<RwLock<Vec<ChatMessage>>>,
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Serialize)]
struct ChatResponse {
    response: String,
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    model: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("🚀 AI Chat Server (Simple Mode)\n");

    let api_key = std::env::var("OLLAMA_API_KEY")
        .expect("OLLAMA_API_KEY not set");

    let model = std::env::var("OLLAMA_DEFAULT_MODEL")
        .unwrap_or_else(|_| "llama2".to_string());

    println!("✅ API key loaded");
    println!("✅ Using model: {}", model);

    let ollama = Arc::new(OllamaClient::cloud(api_key).with_default_model(model));
    let context_provider = Arc::new(AiContextProvider::new());

    print!("📊 Gathering system context... ");
    let sys_ctx = context_provider.gather_context().await.ok();
    if sys_ctx.is_some() {
        println!("✅");
    } else {
        println!("⚠️  (will work without it)");
    }

    let state = AppState {
        ollama,
        context_provider,
        system_context: Arc::new(RwLock::new(sys_ctx)),
        history: Arc::new(RwLock::new(Vec::new())),
    };

    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/mcp/web");

    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/index.html") }))
        .route("/api/status", get(handle_status))
        .route("/api/chat", post(handle_chat))
        .nest_service("/", ServeDir::new(&web_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8080".parse()?;

    println!("\n✅ Server running!");
    println!("   http://localhost:8080");
    println!("   http://80.209.240.244:8080");
    println!("\nPress Ctrl+C to stop\n");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(StatusResponse {
        status: "ok".to_string(),
        model: state.ollama.default_model(),
    })
}

async fn handle_chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    
    println!("💬 {}", req.message);

    let sys_ctx_text = {
        let ctx_lock = state.system_context.read().await;
        if let Some(ctx) = ctx_lock.as_ref() {
            state.context_provider.generate_summary(ctx)
        } else {
            String::new()
        }
    };

    let history = state.history.read().await.clone();

    let model = state.ollama.default_model();
    let response = state
        .ollama
        .chat_with_context(
            &model,
            &sys_ctx_text,
            &history,
            &req.message,
            Some(0.7),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    println!("✅ Response: {} chars", response.len());

    {
        let mut hist = state.history.write().await;
        hist.push(ChatMessage {
            role: "user".to_string(),
            content: req.message,
        });
        hist.push(ChatMessage {
            role: "assistant".to_string(),
            content: response.clone(),
        });
        if hist.len() > 20 {
            let hist_len = hist.len();
            hist.drain(0..hist_len - 20);
        }
    }

    Ok(Json(ChatResponse { response }))
}
