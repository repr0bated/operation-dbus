//! State management - declarative plugin system
pub mod auto_plugin;
pub mod crypto;
pub mod dbus_plugin_base;
pub mod dbus_server;
pub mod manager;
pub mod plugin;
pub mod plugins;
pub mod plugtree;

pub use manager::StateManager;
