//! State management - declarative plugin system
pub mod dbus_plugin_base;
pub mod manager;
pub mod plugin;
pub mod plugins;
pub mod plugtree;

pub use manager::StateManager;
