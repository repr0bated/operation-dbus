// MCP tools for system introspection
// Exposes CPU feature detection, ISP analysis, and hardware discovery to AI agents

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::introspection::{SystemIntrospector, CpuFeatureAnalyzer};
use crate::isp_migration::IspMigrationAnalyzer;
use crate::mcp::system_introspection::SystemIntrospector as DbusIntrospector;

/// MCP Tool: Discover system hardware and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverSystemTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub type_: String,
    pub description: String,
    pub required: bool,
}

impl DiscoverSystemTool {
    pub fn new() -> Self {
        Self {
            name: "discover_system".to_string(),
            description: "Introspect system hardware, CPU features, BIOS locks, and service configuration".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "include_packages".to_string(),
                    type_: "boolean".to_string(),
                    description: "Include installed packages in discovery".to_string(),
                    required: false,
                },
                ToolParameter {
                    name: "detect_provider".to_string(),
                    type_: "boolean".to_string(),
                    description: "Detect and analyze ISP/cloud provider".to_string(),
                    required: false,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let include_packages = params
            .get("include_packages")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let detect_provider = params
            .get("detect_provider")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Run introspection
        let introspector = SystemIntrospector::new();
        let mut report = introspector.introspect_system().await?;

        // Add ISP analysis if requested
        if detect_provider {
            let isp_analyzer = IspMigrationAnalyzer::new();
            if let Ok(migration_report) = isp_analyzer.analyze() {
                let mut result = serde_json::to_value(&report)?;
                result["isp_analysis"] = serde_json::to_value(&migration_report)?;
                return Ok(result);
            }
        }

        Ok(serde_json::to_value(&report)?)
    }
}

/// MCP Tool: Analyze CPU features and BIOS locks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeCpuFeaturesTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl AnalyzeCpuFeaturesTool {
    pub fn new() -> Self {
        Self {
            name: "analyze_cpu_features".to_string(),
            description: "Detect CPU features, BIOS locks, and hidden capabilities (VT-x, IOMMU, SGX, etc.)".to_string(),
            parameters: vec![],
        }
    }

    pub async fn execute(&self, _params: HashMap<String, Value>) -> Result<Value> {
        let analyzer = CpuFeatureAnalyzer::new();
        let analysis = analyzer.analyze()?;
        Ok(serde_json::to_value(&analysis)?)
    }
}

/// MCP Tool: Analyze ISP restrictions and migration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeIspTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl AnalyzeIspTool {
    pub fn new() -> Self {
        Self {
            name: "analyze_isp".to_string(),
            description: "Analyze current ISP/provider restrictions and recommend alternatives".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "requirements".to_string(),
                    type_: "array".to_string(),
                    description: "Required features: gpu, nested-virt, iommu, etc.".to_string(),
                    required: false,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let analyzer = IspMigrationAnalyzer::new();
        let report = analyzer.analyze()?;

        // Filter recommendations based on requirements if provided
        if let Some(requirements) = params.get("requirements").and_then(|v| v.as_array()) {
            let req_strs: Vec<String> = requirements
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();

            let mut result = serde_json::to_value(&report)?;

            // Filter providers that meet requirements
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

            result["recommended_providers"] = serde_json::to_value(&filtered_providers)?;
            return Ok(result);
        }

        Ok(serde_json::to_value(&report)?)
    }
}

