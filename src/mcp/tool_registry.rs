//! Tool registry for dynamic tool management in MCP
//!
//! Eliminates tight coupling by allowing tools to be registered
//! dynamically without hardcoding in the main MCP server.

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tool trait that all MCP tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get tool description
    fn description(&self) -> &str;

    /// Get the JSON schema for input validation
    fn input_schema(&self) -> Value;

    /// Execute the tool with given parameters
    async fn execute(&self, params: Value) -> Result<ToolResult>;

    /// Validate parameters before execution
    async fn validate(&self, params: &Value) -> Result<()> {
        // Default implementation - can be overridden
        Ok(())
    }

    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: self.name().to_string(),
            description: self.description().to_string(),
            category: "general".to_string(),
            tags: vec![],
            author: None,
            version: "1.0.0".to_string(),
        }
    }
}

/// Result from tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl ToolResult {
    /// Create a success result with a single content item
    pub fn success(content: ToolContent) -> Self {
        Self {
            content: vec![content],
            metadata: None,
        }
    }

    /// Create a success result with multiple content items
    pub fn success_multi(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            metadata: None,
        }
    }

    /// Create an error result
    pub fn error(message: &str) -> Self {
        Self {
            content: vec![ToolContent::error(message)],
            metadata: None,
        }
    }

    /// Add metadata to the result
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub data: Option<Value>,
}

impl ToolContent {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: Some(text.into()),
            data: None,
        }
    }

    pub fn json(data: Value) -> Self {
        Self {
            content_type: "json".to_string(),
            text: None,
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content_type: "error".to_string(),
            text: Some(message.into()),
            data: None,
        }
    }
}

/// Tool metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub version: String,
}

/// Tool factory for creating tool instances
#[async_trait]
pub trait ToolFactory: Send + Sync {
    /// Create a new tool instance
    async fn create_tool(&self) -> Result<Box<dyn Tool>>;

    /// Get the tool name this factory creates
    fn tool_name(&self) -> &str;
}

/// Tool registry for managing all tools
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<Box<dyn Tool>>>>>,
    factories: Arc<RwLock<HashMap<String, Box<dyn ToolFactory>>>>,
    categories: Arc<RwLock<HashMap<String, Vec<String>>>>,
    middleware: Arc<RwLock<Vec<Box<dyn ToolMiddleware>>>>,
}

/// Middleware for tool execution
#[async_trait]
pub trait ToolMiddleware: Send + Sync {
    /// Called before tool execution
    async fn before_execute(&self, tool_name: &str, params: &Value) -> Result<()>;

    /// Called after tool execution
    async fn after_execute(&self, tool_name: &str, params: &Value, result: &Result<ToolResult>);
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            factories: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            middleware: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a tool instance
    pub async fn register_tool(&self, tool: Box<dyn Tool>) -> Result<()> {
        let name = tool.name().to_string();
        let metadata = tool.metadata();

        let mut tools = self.tools.write().await;
        if tools.contains_key(&name) {
            bail!("Tool '{}' is already registered", name);
        }

        tools.insert(name.clone(), Arc::new(tool));

        // Add to category
        let mut categories = self.categories.write().await;
        categories
            .entry(metadata.category)
            .or_insert_with(Vec::new)
            .push(name);

        Ok(())
    }

    /// Register a tool factory
    pub async fn register_factory(&self, factory: Box<dyn ToolFactory>) -> Result<()> {
        let name = factory.tool_name().to_string();

        let mut factories = self.factories.write().await;
        if factories.contains_key(&name) {
            bail!("Tool factory '{}' is already registered", name);
        }

        factories.insert(name, factory);
        Ok(())
    }

    /// Register middleware
    pub async fn add_middleware(&self, middleware: Box<dyn ToolMiddleware>) {
        let mut middlewares = self.middleware.write().await;
        middlewares.push(middleware);
    }

    /// Get a tool by name
    pub async fn get_tool(&self, name: &str) -> Option<Arc<Box<dyn Tool>>> {
        let tools = self.tools.read().await;
        tools.get(name).cloned()
    }

    /// Execute a tool
    pub async fn execute_tool(&self, name: &str, params: Value) -> Result<ToolResult> {
        // Try to get existing tool
        let tool = if let Some(tool) = self.get_tool(name).await {
            tool
        } else {
            // Try to create from factory
            let factories = self.factories.read().await;
            if let Some(factory) = factories.get(name) {
                let new_tool = factory.create_tool().await?;
                drop(factories);

                // Register the new tool
                self.register_tool(new_tool).await?;
                self.get_tool(name)
                    .await
                    .ok_or_else(|| anyhow::anyhow!("Failed to register tool"))?
            } else {
                bail!("Tool '{}' not found", name);
            }
        };

        // Call before middleware
        let middlewares = self.middleware.read().await;
        for mw in middlewares.iter() {
            mw.before_execute(name, &params).await?;
        }

        // Validate parameters
        tool.validate(&params).await?;

        // Execute tool
        let result = tool.execute(params.clone()).await;

        // Call after middleware
        for mw in middlewares.iter() {
            mw.after_execute(name, &params, &result).await;
        }

        result
    }

