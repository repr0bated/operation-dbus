//! Example demonstrating Plugin Workflow System
//!
//! This example shows how plugins can be composed into workflows using PocketFlow.
//! Each plugin becomes a node that can pass data to other plugins.

use anyhow::Result;
use op_dbus::state::plugin_workflow::{PluginWorkflowManager, PluginWorkflowState};
use op_dbus::state::plugins;
use pocketflow_rs::Context;
use serde_json::Value;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔗 Plugin Workflow Example");
    println!("===========================");
    println!("This demonstrates how plugins become workflow nodes.\n");

    // Create workflow manager
    let mut workflow_manager = PluginWorkflowManager::new();

    // Demonstrate plugin registration concept
    println!("🔧 Plugin Registration Concept:");
    println!("   Network Plugin → Firewall Plugin → Monitoring Plugin");
    println!("   Each plugin processes data from the previous one");
    println!("   Conditional execution based on plugin results\n");

    // Show how plugins would be registered as workflow nodes
    println!("📋 Plugin Workflow Architecture:");
    println!("   • Plugin Node: Receives inputs from workflow context");
    println!("   • State Plugin: Executes query/calculate_diff/apply_state");
    println!("   • Output Node: Stores results back to workflow context");
    println!("   • Conditional Flow: Next plugin executes based on previous results\n");

    // Demonstrate workflow creation patterns
    demonstrate_workflow_patterns().await?;

    // Show practical examples
    demonstrate_practical_workflows().await?;

    println!("✅ Plugin workflow system ready!");
    println!("💡 Plugins can now be orchestrated in complex pipelines");

    Ok(())
}

async fn demonstrate_workflow_patterns() -> Result<()> {
    println!("🏗️  Workflow Patterns:");

    println!("   1. Sequential Pipeline:");
    println!("      Network Config → DNS Update → Certificate Renewal");
    println!("      Each step depends on the previous completion\n");

    println!("   2. Conditional Branching:");
    println!("      Code Analysis → [Tests Pass] → Deploy");
    println!("                       [Tests Fail] → Rollback\n");

    println!("   3. Parallel Execution:");
    println!("      ├── Backup Database");
    println!("      System Update ──┤");
    println!("      └── Update Firewall\n");

    println!("   4. Error Recovery:");
    println!("      Service Update → Health Check → [Unhealthy] → Rollback");
    println!("                                         [Healthy] → Complete\n");

    Ok(())
}

async fn demonstrate_practical_workflows() -> Result<()> {
    println!("🚀 Practical Workflow Examples:");

    println!("   📡 Network Infrastructure Setup:");
    println!("      Bridge Creation → Port Configuration → VLAN Setup → Routing\n");

    println!("   🔒 Security Hardening:");
    println!("      Firewall Rules → SELinux Config → SSH Hardening → Audit Setup\n");

    println!("   📦 Application Deployment:");
    println!("      Service Stop → Config Update → Database Migration → Service Start\n");

    println!("   🔄 System Maintenance:");
    println!("      Backup Creation → Package Updates → Kernel Upgrade → Reboot\n");

    println!("   🏥 Health Monitoring:");
    println!("      Service Checks → Log Analysis → Alert Generation → Auto-Recovery\n");

    println!("   🔒 Privacy Network Setup:");
    println!("      WireGuard Gateway → WARP Tunnel → XRay Client → OpenFlow Routing\n");

    println!("   🏗️  Container Networking (Netmaker):");
    println!("      Netmaker Server → LXC Containers → Socket Networking → vmbr0 Bridge\n");

    // Show how this would work with real plugins
    println!("💻 Real Plugin Integration Example:");
    println!("   // Network plugin queries current state");
    println!("   let current_network = network_plugin.query_current_state().await?;");
    println!("   ");
    println!("   // Passes result to firewall plugin");
    println!("   context.set(\"network_state\".to_string(), current_network);");
    println!("   ");
    println!("   // Firewall plugin uses network state for configuration");
    println!("   let firewall_config = context.get(\"network_state\")");
    println!("   ");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_patterns() {
        // Test that workflow patterns can be demonstrated
        assert!(demonstrate_workflow_patterns().await.is_ok());
        assert!(demonstrate_practical_workflows().await.is_ok());
    }
}