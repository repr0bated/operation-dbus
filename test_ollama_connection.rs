// Quick test to verify Ollama/DeepSeek connection works
// Compile: rustc --edition 2021 test_ollama_connection.rs -L target/debug/deps --extern op_dbus=target/debug/libop_dbus.rlib --extern tokio --extern anyhow
// Or just: cargo test --lib --features web test_ollama

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_ollama_client() {
        use op_dbus::mcp::ollama::OllamaClient;

        let api_key = std::env::var("OLLAMA_API_KEY")
            .expect("OLLAMA_API_KEY must be set");

        let client = OllamaClient::deepseek_cloud(api_key);

        println!("🧪 Testing DeepSeek connection...");

        match client.deepseek_chat("Hello! Please respond with just 'OK' if you can read this.").await {
            Ok(response) => {
                println!("✅ DeepSeek responded: {}", response);
                assert!(!response.is_empty(), "Response should not be empty");
            }
            Err(e) => {
                println!("❌ DeepSeek connection failed: {}", e);
                panic!("Failed to connect to DeepSeek: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_ai_context_provider() {
        use op_dbus::mcp::ai_context_provider::AiContextProvider;

        println!("🧪 Testing system context gathering...");

        let provider = AiContextProvider::new();

        match provider.gather_context().await {
            Ok(context) => {
                println!("✅ System context gathered successfully!");
                println!("   CPU: {}", context.hardware.cpu_model);
                println!("   Cores: {}", context.hardware.cpu_cores);
                println!("   Memory: {:.1} GB", context.hardware.memory_gb);
                println!("   Virtual Machine: {}", context.hardware.is_virtual_machine);

                let summary = provider.generate_summary(&context);
                println!("\n📊 System Summary:\n{}", summary);

                assert!(!context.hardware.cpu_model.is_empty(), "CPU model should be detected");
            }
            Err(e) => {
                println!("❌ Failed to gather context: {}", e);
                panic!("Context gathering failed: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Run with: cargo test --lib --features web test_ollama");
}
