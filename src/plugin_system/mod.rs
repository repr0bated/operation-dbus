//! Modular plugin system for loose coupling
//!
//! This module provides a trait-based plugin architecture that allows
//! plugins to be added, removed, and modified without changing core code.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique name for this plugin
    fn name(&self) -> &str;

    /// Description of what this plugin does
    fn description(&self) -> &str;

    /// Version of the plugin
    fn version(&self) -> &str;

    /// Get the current state managed by this plugin
    async fn get_state(&self) -> Result<Value>;

    /// Apply a desired state
    async fn apply_state(&self, desired: Value) -> Result<()>;

    /// Calculate diff between current and desired state
    async fn diff(&self, current: Value, desired: Value) -> Result<Vec<Change>>;

    /// Validate a configuration before applying
    async fn validate(&self, config: Value) -> Result<ValidationResult>;

    /// Get plugin capabilities
    fn capabilities(&self) -> PluginCapabilities;

    /// Handle plugin-specific commands
    async fn handle_command(&self, command: &str, _args: Value) -> Result<Value> {
        Err(anyhow::anyhow!(
            "Command '{}' not supported by plugin '{}'",
            command,
            self.name()
        ))
    }

    /// Initialize the plugin
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// Cleanup when plugin is being removed
    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    /// Get plugin metadata
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name().to_string(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            author: None,
            license: None,
            dependencies: vec![],
        }
    }

    /// Convert to Any for downcasting if needed
    fn as_any(&self) -> &dyn Any;
}

/// Represents a change to be applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub operation: ChangeOperation,
    pub path: String,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOperation {
    Create,
    Update,
    Delete,
    NoOp,
}

/// Validation result from a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
            suggestions: vec![],
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![error.into()],
            warnings: vec![],
            suggestions: vec![],
        }
    }
}

/// Plugin capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCapabilities {
    pub can_read: bool,
    pub can_write: bool,
    pub can_delete: bool,
    pub supports_dry_run: bool,
    pub supports_rollback: bool,
    pub supports_transactions: bool,
    pub requires_root: bool,
    pub supported_platforms: Vec<String>,
}

impl Default for PluginCapabilities {
    fn default() -> Self {
        Self {
            can_read: true,
            can_write: true,
            can_delete: false,
            supports_dry_run: true,
            supports_rollback: false,
            supports_transactions: false,
            requires_root: false,
            supported_platforms: vec!["linux".to_string()],
        }
    }
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub dependencies: Vec<String>,
}

/// Plugin registry for managing all plugins
type PluginMap = HashMap<String, Arc<Box<dyn Plugin>>>;

pub struct PluginRegistry {
    plugins: Arc<RwLock<PluginMap>>,
    hooks: Arc<RwLock<PluginHooks>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            hooks: Arc::new(RwLock::new(PluginHooks::default())),
        }
    }

    /// Register a new plugin
    pub async fn register(&self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.name().to_string();

        // Call pre-registration hook
        self.call_hook(PluginEvent::PreRegister { name: name.clone() })
            .await?;

        // Initialize the plugin
        let mut plugin = plugin;
        plugin
            .initialize()
            .await
            .context("Failed to initialize plugin")?;

        // Store the plugin
        let mut plugins = self.plugins.write().await;
        if plugins.contains_key(&name) {
            return Err(anyhow::anyhow!("Plugin '{}' is already registered", name));
        }

        plugins.insert(name.clone(), Arc::new(plugin));

        // Call post-registration hook
        self.call_hook(PluginEvent::PostRegister { name: name.clone() })
            .await?;

        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister(&self, name: &str) -> Result<()> {
        // Call pre-unregister hook
        self.call_hook(PluginEvent::PreUnregister {
            name: name.to_string(),
        })
        .await?;

        let mut plugins = self.plugins.write().await;

        if let Some(plugin) = plugins.get_mut(name) {
            // Get mutable access to cleanup
            if let Some(plugin) = Arc::get_mut(plugin) {
                plugin.cleanup().await.context("Failed to cleanup plugin")?;
            }
        }

        plugins
            .remove(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;

        // Call post-unregister hook
        self.call_hook(PluginEvent::PostUnregister {
            name: name.to_string(),
        })
        .await?;

        Ok(())
    }

    /// Get a plugin by name
    pub async fn get(&self, name: &str) -> Option<Arc<Box<dyn Plugin>>> {
        let plugins = self.plugins.read().await;
        plugins.get(name).cloned()
    }

    /// List all registered plugins
    pub async fn list(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }

    /// Get metadata for all plugins
    pub async fn get_all_metadata(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata()).collect()
    }

    /// Register a hook
    pub async fn register_hook(&self, event: PluginEventType, handler: PluginHookHandler) {
        let mut hooks = self.hooks.write().await;
        hooks.register(event, handler);
    }

    /// Call hooks for an event
    async fn call_hook(&self, event: PluginEvent) -> Result<()> {
        let hooks = self.hooks.read().await;
        hooks.call(event).await
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin lifecycle events
#[derive(Debug, Clone)]
pub enum PluginEvent {
    PreRegister { name: String },
    PostRegister { name: String },
    PreUnregister { name: String },
    PostUnregister { name: String },
    StateChanged { plugin: String, state: Value },
    Error { plugin: String, error: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluginEventType {
    PreRegister,
    PostRegister,
    PreUnregister,
    PostUnregister,
    StateChanged,
    Error,
}

/// Hook handler function
pub type PluginHookHandler = Arc<dyn Fn(PluginEvent) -> Result<()> + Send + Sync>;

/// Plugin hooks for lifecycle events
#[derive(Default)]
struct PluginHooks {
    handlers: HashMap<PluginEventType, Vec<PluginHookHandler>>,
}

impl PluginHooks {
    fn register(&mut self, event: PluginEventType, handler: PluginHookHandler) {
        self.handlers.entry(event).or_default().push(handler);
    }

    async fn call(&self, event: PluginEvent) -> Result<()> {
        let event_type = match &event {
            PluginEvent::PreRegister { .. } => PluginEventType::PreRegister,
            PluginEvent::PostRegister { .. } => PluginEventType::PostRegister,
            PluginEvent::PreUnregister { .. } => PluginEventType::PreUnregister,
            PluginEvent::PostUnregister { .. } => PluginEventType::PostUnregister,
            PluginEvent::StateChanged { .. } => PluginEventType::StateChanged,
            PluginEvent::Error { .. } => PluginEventType::Error,
        };

        if let Some(handlers) = self.handlers.get(&event_type) {
            for handler in handlers {
                handler(event.clone())?;
            }
        }

        Ok(())
    }
}

/// Helper macro to implement Plugin trait
#[macro_export]
macro_rules! impl_plugin {
    ($name:ident, $version:literal, $description:literal) => {
        impl Plugin for $name {
            fn name(&self) -> &str {
                stringify!($name)
            }

            fn version(&self) -> &str {
                $version
            }

            fn description(&self) -> &str {
                $description
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}
