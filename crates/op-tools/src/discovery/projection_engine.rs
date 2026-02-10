//! Projection Engine - Auto-Discovery of D-Bus APIs as tools
//!
//! This engine walks the D-Bus object tree and projects discovered
//! interfaces as executable tools in the registry.

use anyhow::Result;
use futures::{stream::iter, StreamExt};
use simd_json::prelude::*;
use simd_json::OwnedValue as Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::registry::ToolDefinition;
use crate::tool::Tool;
use op_core::BusType;
use op_introspection::IntrospectionService;

/// Projection Engine - auto-discovers D-Bus APIs
pub struct ProjectionEngine {
    introspection: Arc<IntrospectionService>,
}

impl ProjectionEngine {
    pub fn new(introspection: Arc<IntrospectionService>) -> Self {
        Self { introspection }
    }

    /// Discover and register all tools for a bus
    pub async fn discover_all(
        &self,
        registry: &crate::registry::ToolRegistry,
        bus_type: BusType,
    ) -> Result<usize> {
        let services_json = self.introspection.list_services_json(bus_type).await?;
        let mut total_count = 0;

        let services: Vec<String> = if let Some(arr) = services_json.as_array() {
            arr.iter()
                .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
                .filter(|n| !n.starts_with(':')) // Skip unique names (temporary connections)
                .filter(|n| !n.starts_with("org.dbusmcp.")) // Skip our own services
                .collect()
        } else {
            Vec::new()
        };

        tracing::info!(
            "Discovering tools for {} services on {:?} bus",
            services.len(),
            bus_type
        );

        // Process each service
        for service in services {
            tracing::debug!(
                "Introspecting service '{}' on {:?} bus...",
                service,
                bus_type
            );

            // Discover all object paths for this service
            let paths = self.discover_paths(bus_type, &service, "/", 0).await;
            let mut service_tools = 0;

            // Process each object path
            for path in &paths {
                if let Ok(info) = self
                    .introspection
                    .introspect(bus_type, &service, &path)
                    .await
                {
                    for iface in info.interfaces {
                        // Skip standard D-Bus interfaces unless they are interesting
                        if iface.name.starts_with("org.freedesktop.DBus.")
                            && !iface.name.contains("Properties")
                            && !iface.name.contains("ObjectManager")
                        {
                            continue;
                        }

                        for method in iface.methods {
                            let tool = crate::dynamic_tool::DynamicDbusTool::new(
                                service.clone(),
                                path.clone(),
                                iface.name.clone(),
                                method.name.clone(),
                                String::new(), // Signature not easily available here yet
                                method
                                    .in_args
                                    .iter()
                                    .map(|a| a.name.clone().unwrap_or_else(|| "arg".to_string()))
                                    .collect(),
                            );

                            let definition = crate::registry::ToolDefinition {
                                name: tool.name.clone(),
                                description: format!(
                                    "D-Bus method {}.{} on {} at {}",
                                    iface.name, method.name, service, path
                                ),
                                input_schema: tool.input_schema(),
                                schema_version: "https://json-schema.org/draft/next/schema"
                                    .to_string(),
                                category: "dbus-projected".to_string(),
                                namespace: "system.v1".to_string(),
                                tags: vec![
                                    "dbus".to_string(),
                                    "projected".to_string(),
                                    service.clone(),
                                ],
                            };

                            if let Ok(_) = registry
                                .register(tool.name.clone().into(), Arc::new(tool), definition)
                                .await
                            {
                                service_tools += 1;
                            }
                        }
                    }
                }
            }

            total_count += service_tools;
            if service_tools > 0 {
                tracing::info!(
                    "  → Service {}: registered {} tools from {} paths",
                    service,
                    service_tools,
                    paths.len()
                );
            }
        }

        Ok(total_count)
    }

    /// Recursively discover all object paths for a service
    fn discover_paths<'a>(
        &'a self,
        bus_type: BusType,
        service: &'a str,
        path: &'a str,
        depth: usize,
    ) -> Pin<Box<dyn Future<Output = Vec<String>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_DEPTH: usize = 10;
            if depth > MAX_DEPTH {
                return vec![];
            }

            let mut paths = vec![path.to_string()];

            // Introspect to find child nodes
            if let Ok(info) = self.introspection.introspect(bus_type, service, path).await {
                for child in &info.children {
                    if child.is_empty() {
                        continue;
                    }

                    let child_path = if path == "/" {
                        format!("/{}", child)
                    } else {
                        format!("{}/{}", path, child)
                    };

                    // Recursively discover child paths
                    let child_paths = self
                        .discover_paths(bus_type, service, &child_path, depth + 1)
                        .await;
                    paths.extend(child_paths);
                }
            }

            paths
        })
    }
}

impl Clone for ProjectionEngine {
    fn clone(&self) -> Self {
        Self {
            introspection: self.introspection.clone(),
        }
    }
}
