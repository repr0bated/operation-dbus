// Integration: Register introspection tools with existing MCP ToolRegistry
// This adds CPU feature detection, ISP analysis, and hardware discovery to MCP

use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;

use super::tool_registry::{DynamicToolBuilder, Tool, ToolContent, ToolRegistry, ToolResult};
use crate::introspection::{CpuFeatureAnalyzer, SystemIntrospector};
use crate::isp_migration::IspMigrationAnalyzer;
use crate::isp_support;

/// Register all introspection tools with the MCP tool registry
pub async fn register_introspection_tools(registry: &ToolRegistry) -> Result<()> {
    // Tool 1: System introspection
    register_discover_system(registry).await?;

    // Tool 2: CPU feature analysis
    register_analyze_cpu_features(registry).await?;

    // Tool 3: ISP migration analysis
    register_analyze_isp(registry).await?;

    // Tool 4: Generate ISP support request
    register_generate_isp_request(registry).await?;

    // Tool 5: Compare configurations
    register_compare_hardware(registry).await?;

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
                    .unwrap_or(true); // Default to true - always check provider

                // Run introspection
                let introspector = SystemIntrospector::new();
                match introspector.introspect_system().await {
                    Ok(report) => {
                        let mut result = serde_json::to_value(&report)
                            .unwrap_or_else(|_| json!({"error": "Failed to serialize"}));

                        // Add ISP analysis if requested
                        if detect_provider {
                            let analyzer = IspMigrationAnalyzer::new();
                            if let Ok(migration_report) = analyzer.analyze() {
                                if let Ok(isp_value) = serde_json::to_value(&migration_report) {
                                    result["isp_analysis"] = isp_value;
                                }
                            }
                        }

                        Ok(ToolResult::success(ToolContent::text(
                            serde_json::to_string_pretty(&result).unwrap()
                        )))
                    }
                    Err(e) => Ok(ToolResult::error(&format!("Introspection failed: {}", e))),
                }
            })
        })
        .build();

    registry.register_tool(Box::new(tool)).await?;
    Ok(())
}

/// Tool: analyze_cpu_features - CPU feature and BIOS lock detection
async fn register_analyze_cpu_features(registry: &ToolRegistry) -> Result<()> {
    let tool = DynamicToolBuilder::new("analyze_cpu_features")
        .description("Detect CPU features, BIOS locks, and hidden capabilities (VT-x, IOMMU, SGX, Turbo Boost)")
        .schema(json!({
            "type": "object",
            "properties": {}
        }))
        .handler(|_params| {
            Box::pin(async move {
                let analyzer = CpuFeatureAnalyzer::new();
                match analyzer.analyze() {
                    Ok(analysis) => {
                        let result = serde_json::to_value(&analysis)
                            .unwrap_or_else(|_| json!({"error": "Failed to serialize"}));

                        Ok(ToolResult::success(ToolContent::text(
                            serde_json::to_string_pretty(&result).unwrap()
                        )))
                    }
                    Err(e) => Ok(ToolResult::error(&format!("CPU analysis failed: {}", e))),
                }
            })
        })
        .build();

    registry.register_tool(Box::new(tool)).await?;
    Ok(())
}

/// Tool: analyze_isp - ISP restriction and migration analysis
async fn register_analyze_isp(registry: &ToolRegistry) -> Result<()> {
    let tool = DynamicToolBuilder::new("analyze_isp")
        .description("Analyze current ISP/provider restrictions and recommend alternatives with cost comparison")
        .schema(json!({
            "type": "object",
            "properties": {
                "requirements": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["gpu", "nested-virt", "iommu", "full-access"]
                    },
                    "description": "Required features to filter recommendations"
                }
            }
        }))
        .handler(|params| {
            Box::pin(async move {
                let analyzer = IspMigrationAnalyzer::new();
                match analyzer.analyze() {
                    Ok(report) => {
                        let mut result = serde_json::to_value(&report)
                            .unwrap_or_else(|_| json!({"error": "Failed to serialize"}));

                        // Filter recommendations based on requirements if provided
                        if let Some(requirements) = params.get("requirements").and_then(|v| v.as_array()) {
                            let req_strs: Vec<String> = requirements
                                .iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect();

                            let filtered_providers: Vec<_> = report.recommended_providers
                                .into_iter()
                                .filter(|provider| {
                                    req_strs.iter().all(|req| match req.as_str() {
                                        "gpu" => provider.gpu_passthrough,
                                        "nested-virt" => provider.nested_virt,
                                        "iommu" => provider.iommu_available,
                                        "full-access" => provider.full_hardware_access,
                                        _ => true,
                                    })
                                })
                                .collect();

                            if let Ok(filtered_value) = serde_json::to_value(&filtered_providers) {
                                result["recommended_providers"] = filtered_value;
                            }
                        }

                        Ok(ToolResult::success(ToolContent::text(
                            serde_json::to_string_pretty(&result).unwrap()
                        )))
                    }
                    Err(e) => Ok(ToolResult::error(&format!("ISP analysis failed: {}", e))),
                }
            })
        })
        .build();

    registry.register_tool(Box::new(tool)).await?;
    Ok(())
}

