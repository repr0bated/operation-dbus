//! MCP Resources and Agent loading implementation
//!
//! This module provides MCP resource functionality and loads embedded
//! markdown files as executable agents/tools from the agents and commands repositories.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// Include the auto-generated embedded markdown content
include!(concat!(env!("OUT_DIR"), "/embedded_markdown.rs"));

/// Agent specification parsed from markdown frontmatter
#[derive(Debug, Clone, Deserialize)]
pub struct AgentSpec {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl AgentSpec {
    /// Parse agent spec from markdown content
    pub fn from_markdown(content: &str) -> Result<Self> {
        // Check if content starts with YAML frontmatter
        if !content.trim_start().starts_with("---") {
            return Ok(Self {
                name: None,
                description: None,
                model: None,
                extra: HashMap::new(),
            });
        }

        // Extract frontmatter (between --- markers)
        let content_str = content.trim_start();
        let frontmatter_end = content_str[3..].find("---").map(|pos| pos + 3);

        if let Some(end_pos) = frontmatter_end {
            let yaml_content = &content_str[3..end_pos];
            let spec: AgentSpec = serde_yaml::from_str(yaml_content)?;
            Ok(spec)
        } else {
            Ok(Self {
                name: None,
                description: None,
                model: None,
                extra: HashMap::new(),
            })
        }
    }
}

/// Resource trait that all MCP resources must implement
#[async_trait]
pub trait Resource: Send + Sync {
    /// Get the resource URI
    fn uri(&self) -> &str;

    /// Get resource name for display
    fn name(&self) -> &str;

    /// Get resource description
    fn description(&self) -> &str;

    /// Get MIME type
    fn mime_type(&self) -> &str;

    /// Read the resource content
    async fn read(&self) -> Result<ResourceContent>;

    /// Get resource metadata
    fn metadata(&self) -> ResourceMetadata {
        ResourceMetadata {
            name: self.name().to_string(),
            description: self.description().to_string(),
            mime_type: self.mime_type().to_string(),
            size: None,
            last_modified: None,
        }
    }
}

/// Content returned from reading a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub contents: Vec<ResourceContentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContentItem {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
    pub blob: Option<String>, // base64 encoded binary data
}

impl ResourceContent {
    pub fn text(uri: String, mime_type: String, text: String) -> Self {
        Self {
            contents: vec![ResourceContentItem {
                uri,
                mime_type,
                text: Some(text),
                blob: None,
            }],
        }
    }
}

/// Resource metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub size: Option<u64>,
    pub last_modified: Option<String>,
}

/// Embedded markdown file resource implementation
pub struct MarkdownFileResource {
    uri: String,
    name: String,
    description: String,
}

impl MarkdownFileResource {
    pub fn new(uri: String, content: &str) -> Self {
        let name = uri.clone();

        // Extract description from first line
        let description = content
            .lines()
            .next()
            .filter(|line| line.starts_with('#'))
            .map(|line| line.trim_start_matches('#').trim().to_string())
            .unwrap_or_else(|| format!("Markdown file: {}", name));

        Self {
            uri,
            name,
            description,
        }
    }
}

#[async_trait]
impl Resource for MarkdownFileResource {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn mime_type(&self) -> &str {
        "text/markdown"
    }

    async fn read(&self) -> Result<ResourceContent> {
        let content = get_embedded_markdown(&self.uri)
            .ok_or_else(|| anyhow::anyhow!("Embedded content not found for URI: {}", self.uri))?;

        Ok(ResourceContent::text(
            self.uri.clone(),
            self.mime_type().to_string(),
            content.to_string(),
        ))
    }
}

/// Resource registry for managing MCP resources
#[derive(Clone)]
pub struct ResourceRegistry {
    resources: Arc<RwLock<HashMap<String, Box<dyn Resource>>>>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a resource
    pub async fn register_resource(&self, resource: Box<dyn Resource>) -> Result<()> {
        let uri = resource.uri().to_string();
        let mut resources = self.resources.write().await;
        resources.insert(uri, resource);
        Ok(())
    }

    /// Check if a resource exists by URI
    pub async fn has_resource(&self, uri: &str) -> bool {
        let resources = self.resources.read().await;
        resources.contains_key(uri)
    }

    /// List all resources
    pub async fn list_resources(&self) -> Vec<Value> {
        let resources = self.resources.read().await;
        resources.values().map(|resource| {
            json!({
                "uri": resource.uri(),
                "name": resource.name(),
                "description": resource.description(),
                "mimeType": resource.mime_type()
            })
        }).collect()
    }

    /// Read a resource by URI
    pub async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        if !self.has_resource(uri).await {
            anyhow::bail!("Resource not found: {}", uri);
        }

        // Get content directly from embedded data
        let content = get_embedded_markdown(uri)
            .ok_or_else(|| anyhow::anyhow!("Embedded content not found for URI: {}", uri))?;

        Ok(ResourceContent::text(
            uri.to_string(),
            "text/markdown".to_string(),
            content.to_string(),
        ))
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Register embedded markdown URIs in the resource registry
pub async fn register_embedded_markdown_resources(registry: &ResourceRegistry) -> Result<()> {
    let uris = get_embedded_markdown_uris();
    eprintln!("Registering {} embedded markdown resources", uris.len());

    for uri in uris {
        if let Some(content) = get_embedded_markdown(uri) {
            let resource = MarkdownFileResource::new(uri.to_string(), content);
            registry.register_resource(Box::new(resource)).await?;
        } else {
            eprintln!("Warning: No content found for URI: {}", uri);
        }
    }

    Ok(())
}

/// Load embedded agents as MCP tools
pub fn load_embedded_agents() -> Result<Vec<EmbeddedAgent>> {
    let uris = get_embedded_markdown_uris();
    let mut agents = Vec::new();

    eprintln!("Loading {} embedded agents/tools", uris.len());

    for uri in uris {
        if let Some(content) = get_embedded_markdown(uri) {
            match AgentSpec::from_markdown(content) {
                Ok(spec) => {
                    // Only load as agent if it has a name (indicating it's an agent spec)
                    if spec.name.is_some() {
                        let agent = EmbeddedAgent {
                            uri: uri.to_string(),
                            spec,
                            content: content.to_string(),
                        };
                        if let Some(name) = &agent.spec.name {
                            eprintln!("Loaded agent: {} ({})", name, uri);
                        }
                        agents.push(agent);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse agent spec for {}: {}", uri, e);
                }
            }
        }
    }

    eprintln!("Successfully loaded {} agents", agents.len());
    Ok(agents)
}

/// Embedded agent with parsed specification
#[derive(Debug, Clone)]
pub struct EmbeddedAgent {
    pub uri: String,
    pub spec: AgentSpec,
    pub content: String,
}