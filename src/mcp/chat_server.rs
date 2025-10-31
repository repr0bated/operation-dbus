use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::SystemTime};
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;

use super::agent_registry::AgentRegistry;
use super::tool_registry::ToolRegistry;

// Chat message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatMessage {
    User {
        content: String,
        timestamp: u64,
    },
    Assistant {
        content: String,
        timestamp: u64,
        tools_used: Vec<String>,
    },
    System {
        content: String,
        timestamp: u64,
    },
    Error {
        content: String,
        timestamp: u64,
    },
}

// Command parsing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub intent: CommandIntent,
    pub parameters: HashMap<String, serde_json::Value>,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandIntent {
    ExecuteTool { tool_name: String },
    ManageAgent { action: String, agent_name: String },
    QueryStatus,
    ListTools,
    ListAgents,
    GetHelp { topic: Option<String> },
    Unknown,
}

// Conversation context
#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub current_agent: Option<String>,
    pub variables: HashMap<String, String>,
}

// Natural language processor
pub struct NaturalLanguageProcessor;

impl NaturalLanguageProcessor {
    pub fn parse_command(input: &str) -> ParsedCommand {
        let lower = input.to_lowercase();
        let mut parameters = HashMap::new();

        // Pattern matching for different command types
        let intent = if lower.starts_with("run ") || lower.starts_with("execute ") {
            // Extract tool name and parameters
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() > 1 {
                let tool_name = parts[1].to_string();

                // Parse remaining as parameters
                if parts.len() > 2 {
                    let params_str = parts[2..].join(" ");
                    // Try to parse as JSON first
                    if let Ok(json_params) = serde_json::from_str::<serde_json::Value>(&params_str)
                    {
                        if let serde_json::Value::Object(map) = json_params {
                            for (k, v) in map {
                                parameters.insert(k, v);
                            }
                        }
                    } else {
                        // Simple key=value parsing
                        for param in params_str.split(',') {
                            if let Some((key, value)) = param.split_once('=') {
                                parameters.insert(
                                    key.trim().to_string(),
                                    serde_json::Value::String(value.trim().to_string()),
                                );
                            }
                        }
                    }
                }

                CommandIntent::ExecuteTool { tool_name }
            } else {
                CommandIntent::Unknown
            }
        } else if lower.starts_with("start agent ") || lower.starts_with("spawn ") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() > 1 {
                let agent_name = parts[parts.len() - 1].to_string();
                CommandIntent::ManageAgent {
                    action: "start".to_string(),
                    agent_name,
                }
            } else {
                CommandIntent::Unknown
            }
        } else if lower.starts_with("stop agent ") || lower.starts_with("kill ") {
            let parts: Vec<&str> = input.split_whitespace().collect();
            if parts.len() > 1 {
                let agent_name = parts[parts.len() - 1].to_string();
                CommandIntent::ManageAgent {
                    action: "stop".to_string(),
                    agent_name,
                }
            } else {
                CommandIntent::Unknown
            }
        } else if lower.contains("status") || lower == "?" {
            CommandIntent::QueryStatus
        } else if lower.contains("list tools") || lower == "tools" {
            CommandIntent::ListTools
        } else if lower.contains("list agents") || lower == "agents" {
            CommandIntent::ListAgents
        } else if lower.starts_with("help") {
            let topic = if lower.len() > 5 {
                Some(lower[5..].trim().to_string())
            } else {
                None
            };
            CommandIntent::GetHelp { topic }
        } else {
            // Try to infer intent from keywords
            if lower.contains("systemd") || lower.contains("service") {
                parameters.insert(
                    "service".to_string(),
                    serde_json::Value::String(extract_service_name(&lower).unwrap_or_default()),
                );
                CommandIntent::ExecuteTool {
                    tool_name: "systemd".to_string(),
                }
            } else if lower.contains("file") || lower.contains("read") || lower.contains("write") {
                CommandIntent::ExecuteTool {
                    tool_name: "file".to_string(),
                }
            } else if lower.contains("network")
                || lower.contains("interface")
                || lower.contains("ip")
            {
                CommandIntent::ExecuteTool {
                    tool_name: "network".to_string(),
                }
            } else {
                CommandIntent::Unknown
            }
        };

        ParsedCommand {
            intent,
            parameters,
            raw_text: input.to_string(),
        }
    }

    pub fn generate_suggestions(partial_input: &str) -> Vec<String> {
        let suggestions = vec![
            "run systemd status service=",
            "run file read path=",
            "run network list",
            "run process list",
            "start agent executor",
            "list tools",
            "list agents",
            "status",
            "help",
        ];

        suggestions
            .into_iter()
            .filter(|s| s.starts_with(&partial_input.to_lowercase()))
            .map(String::from)
            .collect()
    }
}