/// Tool: generate_isp_request - Generate professional support request
async fn register_generate_isp_request(registry: &ToolRegistry) -> Result<()> {
    let tool = DynamicToolBuilder::new("generate_isp_request")
        .description("Generate professional technical support request for ISP to enable features")
        .schema(json!({
            "type": "object",
            "properties": {
                "feature": {
                    "type": "string",
                    "enum": ["gpu-passthrough", "nested-virt", "iommu"],
                    "description": "Feature to request from ISP"
                },
                "use_case": {
                    "type": "string",
                    "description": "Brief description of your use case"
                }
            },
            "required": ["feature"]
        }))
        .handler(|params| {
            Box::pin(async move {
                let feature = match params.get("feature").and_then(|v| v.as_str()) {
                    Some(f) => f,
                    None => return Ok(ToolResult::error("feature parameter is required")),
                };

                let use_case = params
                    .get("use_case")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Production infrastructure requiring advanced features");

                let request_text = match feature {
                    "gpu-passthrough" => {
                        isp_support::generate_gpu_passthrough_request(None, use_case)
                    }
                    "nested-virt" => {
                        isp_support::generate_nested_virt_request(use_case)
                    }
                    "iommu" => {
                        isp_support::generate_iommu_enable_request()
                    }
                    _ => {
                        return Ok(ToolResult::error(&format!("Unknown feature: {}", feature)));
                    }
                };

                match request_text {
                    Ok(text) => {
                        let result = json!({
                            "feature": feature,
                            "request_text": text,
                            "generated_at": chrono::Utc::now().to_rfc3339(),
                        });

                        Ok(ToolResult::success(ToolContent::text(
                            serde_json::to_string_pretty(&result).unwrap()
                        )))
                    }
                    Err(e) => Ok(ToolResult::error(&format!("Failed to generate request: {}", e))),
                }
            })
        })
        .build();

    registry.register_tool(Box::new(tool)).await?;
    Ok(())
}

/// Tool: compare_hardware - Compare two hardware configurations
async fn register_compare_hardware(registry: &ToolRegistry) -> Result<()> {
    let tool = DynamicToolBuilder::new("compare_hardware")
        .description("Compare two hardware configurations (e.g., VPS vs bare metal, HostKey vs Hetzner)")
        .schema(json!({
            "type": "object",
            "properties": {
                "config1_path": {
                    "type": "string",
                    "description": "Path to first introspection JSON file"
                },
                "config2_path": {
                    "type": "string",
                    "description": "Path to second introspection JSON file"
                }
            },
            "required": ["config1_path", "config2_path"]
        }))
        .handler(|params| {
            Box::pin(async move {
                let config1_path = match params.get("config1_path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return Ok(ToolResult::error("config1_path is required")),
                };

                let config2_path = match params.get("config2_path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => return Ok(ToolResult::error("config2_path is required")),
                };

                // Read both configs
                let config1_result = tokio::fs::read_to_string(config1_path).await;
                let config2_result = tokio::fs::read_to_string(config2_path).await;

                match (config1_result, config2_result) {
                    (Ok(config1_str), Ok(config2_str)) => {
                        match (
                            serde_json::from_str::<Value>(&config1_str),
                            serde_json::from_str::<Value>(&config2_str),
                        ) {
                            (Ok(config1), Ok(config2)) => {
                                let mut differences = vec![];

                                // Compare key aspects
                                if let (Some(cpu1), Some(cpu2)) = (
                                    config1.get("system_config").and_then(|c| c.get("cpu_features")),
                                    config2.get("system_config").and_then(|c| c.get("cpu_features")),
                                ) {
                                    differences.push(json!({
                                        "category": "cpu_features",
                                        "config1": cpu1,
                                        "config2": cpu2,
                                    }));
                                }

                                // Hardware comparison
                                if let (Some(hw1), Some(hw2)) = (
                                    config1.get("system_config").and_then(|c| c.get("hardware")),
                                    config2.get("system_config").and_then(|c| c.get("hardware")),
                                ) {
                                    differences.push(json!({
                                        "category": "hardware",
                                        "config1": hw1,
                                        "config2": hw2,
                                    }));
                                }

                                let result = json!({
                                    "comparison": {
                                        "config1": config1_path,
                                        "config2": config2_path,
                                        "differences": differences,
                                    }
                                });

                                Ok(ToolResult::success(ToolContent::text(
                                    serde_json::to_string_pretty(&result).unwrap()
                                )))
                            }
                            _ => Ok(ToolResult::error("Failed to parse JSON configs")),
                        }
                    }
                    _ => Ok(ToolResult::error("Failed to read configuration files")),
                }
            })
        })
        .build();

    registry.register_tool(Box::new(tool)).await?;
    Ok(())
}
