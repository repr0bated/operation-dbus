use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ChatMessage,
    done: bool,
    model: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing DeepSeek Ollama Cloud API...");

    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    let api_key = "1e4ffc3e35d14302ae8c38a3b88afbdf.6rcSE8GW_DsKPquVev9o7obK";

    let request = ChatRequest {
        model: "deepseek-v3.1:671b-cloud".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello! Can you tell me what 2+2 equals?".to_string(),
        }],
        stream: false,
    };

    let response = client
        .post("https://ollama.com/api/chat")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let chat_response: ChatResponse = response.json().await?;
        println!("✅ Success! DeepSeek Response:");
        println!("Model: {}", chat_response.model.unwrap_or("unknown".to_string()));
        println!("Content: {}", chat_response.message.content);
    } else {
        println!("❌ API Error: {}", response.status());
        let error_text = response.text().await?;
        println!("Error details: {}", error_text);
    }

    Ok(())
}