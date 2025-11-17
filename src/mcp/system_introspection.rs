//! System Introspection Tool
//! Automatically discovers and introspects all D-Bus services
//! Generates MCP tool schemas for each discovered service
//!
//! Now uses HierarchicalIntrospector directly for comprehensive D-Bus discovery

use crate::introspection::HierarchicalIntrospector;
use crate::mcp::introspection_parser::{IntrospectionParser, InterfaceInfo};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Discovered D-Bus service with full introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub service_name: String,
    pub object_paths: Vec<String>,
    pub interfaces: HashMap<String, Vec<InterfaceInfo>>, // path -> interfaces
}

/// System introspection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIntrospection {
    pub services: Vec<DiscoveredService>,
    pub timestamp: i64,
    pub total_services: usize,
    pub total_interfaces: usize,
    pub total_methods: usize,
}

pub struct SystemIntrospector {
    hierarchical: HierarchicalIntrospector,
}

impl SystemIntrospector {
    /// Create new system introspector - uses HierarchicalIntrospector directly
    pub async fn new() -> Result<Self> {
        let cache_dir = PathBuf::from("/var/lib/op-dbus/@cache");
        let hierarchical = HierarchicalIntrospector::new(cache_dir)
            .await
            .context("Failed to create hierarchical introspector")?;

        Ok(Self { hierarchical })
    }

    /// List all D-Bus service names - calls hierarchical introspection
    pub async fn list_all_services(&self) -> Result<Vec<String>> {
        // Use cached introspection if available
        let introspection = match self.hierarchical.load_latest().await {
            Ok(cached) => cached,
            Err(_) => {
                // No cache, perform fresh introspection
                self.hierarchical.introspect_all().await?
            }
        };

        let services: Vec<String> = introspection
            .system_bus
            .services
            .keys()
            .map(|s| s.clone())
            .collect();

        Ok(services)
    }

    /// Introspect a specific service - converts from hierarchical format
    pub async fn introspect_service(&self, service_name: &str) -> Result<DiscoveredService> {
        log::info!("Introspecting service: {} (using hierarchical cache)", service_name);

        // Get hierarchical introspection data
        let introspection = match self.hierarchical.load_latest().await {
            Ok(cached) => cached,
            Err(_) => self.hierarchical.introspect_all().await?,
        };

        // Find the service in hierarchical data
        let service_data = introspection
            .system_bus
            .services
            .get(service_name)
            .ok_or_else(|| anyhow::anyhow!("Service {} not found", service_name))?;

        // Convert hierarchical format to DiscoveredService format
        let object_paths: Vec<String> = service_data.objects.keys().map(|s| s.clone()).collect();

        let mut interfaces: HashMap<String, Vec<InterfaceInfo>> = HashMap::new();

        for (path, obj) in &service_data.objects {
            let mut iface_list = Vec::new();

            for iface in &obj.interfaces {
                // Convert hierarchical interface to InterfaceInfo
                let methods = iface
                    .methods
                    .iter()
                    .map(|m| crate::mcp::introspection_parser::MethodInfo {
                        name: m.name.clone(),
                        inputs: m
                            .inputs
                            .iter()
                            .map(|arg| crate::mcp::introspection_parser::ArgInfo {
                                name: arg.name.clone().unwrap_or_default(),
                                type_sig: arg.type_.clone(),
                                type_name: dbus_type_to_name(&arg.type_),
                            })
                            .collect(),
                        outputs: m
                            .outputs
                            .iter()
                            .map(|arg| crate::mcp::introspection_parser::ArgInfo {
                                name: arg.name.clone().unwrap_or_default(),
                                type_sig: arg.type_.clone(),
                                type_name: dbus_type_to_name(&arg.type_),
                            })
                            .collect(),
                    })
                    .collect();

                let properties = iface
                    .properties
                    .iter()
                    .map(|p| crate::mcp::introspection_parser::PropertyInfo {
                        name: p.name.clone(),
                        type_sig: p.type_.clone(),
                        access: p.access.clone(),
                    })
                    .collect();

                let signals = iface
                    .signals
                    .iter()
                    .map(|s| crate::mcp::introspection_parser::SignalInfo {
                        name: s.name.clone(),
                        args: s
                            .args
                            .iter()
                            .map(|arg| crate::mcp::introspection_parser::ArgInfo {
                                name: arg.name.clone().unwrap_or_default(),
                                type_sig: arg.type_.clone(),
                                type_name: dbus_type_to_name(&arg.type_),
                            })
                            .collect(),
                    })
                    .collect();

                iface_list.push(InterfaceInfo {
                    name: iface.name.clone(),
                    methods,
                    properties,
                    signals,
                });
            }

            if !iface_list.is_empty() {
                interfaces.insert(path.clone(), iface_list);
            }
        }

        Ok(DiscoveredService {
            service_name: service_name.to_string(),
            object_paths,
            interfaces,
        })
    }

