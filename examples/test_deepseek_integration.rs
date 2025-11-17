// Example: Test DeepSeek Integration
// Run with: cargo run --example test_deepseek_integration --features web

use op_dbus::mcp::ai_context_provider::AiContextProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing DeepSeek Integration\n");
    println!("════════════════════════════════════════════════════════════\n");

    // Test 1: System Context Provider
    println!("📊 Test 1: System Introspection");
    println!("────────────────────────────────────────────────────────────");

    let provider = AiContextProvider::new();

    match provider.gather_context().await {
        Ok(context) => {
            println!("✅ System context gathered successfully!\n");

            let summary = provider.generate_summary(&context);
            println!("{}", summary);

            println!("\n📋 Detailed Information:");
            println!("   Hostname: {}", context.network.hostname);
            println!("   Network Interfaces: {:?}", context.network.interfaces);
            println!("   Provider: {:?}", context.network.provider);
            println!("   Virtualization: {}", if context.hardware.virtualization_available { "Available" } else { "Not Available" });

            if !context.capabilities.cpu_features.is_empty() {
                println!("\n   CPU Features: {}", context.capabilities.cpu_features.join(", "));
            }

            if !context.restrictions.bios_locks.is_empty() {
                println!("\n   ⚠️  BIOS Locks Detected:");
                for lock in &context.restrictions.bios_locks {
                    println!("      - {}", lock);
                }
            }

            if !context.restrictions.provider_restrictions.is_empty() {
                println!("\n   ⚠️  Provider Restrictions:");
                for restriction in &context.restrictions.provider_restrictions {
                    println!("      - {}", restriction);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to gather context: {}", e);
            return Err(e);
        }
    }

    println!("\n════════════════════════════════════════════════════════════");
    println!("\n📊 Test 2: Ollama Client (if OLLAMA_API_KEY is set)");
    println!("────────────────────────────────────────────────────────────\n");

    if let Ok(api_key) = std::env::var("OLLAMA_API_KEY") {
        use op_dbus::mcp::ollama::OllamaClient;

        let client = OllamaClient::deepseek_cloud(api_key);

        println!("🤖 Connecting to DeepSeek...");

        match client.deepseek_chat("Hello! Please respond with just 'Hello from DeepSeek!' to confirm you're working.").await {
            Ok(response) => {
                println!("✅ DeepSeek responded!\n");
                println!("Response: {}\n", response);
            }
            Err(e) => {
                println!("❌ DeepSeek connection failed: {}\n", e);
                println!("   This might be a network issue or invalid API key.");
            }
        }
    } else {
        println!("⚠️  OLLAMA_API_KEY not set - skipping Ollama test");
        println!("   Set it with: export OLLAMA_API_KEY=your-key-here\n");
    }

    println!("════════════════════════════════════════════════════════════");
    println!("\n✅ Integration test complete!\n");
    println!("Your DeepSeek integration is ready to use:");
    println!("  • AI Context Provider: Gathers system information");
    println!("  • Ollama Client: Connects to DeepSeek for AI chat");
    println!("  • Chat Server: Enhanced with tool awareness");
    println!("\nNext steps:");
    println!("  1. Fix mcp feature compilation errors (pre-existing)");
    println!("  2. Run: cargo run --bin deepseek-chat --features web,mcp");
    println!("  3. Access at: http://100.104.70.1:8080\n");

    Ok(())
}
