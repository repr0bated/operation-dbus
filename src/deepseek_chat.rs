#[path = "mcp/ollama.rs"]
mod ollama;

use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{info, error};

// Use local Ollama client
use ollama::OllamaClient;

// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ChatMessage {
    User { content: String, timestamp: u64 },
    Assistant { content: String, timestamp: u64 },
    Error { content: String, timestamp: u64 },
}

// Chat server state
#[derive(Clone)]
struct ChatState {
    ollama_client: Option<Arc<OllamaClient>>,
    conversations: Arc<RwLock<HashMap<String, Vec<ChatMessage>>>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting AI Chat Server...");

    // Create Ollama client for AI
    let ollama_client = if let Ok(api_key) = std::env::var("OLLAMA_API_KEY") {
        info!("Using Ollama Cloud API with AI");
        Some(Arc::new(OllamaClient::cloud(api_key)))
    } else {
        info!("OLLAMA_API_KEY not set. Set your Ollama API key to enable AI chat.");
        info!("Get your API key from: https://ollama.com");
        None
    };

    // Create chat state
    let chat_state = ChatState {
        ollama_client,
        conversations: Arc::new(RwLock::new(HashMap::new())),
    };

    // Setup static file serving for the web UI
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("mcp")
        .join("web");

    // Create the main router
    let app = Router::new()
        // WebSocket endpoint for chat
        .route("/ws", get(websocket_handler))
        // Serve static files
        .route(
            "/",
            get(|| async { axum::response::Redirect::permanent("/chat.html") }),
        )
        .nest_service("/chat.html", ServeFile::new(web_dir.join("chat.html")))
        .nest_service("/chat.js", ServeFile::new(web_dir.join("chat.js")))
        .nest_service(
            "/chat-styles.css",
            ServeFile::new(web_dir.join("chat-styles.css")),
        )
        // Fallback to web directory for other static assets
        .nest_service("/", ServeDir::new(&web_dir))
        // Add tracing
        .layer(TraceLayer::new_for_http())
        .with_state(chat_state);

    // Start the server - bind to Netmaker IP for network access
    let netmaker_ip = "100.104.70.1";
    let addr = SocketAddr::from((netmaker_ip.parse::<std::net::IpAddr>()?, 8080));
    info!("AI Chat Server listening on http://{}", addr);
    info!("Access from Netmaker network: http://{}:8080", netmaker_ip);
    info!("Access locally: http://localhost:8080");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// WebSocket handler for chat
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ChatState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ChatState) {
    let (mut sender, mut receiver) = socket.split();

    // Generate a simple conversation ID
    let conversation_id = format!("conv_{}", std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis());

    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            // Parse incoming message
            if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(&text) {
                match chat_msg {
                    ChatMessage::User { content, .. } => {
                        // Store user message
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        let user_msg = ChatMessage::User {
                            content: content.clone(),
                            timestamp,
                        };

                        // Add to conversation
                        {
                            let mut conversations = state.conversations.write().await;
                            conversations.entry(conversation_id.clone())
                                .or_insert_with(Vec::new)
                                .push(user_msg.clone());
                        }

                        // Send back the user message for UI update
                        if let Ok(response) = serde_json::to_string(&user_msg) {
                            let _ = sender.send(Message::Text(response)).await;
                        }

                        // Generate AI response if Ollama client is available
                        if let Some(ollama_client) = &state.ollama_client {
                            let model = ollama_client.default_model();
                            match ollama_client.simple_chat(&model, &content).await {
                                Ok(ai_response) => {
                                    let ai_msg = ChatMessage::Assistant {
                                        content: ai_response,
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    };

                                    // Add to conversation
                                    {
                                        let mut conversations = state.conversations.write().await;
                                        conversations.entry(conversation_id.clone())
                                            .or_insert_with(Vec::new)
                                            .push(ai_msg.clone());
                                    }

                                    // Send AI response
                                    if let Ok(response) = serde_json::to_string(&ai_msg) {
                                        let _ = sender.send(Message::Text(response)).await;
                                    }
                                }
                                Err(e) => {
                                    error!("AI chat error: {}", e);
                                    let error_msg = ChatMessage::Error {
                                        content: format!("AI chat failed: {}", e),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                    };

                                    if let Ok(response) = serde_json::to_string(&error_msg) {
                                        let _ = sender.send(Message::Text(response)).await;
                                    }
                                }
                            }
                        } else {
                            // No Ollama client - send error message
                            let error_msg = ChatMessage::Error {
                                content: "AI AI is not available. Please set OLLAMA_API_KEY environment variable.".to_string(),
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            };

                            if let Ok(response) = serde_json::to_string(&error_msg) {
                                let _ = sender.send(Message::Text(response)).await;
                            }
                        }
                    }
                    _ => {} // Ignore other message types
                }
            }
        }
    }
}