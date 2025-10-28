//! Operation D-Bus - Declarative system state management via native protocols
//!
//! This crate provides declarative system state management through native Linux protocols.

// Core modules
pub mod state;
pub mod native;
pub mod cache;
pub mod blockchain;
pub mod nonnet_db;

// Loose coupling modules
pub mod plugin_system;
pub mod event_bus;

// Optional modules
#[cfg(feature = "web")]
pub mod webui;

#[cfg(feature = "ml")]
pub mod ml;

#[cfg(feature = "mcp")]
pub mod mcp;

// Re-exports for convenience
pub use state::StateManager;
pub use plugin_system::{Plugin, PluginRegistry};
pub use event_bus::{EventBus, Event};