/// MCP Tool: Generate ISP support request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateIspRequestTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl GenerateIspRequestTool {
    pub fn new() -> Self {
        Self {
            name: "generate_isp_request".to_string(),
            description: "Generate professional support request for ISP to enable features".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "feature".to_string(),
                    type_: "string".to_string(),
                    description: "Feature to request: gpu-passthrough, nested-virt, iommu".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "use_case".to_string(),
                    type_: "string".to_string(),
                    description: "Brief description of use case".to_string(),
                    required: false,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let feature = params
            .get("feature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("feature parameter required"))?;

        let use_case = params
            .get("use_case")
            .and_then(|v| v.as_str())
            .unwrap_or("Production infrastructure requiring advanced features");

        let request_text = match feature {
            "gpu-passthrough" => {
                crate::isp_support::generate_gpu_passthrough_request(None, use_case)?
            }
            "nested-virt" => {
                crate::isp_support::generate_nested_virt_request(use_case)?
            }
            "iommu" => {
                crate::isp_support::generate_iommu_enable_request()?
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown feature: {}", feature));
            }
        };

        Ok(serde_json::json!({
            "feature": feature,
            "request_text": request_text,
            "generated_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

/// MCP Tool: Compare hardware configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareHardwareTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl CompareHardwareTool {
    pub fn new() -> Self {
        Self {
            name: "compare_hardware".to_string(),
            description: "Compare two hardware configurations (e.g., VPS vs bare metal, before vs after migration)".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "config1_path".to_string(),
                    type_: "string".to_string(),
                    description: "Path to first introspection JSON".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "config2_path".to_string(),
                    type_: "string".to_string(),
                    description: "Path to second introspection JSON".to_string(),
                    required: true,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let config1_path = params
            .get("config1_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("config1_path required"))?;

        let config2_path = params
            .get("config2_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("config2_path required"))?;

        // Read both configs
        let config1_str = tokio::fs::read_to_string(config1_path).await?;
        let config2_str = tokio::fs::read_to_string(config2_path).await?;

        let config1: Value = serde_json::from_str(&config1_str)?;
        let config2: Value = serde_json::from_str(&config2_str)?;

        // Compare key aspects
        let mut differences = vec![];

        // CPU features
        if let (Some(cpu1), Some(cpu2)) = (
            config1.get("system_config").and_then(|c| c.get("cpu_features")),
            config2.get("system_config").and_then(|c| c.get("cpu_features")),
        ) {
            differences.push(serde_json::json!({
                "category": "cpu_features",
                "config1": cpu1,
                "config2": cpu2,
            }));
        }

        // Hardware
        if let (Some(hw1), Some(hw2)) = (
            config1.get("system_config").and_then(|c| c.get("hardware")),
            config2.get("system_config").and_then(|c| c.get("hardware")),
        ) {
            differences.push(serde_json::json!({
                "category": "hardware",
                "config1": hw1,
                "config2": hw2,
            }));
        }

        // Virtualization
        if let (Some(virt1), Some(virt2)) = (
            config1.get("system_config").and_then(|c| c.get("virtualization")),
            config2.get("system_config").and_then(|c| c.get("virtualization")),
        ) {
            differences.push(serde_json::json!({
                "category": "virtualization",
                "config1": virt1,
                "config2": virt2,
            }));
        }

        Ok(serde_json::json!({
            "comparison": {
                "config1": config1_path,
                "config2": config2_path,
                "differences": differences,
            }
        }))
    }
}

/// MCP Tool: Query cached D-Bus methods for a service/interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCachedDbusMethodsTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl QueryCachedDbusMethodsTool {
    pub fn new() -> Self {
        Self {
            name: "query_cached_dbus_methods".to_string(),
            description: "Fast query for D-Bus methods from cache (no D-Bus call overhead)".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "service_name".to_string(),
                    type_: "string".to_string(),
                    description: "D-Bus service name (e.g., org.freedesktop.systemd1)".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "interface_name".to_string(),
                    type_: "string".to_string(),
                    description: "D-Bus interface name (e.g., org.freedesktop.systemd1.Manager)".to_string(),
                    required: true,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let service_name = params
            .get("service_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("service_name parameter required"))?;

        let interface_name = params
            .get("interface_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("interface_name parameter required"))?;

        let introspector = DbusIntrospector::new().await?;

        match introspector.query_cached_methods(service_name, interface_name)? {
            Some(methods) => Ok(serde_json::json!({
                "service": service_name,
                "interface": interface_name,
                "methods": methods,
                "source": "cache"
            })),
            None => Ok(serde_json::json!({
                "service": service_name,
                "interface": interface_name,
                "methods": [],
                "source": "cache_miss",
                "message": "No cached data found. Run introspection first to populate cache."
            })),
        }
    }
}

/// MCP Tool: Search D-Bus methods by pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDbusMethodsTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl SearchDbusMethodsTool {
    pub fn new() -> Self {
        Self {
            name: "search_dbus_methods".to_string(),
            description: "Search cached D-Bus methods by name pattern across all services".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "pattern".to_string(),
                    type_: "string".to_string(),
                    description: "Search pattern (case-sensitive, SQL LIKE syntax with %)".to_string(),
                    required: true,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("pattern parameter required"))?;

        let introspector = DbusIntrospector::new().await?;
        let results = introspector.search_cached_methods(pattern)?;

        Ok(serde_json::json!({
            "pattern": pattern,
            "results": results,
            "count": results.len()
        }))
    }
}

/// MCP Tool: Get introspection cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCacheStatsTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl GetCacheStatsTool {
    pub fn new() -> Self {
        Self {
            name: "get_introspection_cache_stats".to_string(),
            description: "Get statistics about the D-Bus introspection cache".to_string(),
            parameters: vec![],
        }
    }

    pub async fn execute(&self, _params: HashMap<String, Value>) -> Result<Value> {
        let introspector = DbusIntrospector::new().await?;

        match introspector.get_cache_stats() {
            Some(stats) => Ok(stats),
            None => Ok(serde_json::json!({
                "cache_enabled": false,
                "message": "Introspection cache is not available"
            })),
        }
    }
}

/// MCP Tool: Warm the introspection cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmCacheTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl WarmCacheTool {
    pub fn new() -> Self {
        Self {
            name: "warm_introspection_cache".to_string(),
            description: "Proactively cache common D-Bus services for faster future queries".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "services".to_string(),
                    type_: "array".to_string(),
                    description: "Optional list of service names. If empty, caches priority services".to_string(),
                    required: false,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let introspector = DbusIntrospector::new().await?;

        let services = if let Some(services_val) = params.get("services") {
            if let Some(arr) = services_val.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            } else {
                get_priority_services()
            }
        } else {
            get_priority_services()
        };

        let mut cached = Vec::new();
        let mut failed = Vec::new();

        for service_name in &services {
            match introspector.introspect_service(service_name).await {
                Ok(service) => {
                    cached.push(serde_json::json!({
                        "service": service_name,
                        "object_paths": service.object_paths.len(),
                        "interfaces": service.interfaces.len()
                    }));
                }
                Err(e) => {
                    failed.push(serde_json::json!({
                        "service": service_name,
                        "error": e.to_string()
                    }));
                }
            }
        }

        Ok(serde_json::json!({
            "cached": cached,
            "failed": failed,
            "total_attempted": services.len(),
            "success_count": cached.len()
        }))
    }
}

/// MCP Tool: List all available D-Bus services (DISCOVERY TOOL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDbusServicesTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl ListDbusServicesTool {
    pub fn new() -> Self {
        Self {
            name: "list_dbus_services".to_string(),
            description: "List all available D-Bus services on the system bus. Use this FIRST to discover what services exist before querying them.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "include_activatable".to_string(),
                    type_: "boolean".to_string(),
                    description: "Include services that can be activated but aren't currently running".to_string(),
                    required: false,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let include_activatable = params
            .get("include_activatable")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let introspector = DbusIntrospector::new().await?;
        let mut active_services = introspector.list_all_services().await?;
        active_services.sort();

        let mut result = serde_json::json!({
            "active_services": active_services,
            "count": active_services.len(),
        });

        if include_activatable {
            let mut activatable = introspector.list_activatable_services().await?;
            activatable.sort();
            result["activatable_services"] = serde_json::json!(activatable);
            result["activatable_count"] = serde_json::json!(activatable.len());
        }

        // Add helpful examples of common services
        result["examples"] = serde_json::json!({
            "systemd": "org.freedesktop.systemd1",
            "networkmanager": "org.freedesktop.NetworkManager",
            "login": "org.freedesktop.login1",
            "packagekit": "org.freedesktop.PackageKit",
            "upower": "org.freedesktop.UPower",
            "udisks": "org.freedesktop.UDisks2"
        });

        result["next_steps"] = serde_json::json!({
            "explore_service": "Use list_dbus_object_paths with a service name to see what objects it provides",
            "introspect_object": "Use introspect_dbus_object to see interfaces/methods/properties"
        });

        Ok(result)
    }
}

/// MCP Tool: List object paths for a D-Bus service (DISCOVERY TOOL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDbusObjectPathsTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl ListDbusObjectPathsTool {
    pub fn new() -> Self {
        Self {
            name: "list_dbus_object_paths".to_string(),
            description: "List all object paths provided by a D-Bus service. Use this SECOND after listing services to discover what objects a service provides.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "service_name".to_string(),
                    type_: "string".to_string(),
                    description: "D-Bus service name (e.g., org.freedesktop.systemd1). Get this from list_dbus_services first.".to_string(),
                    required: true,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let service_name = params
            .get("service_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("service_name parameter required. Use list_dbus_services to discover available services."))?;

        let introspector = DbusIntrospector::new().await?;

        // Introspect the service to get all paths
        match introspector.introspect_service(service_name).await {
            Ok(service_info) => {
                let mut paths = service_info.object_paths;
                paths.sort();

                Ok(serde_json::json!({
                    "service": service_name,
                    "object_paths": paths,
                    "count": paths.len(),
                    "next_steps": {
                        "introspect_object": format!("Use introspect_dbus_object with service={} and one of these paths", service_name)
                    }
                }))
            }
            Err(e) => {
                Err(anyhow::anyhow!(
                    "Failed to introspect service '{}': {}. Tip: Use list_dbus_services to verify the service name is correct.",
                    service_name, e
                ))
            }
        }
    }
}

/// MCP Tool: Introspect a specific D-Bus object (DISCOVERY TOOL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntrospectDbusObjectTool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
}

impl IntrospectDbusObjectTool {
    pub fn new() -> Self {
        Self {
            name: "introspect_dbus_object".to_string(),
            description: "Introspect a specific D-Bus object to see what interfaces, methods, properties, and signals it provides. Use this THIRD after listing services and paths.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "service_name".to_string(),
                    type_: "string".to_string(),
                    description: "D-Bus service name (from list_dbus_services)".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "object_path".to_string(),
                    type_: "string".to_string(),
                    description: "Object path (from list_dbus_object_paths)".to_string(),
                    required: true,
                },
            ],
        }
    }

    pub async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        let service_name = params
            .get("service_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("service_name required. Use list_dbus_services first."))?;

        let object_path = params
            .get("object_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("object_path required. Use list_dbus_object_paths first."))?;

        let introspector = DbusIntrospector::new().await?;

        // Get introspection XML and parse it
        match introspector.introspect_service_at_path(service_name, object_path).await {
            Ok(xml) => {
                // Parse XML to extract interfaces, methods, properties, signals
                use zbus_xml::Node;
                let node = Node::from_reader(xml.as_bytes())?;

                let mut interfaces_info = vec![];

                for interface in node.interfaces() {
                    let iface_name = interface.name().as_ref().to_string();

                    // Extract methods
                    let methods: Vec<_> = interface.methods().iter().map(|m| {
                        let method_name = m.name().as_ref().to_string();
                        let in_args: Vec<_> = m.args().iter()
                            .filter(|a| matches!(a.direction(), Some(zbus_xml::ArgDirection::In)))
                            .map(|a| serde_json::json!({
                                "name": a.name().unwrap_or(""),
                                "type": a.ty()
                            }))
                            .collect();
                        let out_args: Vec<_> = m.args().iter()
                            .filter(|a| matches!(a.direction(), Some(zbus_xml::ArgDirection::Out)))
                            .map(|a| serde_json::json!({
                                "name": a.name().unwrap_or(""),
                                "type": a.ty()
                            }))
                            .collect();

                        serde_json::json!({
                            "name": method_name,
                            "in_args": in_args,
                            "out_args": out_args
                        })
                    }).collect();

                    // Extract properties
                    let properties: Vec<_> = interface.properties().iter().map(|p| {
                        let access_str = match p.access() {
                            zbus_xml::PropertyAccess::Read => "read",
                            zbus_xml::PropertyAccess::Write => "write",
                            zbus_xml::PropertyAccess::ReadWrite => "readwrite",
                        };
                        serde_json::json!({
                            "name": p.name().as_ref(),
                            "type": p.ty(),
                            "access": access_str
                        })
                    }).collect();

                    // Extract signals
                    let signals: Vec<_> = interface.signals().iter().map(|s| {
                        serde_json::json!({
                            "name": s.name().as_ref(),
                            "args": s.args().iter().map(|a| serde_json::json!({
                                "name": a.name().unwrap_or(""),
                                "type": a.ty()
                            })).collect::<Vec<_>>()
                        })
                    }).collect();

                    interfaces_info.push(serde_json::json!({
                        "name": iface_name,
                        "methods": methods,
                        "properties": properties,
                        "signals": signals
                    }));
                }

                // Extract child nodes
                let child_nodes: Vec<String> = node.nodes().iter()
                    .filter_map(|n| n.name().map(|name| name.to_string()))
                    .collect();

                Ok(serde_json::json!({
                    "service": service_name,
                    "object_path": object_path,
                    "interfaces": interfaces_info,
                    "child_nodes": child_nodes,
                    "summary": {
                        "interface_count": interfaces_info.len(),
                        "total_methods": interfaces_info.iter().map(|i| i["methods"].as_array().unwrap().len()).sum::<usize>(),
                        "total_properties": interfaces_info.iter().map(|i| i["properties"].as_array().unwrap().len()).sum::<usize>(),
                        "total_signals": interfaces_info.iter().map(|i| i["signals"].as_array().unwrap().len()).sum::<usize>(),
                    }
                }))
            }
            Err(e) => {
                Err(anyhow::anyhow!(
                    "Failed to introspect {}{}: {}. Tip: Use list_dbus_object_paths to verify the path exists.",
                    service_name, object_path, e
                ))
            }
        }
    }
}

/// Get list of priority D-Bus services to cache
fn get_priority_services() -> Vec<String> {
    vec![
        "org.freedesktop.systemd1".to_string(),
        "org.freedesktop.NetworkManager".to_string(),
        "org.freedesktop.login1".to_string(),
        "org.freedesktop.PackageKit".to_string(),
        "org.freedesktop.UDisks2".to_string(),
        "org.freedesktop.UPower".to_string(),
    ]
}

/// Register all introspection tools
pub fn register_introspection_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        // Discovery tools (MOST IMPORTANT - use these first!)
        Box::new(ListDbusServicesTool::new()),
        Box::new(ListDbusObjectPathsTool::new()),
        Box::new(IntrospectDbusObjectTool::new()),

        // System introspection tools
        Box::new(DiscoverSystemTool::new()),
        Box::new(AnalyzeCpuFeaturesTool::new()),
        Box::new(AnalyzeIspTool::new()),
        Box::new(GenerateIspRequestTool::new()),
        Box::new(CompareHardwareTool::new()),

        // Cache query tools (require knowing service/interface names)
        Box::new(QueryCachedDbusMethodsTool::new()),
        Box::new(SearchDbusMethodsTool::new()),
        Box::new(GetCacheStatsTool::new()),
        Box::new(WarmCacheTool::new()),
    ]
}

/// Trait for MCP tools
#[async_trait::async_trait]
pub trait McpTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &[ToolParameter];
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value>;
}

// Implement McpTool for all our tools
#[async_trait::async_trait]
impl McpTool for DiscoverSystemTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for AnalyzeCpuFeaturesTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for AnalyzeIspTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for GenerateIspRequestTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for CompareHardwareTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for QueryCachedDbusMethodsTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for SearchDbusMethodsTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for GetCacheStatsTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for WarmCacheTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for ListDbusServicesTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for ListDbusObjectPathsTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}

#[async_trait::async_trait]
impl McpTool for IntrospectDbusObjectTool {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn parameters(&self) -> &[ToolParameter] {
        &self.parameters
    }
    async fn execute(&self, params: HashMap<String, Value>) -> Result<Value> {
        self.execute(params).await
    }
}
