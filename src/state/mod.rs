//! State management - declarative plugin system
#[cfg(feature = "mcp")]
pub mod auto_plugin;
pub mod crypto;
pub mod dbus_plugin_base;
pub mod dbus_server;
pub mod manager;
pub mod plugin;
pub mod plugin_workflow;
pub mod plugins;
pub mod plugtree;

pub use manager::StateManager;
pub use plugin_workflow::{PluginWorkflowManager, WorkflowPluginNode};
