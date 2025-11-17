//! Embedded MCP resources - documentation and agent definitions
//!
//! This module embeds markdown documentation files directly into the MCP server binary,
//! making them available via the MCP resources protocol without requiring external files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub content: String,
}

/// Registry of embedded resources
pub struct ResourceRegistry {
    resources: HashMap<String, Resource>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        let mut resources = HashMap::new();

        // Embed agent documentation
        resources.insert(
            "agent://agents/overview".to_string(),
            Resource {
                uri: "agent://agents/overview".to_string(),
                name: "Agent System Overview".to_string(),
                description: "Complete overview of the agent-based architecture and guidelines"
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../AGENTS.md").to_string(),
            },
        );

        // Embed MCP documentation
        resources.insert(
            "mcp://docs/complete-guide".to_string(),
            Resource {
                uri: "mcp://docs/complete-guide".to_string(),
                name: "MCP Complete Guide".to_string(),
                description: "Complete guide to the Model Context Protocol integration".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../docs/MCP-COMPLETE-GUIDE.md").to_string(),
            },
        );

        resources.insert(
            "mcp://docs/developer-guide".to_string(),
            Resource {
                uri: "mcp://docs/developer-guide".to_string(),
                name: "MCP Developer Guide".to_string(),
                description: "Developer guide for extending the MCP server".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../docs/MCP-DEVELOPER-GUIDE.md").to_string(),
            },
        );

        resources.insert(
            "mcp://docs/api-reference".to_string(),
            Resource {
                uri: "mcp://docs/api-reference".to_string(),
                name: "MCP API Reference".to_string(),
                description: "Complete API reference for MCP tools and resources".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../docs/MCP-API-REFERENCE.md").to_string(),
            },
        );

        resources.insert(
            "mcp://docs/chat-console".to_string(),
            Resource {
                uri: "mcp://docs/chat-console".to_string(),
                name: "MCP Chat Console Guide".to_string(),
                description: "Guide to using the interactive MCP chat console".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../docs/MCP-CHAT-CONSOLE.md").to_string(),
            },
        );

        // Embed hierarchical D-Bus design
        resources.insert(
            "dbus://design/hierarchical".to_string(),
            Resource {
                uri: "dbus://design/hierarchical".to_string(),
                name: "Hierarchical D-Bus Design".to_string(),
                description: "Design document for the hierarchical D-Bus abstraction layer"
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../HIERARCHICAL_DBUS_DESIGN.md").to_string(),
            },
        );

        resources.insert(
            "dbus://guide/introspection".to_string(),
            Resource {
                uri: "dbus://guide/introspection".to_string(),
                name: "D-Bus Introspection with zbus".to_string(),
                description: "Comprehensive guide to D-Bus introspection using Rust zbus".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../d_bus_introspection_with_zbus.md").to_string(),
            },
        );

        resources.insert(
            "dbus://guide/indexer-implementation".to_string(),
            Resource {
                uri: "dbus://guide/indexer-implementation".to_string(),
                name: "D-Bus Indexer Implementation Guide".to_string(),
                description: "Implementation guide for the D-Bus indexer based on zbus patterns"
                    .to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../DBUS_INDEXER_IMPLEMENTATION_GUIDE.md").to_string(),
            },
        );

        // Embed snapshot automation
        resources.insert(
            "snapshot://automation".to_string(),
            Resource {
                uri: "snapshot://automation".to_string(),
                name: "Snapshot Automation Guide".to_string(),
                description: "Guide to BTRFS snapshot automation for D-Bus index".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../SNAPSHOT_AUTOMATION.md").to_string(),
            },
        );

        // Embed plugin development guide
        resources.insert(
            "plugin://development-guide".to_string(),
            Resource {
                uri: "plugin://development-guide".to_string(),
                name: "Plugin Development Guide".to_string(),
                description: "Complete guide to developing plugins for op-dbus".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../PLUGIN-DEVELOPMENT-GUIDE.md").to_string(),
            },
        );

        // Embed architecture documentation
        resources.insert(
            "architecture://correct".to_string(),
            Resource {
                uri: "architecture://correct".to_string(),
                name: "Correct Architecture".to_string(),
                description: "The correct architecture for op-dbus system".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../docs/CORRECT-ARCHITECTURE.md").to_string(),
            },
        );

        resources.insert(
            "architecture://final".to_string(),
            Resource {
                uri: "architecture://final".to_string(),
                name: "Final Architecture".to_string(),
                description: "Final architecture design for the distributed system".to_string(),
                mime_type: "text/markdown".to_string(),
                content: include_str!("../../docs/FINAL-ARCHITECTURE.md").to_string(),
            },
        );

        Self { resources }
    }

    /// List all available resources
    pub fn list_resources(&self) -> Vec<&Resource> {
        self.resources.values().collect()
    }

    /// Get a specific resource by URI
    pub fn get_resource(&self, uri: &str) -> Option<&Resource> {
        self.resources.get(uri)
    }

    /// Search resources by keyword
    pub fn search(&self, query: &str) -> Vec<&Resource> {
        let query_lower = query.to_lowercase();
        self.resources
            .values()
            .filter(|r| {
                r.name.to_lowercase().contains(&query_lower)
                    || r.description.to_lowercase().contains(&query_lower)
                    || r.content.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Get resources by category (extracted from URI scheme)
    pub fn get_by_category(&self, category: &str) -> Vec<&Resource> {
        let scheme = format!("{}://", category);
        self.resources
            .values()
            .filter(|r| r.uri.starts_with(&scheme))
            .collect()
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_registry() {
        let registry = ResourceRegistry::new();

        // Test listing
        let resources = registry.list_resources();
        assert!(!resources.is_empty(), "Should have embedded resources");

        // Test get by URI
        let agent_overview = registry.get_resource("agent://agents/overview");
        assert!(agent_overview.is_some(), "Should find agent overview");

        // Test search
        let mcp_resources = registry.search("MCP");
        assert!(!mcp_resources.is_empty(), "Should find MCP resources");

        // Test category
        let dbus_docs = registry.get_by_category("dbus");
        assert!(!dbus_docs.is_empty(), "Should find D-Bus documentation");
    }
}