    /// Introspect all services - uses hierarchical introspection directly
    pub async fn introspect_all_services(&self) -> Result<SystemIntrospection> {
        log::info!("Introspecting all services (using hierarchical introspection)");

        // Perform fresh comprehensive introspection
        let introspection = self.hierarchical.introspect_all().await?;

        let mut services = Vec::new();

        // Convert all services from hierarchical format
        for service_name in introspection.system_bus.services.keys() {
            match self.introspect_service(service_name).await {
                Ok(service) => services.push(service),
                Err(e) => log::warn!("Failed to convert service {}: {}", service_name, e),
            }
        }

        Ok(SystemIntrospection {
            total_services: introspection.summary.total_services,
            total_interfaces: introspection.summary.total_interfaces,
            total_methods: introspection.summary.total_methods,
            services,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Generate MCP tool schema for a service
    pub fn generate_mcp_tools(&self, service: &DiscoveredService) -> Vec<serde_json::Value> {
        use serde_json::json;

        let mut tools = Vec::new();

        for (path, interfaces) in &service.interfaces {
            for interface in interfaces {
                for method in &interface.methods {
                    // Generate MCP tool for each method
                    let tool_name = format!(
                        "{}__{}",
                        service.service_name.replace('.', "_"),
                        method.name.to_lowercase()
                    );

                    let mut properties = serde_json::Map::new();
                    let mut required = Vec::new();

                    // Add method inputs as tool parameters
                    for input in &method.inputs {
                        let schema = IntrospectionParser::dbus_type_to_mcp_schema(&input.type_sig);
                        properties.insert(input.name.clone(), schema);
                        required.push(input.name.clone());
                    }

                    let tool = json!({
                        "name": tool_name,
                        "description": format!("{}.{} on {} ({})",
                            interface.name,
                            method.name,
                            service.service_name,
                            path
                        ),
                        "inputSchema": {
                            "type": "object",
                            "properties": properties,
                            "required": required
                        },
                        "metadata": {
                            "service": service.service_name,
                            "interface": interface.name,
                            "path": path,
                            "method": method.name
                        }
                    });

                    tools.push(tool);
                }
            }
        }

        tools
    }
}

/// CLI tool for system introspection
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        println!("Usage:");
        println!("  {} list                   - List all D-Bus services", args[0]);
        println!("  {} introspect <service>   - Introspect specific service", args[0]);
        println!("  {} introspect-all         - Introspect all services", args[0]);
        println!("  {} mcp-tools <service>    - Generate MCP tools for service", args[0]);
        println!("  {} mcp-tools-all          - Generate MCP tools for all services", args[0]);
        return Ok(());
    }

    let introspector = SystemIntrospector::new().await?;

    match args[1].as_str() {
        "list" => {
            let services = introspector.list_all_services().await?;
            println!("Found {} D-Bus services:\n", services.len());
            for service in services {
                println!("  {}", service);
            }
        }
        "introspect" => {
            if args.len() < 3 {
                eprintln!("Error: Service name required");
                return Ok(());
            }

            let service_name = &args[2];
            let service = introspector.introspect_service(service_name).await?;

            println!("Service: {}", service.service_name);
            println!("Object paths: {}", service.object_paths.len());
            println!("\nInterfaces:");

            for (path, interfaces) in &service.interfaces {
                println!("\n  Path: {}", path);
                for interface in interfaces {
                    println!("    Interface: {}", interface.name);
                    println!("      Methods: {}", interface.methods.len());
                    for method in &interface.methods {
                        println!("        - {}", method.name);
                    }
                    println!("      Properties: {}", interface.properties.len());
                    println!("      Signals: {}", interface.signals.len());
                }
            }

            // Output JSON
            let json = serde_json::to_string_pretty(&service)?;
            println!("\nJSON:\n{}", json);
        }
        "introspect-all" => {
            let result = introspector.introspect_all_services().await?;

            println!("System Introspection Results:");
            println!("  Total services: {}", result.total_services);
            println!("  Total interfaces: {}", result.total_interfaces);
            println!("  Total methods: {}", result.total_methods);

            let json = serde_json::to_string_pretty(&result)?;
            std::fs::write("/tmp/system-introspection.json", &json)?;
            println!("\nFull results written to: /tmp/system-introspection.json");
        }
        "mcp-tools" => {
            if args.len() < 3 {
                eprintln!("Error: Service name required");
                return Ok(());
            }

            let service_name = &args[2];
            let service = introspector.introspect_service(service_name).await?;
            let tools = introspector.generate_mcp_tools(&service);

            println!("Generated {} MCP tools for {}\n", tools.len(), service_name);

            let json = serde_json::to_string_pretty(&tools)?;
            println!("{}", json);
        }
        "mcp-tools-all" => {
            let result = introspector.introspect_all_services().await?;

            let mut all_tools = Vec::new();
            for service in &result.services {
                let tools = introspector.generate_mcp_tools(service);
                all_tools.extend(tools);
            }

            println!("Generated {} total MCP tools\n", all_tools.len());

            let json = serde_json::to_string_pretty(&all_tools)?;
            std::fs::write("/tmp/mcp-tools-all.json", &json)?;
            println!("MCP tools written to: /tmp/mcp-tools-all.json");
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
        }
    }

    Ok(())
}

/// Convert D-Bus type signatures to friendly names
fn dbus_type_to_name(type_sig: &str) -> String {
    match type_sig {
        "s" => "string",
        "i" => "int32",
        "u" => "uint32",
        "x" => "int64",
        "t" => "uint64",
        "n" => "int16",
        "q" => "uint16",
        "y" => "byte",
        "b" => "boolean",
        "d" => "double",
        "o" => "object_path",
        "g" => "signature",
        "h" => "unix_fd",
        "a" => "array",
        "(" => "struct",
        "{" => "dict",
        "v" => "variant",
        _ => type_sig, // Return as-is if unknown
    }.to_string()
}