    /// List all registered tools
    pub async fn list_tools(&self) -> Vec<ToolInfo> {
        let tools = self.tools.read().await;
        tools
            .values()
            .map(|tool| ToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.input_schema(),
                metadata: tool.metadata(),
            })
            .collect()
    }

    /// List tools by category
    pub async fn list_tools_by_category(&self, category: &str) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.get(category).cloned().unwrap_or_default()
    }

    /// Get all categories
    pub async fn list_categories(&self) -> Vec<String> {
        let categories = self.categories.read().await;
        categories.keys().cloned().collect()
    }
}

/// Tool information for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    pub metadata: ToolMetadata,
}

/// Example logging middleware
pub struct LoggingMiddleware;

#[async_trait]
impl ToolMiddleware for LoggingMiddleware {
    async fn before_execute(&self, tool_name: &str, params: &Value) -> Result<()> {
        log::info!("Executing tool '{}' with params: {:?}", tool_name, params);
        Ok(())
    }

    async fn after_execute(&self, tool_name: &str, _params: &Value, result: &Result<ToolResult>) {
        match result {
            Ok(_) => log::info!("Tool '{}' executed successfully", tool_name),
            Err(e) => log::error!("Tool '{}' failed: {}", tool_name, e),
        }
    }
}

/// Example audit middleware
pub struct AuditMiddleware {
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_name: String,
    pub params: Value,
    pub success: bool,
    pub error: Option<String>,
}

impl AuditMiddleware {
    pub fn new() -> Self {
        Self {
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_audit_log(&self) -> Vec<AuditEntry> {
        let log = self.audit_log.read().await;
        log.clone()
    }
}

#[async_trait]
impl ToolMiddleware for AuditMiddleware {
    async fn before_execute(&self, _tool_name: &str, _params: &Value) -> Result<()> {
        Ok(())
    }

    async fn after_execute(&self, tool_name: &str, params: &Value, result: &Result<ToolResult>) {
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            tool_name: tool_name.to_string(),
            params: params.clone(),
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
        };

        let mut log = self.audit_log.write().await;
        log.push(entry);

        // Keep only last 1000 entries
        if log.len() > 1000 {
            let drain_to = log.len().saturating_sub(1000);
            log.drain(0..drain_to);
        }
    }
}

/// Helper macro to implement tools
#[macro_export]
macro_rules! impl_tool {
    ($name:ident, $tool_name:expr, $description:expr, $schema:expr) => {
        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> &str {
                $description
            }

            fn input_schema(&self) -> Value {
                $schema
            }

            async fn execute(&self, params: Value) -> Result<ToolResult> {
                self.execute_impl(params).await
            }
        }
    };
}

/// Example tool implementation
pub struct SystemdStatusTool;

impl SystemdStatusTool {
    async fn execute_impl(&self, params: Value) -> Result<ToolResult> {
        let service = params["service"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'service' parameter"))?;

        // In real implementation, would query systemd
        let status = format!("Service '{}' is running", service);

        Ok(ToolResult {
            content: vec![ToolContent::text(status)],
            metadata: None,
        })
    }
}

impl_tool!(
    SystemdStatusTool,
    "systemd_status",
    "Get the status of a systemd service",
    json!({
        "type": "object",
        "properties": {
            "service": {
                "type": "string",
                "description": "Name of the systemd service"
            }
        },
        "required": ["service"]
    })
);

/// Dynamic tool builder for runtime tool creation
use std::pin::Pin;
use std::future::Future;

pub struct DynamicToolBuilder {
    name: String,
    description: String,
    schema: Value,
    handler: Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<ToolResult>> + Send>> + Send + Sync>,
}

impl DynamicToolBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            schema: json!({}),
            handler: Arc::new(|_| {
                Box::pin(async {
                    Ok(ToolResult {
                        content: vec![ToolContent::text("Not implemented")],
                        metadata: None,
                    })
                })
            }),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn schema(mut self, schema: Value) -> Self {
        self.schema = schema;
        self
    }

    pub fn handler<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolResult>> + Send + 'static,
    {
        self.handler = Arc::new(move |params| Box::pin(handler(params)));
        self
    }

    pub fn build(self) -> DynamicTool {
        DynamicTool {
            name: self.name,
            description: self.description,
            schema: self.schema,
            handler: self.handler,
        }
    }
}

pub struct DynamicTool {
    name: String,
    description: String,
    schema: Value,
    handler: Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Result<ToolResult>> + Send>> + Send + Sync>,
}

#[async_trait]
impl Tool for DynamicTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, params: Value) -> Result<ToolResult> {
        (self.handler)(params).await
    }
}
