use std::sync::Arc;

// Include the ollama module directly
mod ollama;
use ollama::OllamaClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Ollama DeepSeek integration...");

    // Create Ollama client with API key
    let api_key = "1e4ffc3e35d14302ae8c38a3b88afbdf.6rcSE8GW_DsKPquVev9o7obK";
    let client = OllamaClient::deepseek_cloud(api_key.to_string());

    // Test a simple chat
    println!("Sending test message to DeepSeek...");
    match client.deepseek_chat("Hello, can you tell me what 2+2 equals?").await {
        Ok(response) => {
            println!("✅ DeepSeek Response:");
            println!("{}", response);
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}