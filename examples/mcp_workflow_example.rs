//! Example demonstrating MCP workflows using PocketFlow
//!
//! This example shows how to create complex agent workflows where:
//! 1. A code analysis agent reviews code
//! 2. A test generation agent creates tests based on the analysis
//! 3. A documentation agent updates docs
//! 4. A deployment agent prepares the release

use anyhow::Result;
use pocketflow_rs::Context;
use serde_json::Value;

#[cfg(feature = "mcp")]
use op_dbus::mcp::workflows::{McpWorkflowManager, McpWorkflowState};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 PocketFlow Integration Example");
    println!("===================================");
    println!("This example demonstrates flow-based programming");
    println!("for complex agent interactions using PocketFlow.\n");

    // Demonstrate basic PocketFlow usage
    println!("🔄 Creating a simple flow...");

    // For now, just show that PocketFlow is available
    let context = Context::new();
    println!("✅ PocketFlow Context created: {:?}", context);
    println!("✅ PocketFlow integration ready for MCP workflows!");

    #[cfg(feature = "mcp")]
    {
        println!("\n🤖 MCP Workflows (requires --features mcp):");

        // Create workflow manager
        let mut manager = McpWorkflowManager::new();

        // Create a Rust code review workflow
        println!("📋 Creating Rust code review workflow...");
        manager.create_code_review_workflow("rust")?;

        // List available workflows
        let workflows = manager.list_workflows();
        println!("📋 Available workflows: {:?}", workflows);

        // Create context with sample code
        let mut context = Context::new();
        context.set(
            "code".to_string(),
            Value::String(r#"
    fn main() {
        println!("Hello, World!");
        let x = 42;
        if x > 0 {
            println!("Positive number: {}", x);
        }
    }
    "#.to_string())
        );

        // Run the workflow
        println!("⚡ Running code review workflow...");
        let result = manager.run_workflow("code_review_rust", context).await?;

        println!("✅ Workflow completed!");
        println!("📊 Final result: {:?}", result);
    }

    #[cfg(not(feature = "mcp"))]
    {
        println!("\n💡 To enable MCP workflows, run with:");
        println!("   cargo run --example mcp-workflow --features mcp");
    }

    Ok(())
}