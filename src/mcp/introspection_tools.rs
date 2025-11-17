// Integration: Register introspection tools with existing MCP ToolRegistry
// This adds system discovery and hardware introspection to MCP

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use super::tool_registry::{DynamicToolBuilder, Tool, ToolContent, ToolRegistry, ToolResult};

/// Register all introspection tools with the MCP tool registry
pub async fn register_introspection_tools(registry: &ToolRegistry) -> Result<()> {
    // Tool 1: System introspection
    register_discover_system(registry).await?;

    Ok(())
}

/// Tool: discover_system - Full system introspection
async fn register_discover_system(registry: &ToolRegistry) -> Result<()> {
    let tool = DynamicToolBuilder::new("discover_system")
        .description("Introspect system hardware, CPU features, BIOS locks, D-Bus services, and configuration")
        .schema(json!({
            "type": "object",
            "properties": {
                "include_packages": {
                    "type": "boolean",
                    "description": "Include installed packages in discovery"
                },
                "detect_provider": {
                    "type": "boolean",
                    "description": "Detect and analyze ISP/cloud provider restrictions"
                }
            }
        }))
        .handler(|params| {
            Box::pin(async move {
                let include_packages = params
                    .get("include_packages")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let detect_provider = params
                    .get("detect_provider")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                // Perform system introspection using system commands
                use std::process::Command;
                
                let mut result = json!({
                    "system_info": {},
                    "hardware": {},
                    "cpu_features": {},
                    "network": {},
                    "services": {}
                });

                // Get basic system info
                if let Ok(output) = Command::new("uname").arg("-a").output() {
                    let uname = String::from_utf8_lossy(&output.stdout);
                    result["system_info"]["kernel"] = json!(uname.trim());
                }

                // Get CPU info
                if let Ok(output) = Command::new("lscpu").output() {
                    let lscpu = String::from_utf8_lossy(&output.stdout);
                    let cpu_info: Vec<&str> = lscpu.lines().take(10).collect();
                    result["hardware"]["cpu"] = json!(cpu_info);
                }

                // Get memory info
                if let Ok(output) = Command::new("free").arg("-h").output() {
                    let memory = String::from_utf8_lossy(&output.stdout);
                    result["hardware"]["memory"] = json!(memory.trim());
                }

                // Get network interfaces
                if let Ok(output) = Command::new("ip").arg("addr").output() {
                    let interfaces = String::from_utf8_lossy(&output.stdout);
                    result["network"]["interfaces"] = json!(interfaces.lines().take(20).collect::<Vec<_>>());
                }

                // Get running services
                if let Ok(output) = Command::new("systemctl")
                    .args(&["list-units", "--type=service", "--state=running", "--no-pager"])
                    .output() 
                {
                    let services = String::from_utf8_lossy(&output.stdout);
                    result["services"]["running"] = json!(services.lines().take(10).collect::<Vec<_>>());
                }

                // Check for virtualization
                if let Ok(output) = Command::new("systemd-detect-virt").output() {
                    let virt = String::from_utf8_lossy(&output.stdout);
                    result["system_info"]["virtualization"] = json!(virt.trim());
                }

                // CPU features from /proc/cpuinfo
                if let Ok(contents) = std::fs::read_to_string("/proc/cpuinfo") {
                    let flags_line = contents.lines()
                        .find(|line| line.starts_with("flags"))
                        .unwrap_or("");
                    let flags: Vec<&str> = flags_line
                        .split(':')
                        .nth(1)
                        .unwrap_or("")
                        .split_whitespace()
                        .take(20)
                        .collect();
                    result["cpu_features"]["flags"] = json!(flags);
                }

                // Add provider detection if requested
                if detect_provider {
                    result["isp_analysis"] = json!({
                        "detected_provider": "Unknown",
                        "restrictions": ["Full introspection requires additional modules"],
                        "note": "Provider detection simplified for compatibility"
                    });
                }

                // Add package info if requested
                if include_packages {
                    if let Ok(output) = Command::new("dpkg").args(&["-l"]).output() {
                        let packages = String::from_utf8_lossy(&output.stdout);
                        let package_lines: Vec<&str> = packages.lines()
                            .filter(|line| line.starts_with("ii"))
                            .take(10)
                            .collect();
                        result["packages"] = json!(package_lines);
                    }
                }

                result["timestamp"] = json!(chrono::Utc::now().to_rfc3339());
                result["introspection_version"] = json!("1.0-simplified");

                Ok(ToolResult::success(ToolContent::text(
                    serde_json::to_string_pretty(&result).unwrap()
                )))
            })
        })
        .build();

    registry.register_tool(Box::new(tool)).await?;
    Ok(())
}