fn extract_service_name(text: &str) -> Option<String> {
    // Simple heuristic to extract service names
    for word in text.split_whitespace() {
        if word.ends_with(".service") || word.ends_with("d") {
            return Some(word.to_string());
        }
    }
    None
}

// Chat server state
#[derive(Clone)]
pub struct ChatServerState {
    pub tool_registry: Arc<ToolRegistry>,
    pub agent_registry: Arc<AgentRegistry>,
    pub conversations: Arc<RwLock<HashMap<String, ConversationContext>>>,
    pub broadcast_tx: broadcast::Sender<ChatMessage>,
}

impl ChatServerState {
    pub fn new(tool_registry: Arc<ToolRegistry>, agent_registry: Arc<AgentRegistry>) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);

        Self {
            tool_registry,
            agent_registry,
            conversations: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    pub async fn process_message(&self, conversation_id: &str, message: &str) -> ChatMessage {
        // Parse the command
        let parsed = NaturalLanguageProcessor::parse_command(message);

        // Store user message
        let user_msg = ChatMessage::User {
            content: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.add_message(conversation_id, user_msg.clone()).await;

        // Process based on intent
        let response = match parsed.intent {
            CommandIntent::ExecuteTool { tool_name } => {
                self.execute_tool(&tool_name, parsed.parameters).await
            }
            CommandIntent::ManageAgent { action, agent_name } => {
                self.manage_agent(&action, &agent_name).await
            }
            CommandIntent::QueryStatus => self.get_status().await,
            CommandIntent::ListTools => self.list_tools().await,
            CommandIntent::ListAgents => self.list_agents().await,
            CommandIntent::GetHelp { topic } => self.get_help(topic.as_deref()).await,
            CommandIntent::Unknown => ChatMessage::Assistant {
                content: format!(
                    "I didn't understand '{}'. Try:\n\
                        • 'run <tool> <params>' to execute a tool\n\
                        • 'list tools' to see available tools\n\
                        • 'status' to check system status\n\
                        • 'help' for more information",
                    message
                ),
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                tools_used: vec![],
            },
        };

        // Store assistant response
        self.add_message(conversation_id, response.clone()).await;

        response
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> ChatMessage {
        match self
            .tool_registry
            .execute_tool(
                tool_name,
                serde_json::Value::Object(params.into_iter().collect()),
            )
            .await
        {
            Ok(result) => {
                let content_str = result
                    .content
                    .iter()
                    .filter_map(|c| c.text.as_ref())
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                ChatMessage::Assistant {
                    content: format!(
                        "✅ Tool '{}' executed successfully:\n{}",
                        tool_name, content_str
                    ),
                    timestamp: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    tools_used: vec![tool_name.to_string()],
                }
            }
            Err(e) => ChatMessage::Error {
                content: format!("❌ Failed to execute tool '{}': {}", tool_name, e),
                timestamp: SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
        }
    }

    async fn manage_agent(&self, action: &str, agent_name: &str) -> ChatMessage {
        let result = match action {
            "start" | "spawn" => match self.agent_registry.spawn_agent(agent_name, None).await {
                Ok(instance_id) => {
                    format!("✅ Agent '{}' started (ID: {})", agent_name, instance_id)
                }
                Err(e) => format!("❌ Failed to start agent '{}': {}", agent_name, e),
            },
            "stop" | "kill" => match self.agent_registry.kill_agent(agent_name).await {
                Ok(_) => format!("✅ Agent '{}' stopped", agent_name),
                Err(e) => format!("❌ Failed to stop agent '{}': {}", agent_name, e),
            },
            _ => format!("❓ Unknown agent action: {}", action),
        };

        ChatMessage::Assistant {
            content: result,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tools_used: vec![],
        }
    }

    async fn get_status(&self) -> ChatMessage {
        let tools = self.tool_registry.list_tools().await;
        let agent_types = self.agent_registry.list_agent_types().await;
        let instances = self.agent_registry.list_instances().await;

        let content = format!(
            "📊 System Status\n\
            ═══════════════\n\
            🔧 Tools: {} available\n\
            🤖 Agents: {} registered, {} running\n\
            💬 Conversations: {} active",
            tools.len(),
            agent_types.len(),
            instances.len(),
            self.conversations.read().await.len()
        );

        ChatMessage::Assistant {
            content,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tools_used: vec![],
        }
    }

    async fn list_tools(&self) -> ChatMessage {
        let tools = self.tool_registry.list_tools().await;

        let mut content = String::from("🔧 Available Tools\n═════════════════\n");
        for tool in tools {
            content.push_str(&format!("• {} - {}\n", tool.name, tool.description));
        }

        ChatMessage::Assistant {
            content,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tools_used: vec![],
        }
    }

    async fn list_agents(&self) -> ChatMessage {
        let agent_types = self.agent_registry.list_agent_types().await;
        let instances = self.agent_registry.list_instances().await;

        let mut content = String::from("🤖 Agents\n════════\n");

        content.push_str("Registered:\n");
        for agent_type in &agent_types {
            content.push_str(&format!("• {}\n", agent_type));
        }

        if !instances.is_empty() {
            content.push_str("\nRunning:\n");
            for instance in &instances {
                content.push_str(&format!(
                    "• {} (ID: {})\n",
                    instance.agent_type, instance.id
                ));
            }
        }

        ChatMessage::Assistant {
            content,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tools_used: vec![],
        }
    }

    async fn get_help(&self, topic: Option<&str>) -> ChatMessage {
        let content = match topic {
            Some("tools") => {
                "🔧 Tool Commands:\n\
                • run <tool> <params> - Execute a tool\n\
                • list tools - Show available tools\n\
                \n\
                Examples:\n\
                • run systemd status service=nginx\n\
                • run file read path=/etc/hosts\n\
                • run network list"
            }
            Some("agents") => {
                "🤖 Agent Commands:\n\
                • start agent <name> - Start an agent\n\
                • stop agent <name> - Stop an agent\n\
                • list agents - Show all agents\n\
                \n\
                Examples:\n\
                • start agent executor\n\
                • stop agent file\n\
                • agents"
            }
            _ => {
                "📚 MCP Chat Help\n\
                ══════════════\n\
                \n\
                Commands:\n\
                • run <tool> <params> - Execute a tool\n\
                • start/stop agent <name> - Manage agents\n\
                • list tools/agents - Show available resources\n\
                • status - System status\n\
                • help [topic] - Get help\n\
                \n\
                Try 'help tools' or 'help agents' for more details."
            }
        };

        ChatMessage::Assistant {
            content: content.to_string(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tools_used: vec![],
        }
    }

    async fn add_message(&self, conversation_id: &str, message: ChatMessage) {
        let mut conversations = self.conversations.write().await;
        let context = conversations
            .entry(conversation_id.to_string())
            .or_insert_with(|| ConversationContext {
                id: conversation_id.to_string(),
                messages: Vec::new(),
                current_agent: None,
                variables: HashMap::new(),
            });

        context.messages.push(message.clone());

        // Keep only last 100 messages per conversation
        if context.messages.len() > 100 {
            context.messages.remove(0);
        }

        // Broadcast to all connected clients
        let _ = self.broadcast_tx.send(message);
    }
}

// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<ChatServerState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, state: ChatServerState) {
    use axum::extract::ws::Message;
    use futures::{sink::SinkExt, stream::StreamExt};

    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.broadcast_tx.subscribe();

    // Generate conversation ID
    let conversation_id = uuid::Uuid::new_v4().to_string();

    // Send welcome message
    let welcome = ChatMessage::System {
        content: format!(
            "Welcome to MCP Chat! (Session: {})\nType 'help' to get started.",
            &conversation_id[..8]
        ),
        timestamp: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let _ = sender
        .send(Message::Text(serde_json::to_string(&welcome).unwrap()))
        .await;

    // Spawn task to handle broadcasts
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // Process the message
            let response = state.process_message(&conversation_id, &text).await;

            // Response is sent via broadcast, so no need to send directly
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

// REST API endpoints
pub async fn get_suggestions(
    State(state): State<ChatServerState>,
    Json(payload): Json<HashMap<String, String>>,
) -> Json<Vec<String>> {
    let partial = payload.get("partial").map(|s| s.as_str()).unwrap_or("");
    let suggestions = NaturalLanguageProcessor::generate_suggestions(partial);
    Json(suggestions)
}

pub async fn get_conversation_history(
    State(state): State<ChatServerState>,
    Json(payload): Json<HashMap<String, String>>,
) -> Json<Vec<ChatMessage>> {
    let conversation_id = payload.get("id").map(|s| s.as_str()).unwrap_or("");
    let conversations = state.conversations.read().await;

    if let Some(context) = conversations.get(conversation_id) {
        Json(context.messages.clone())
    } else {
        Json(vec![])
    }
}

pub fn create_chat_router(state: ChatServerState) -> Router {
    Router::new()
        .route("/ws", get(websocket_handler))
        .route("/api/suggestions", post(get_suggestions))
        .route("/api/history", post(get_conversation_history))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
