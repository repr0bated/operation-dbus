//! Operation D-Bus - Declarative system state management via native protocols
//!
//! This crate provides declarative system state management through native Linux protocols.

// Core modules
pub mod blockchain;
pub mod cache;
pub mod introspection;
pub mod native;
pub mod nonnet_db;
pub mod state;

// Loose coupling modules
pub mod event_bus;
pub mod plugin_system;

// Optional modules
#[cfg(feature = "web")]
pub mod webui;

#[cfg(feature = "ml")]
pub mod ml;

#[cfg(feature = "mcp")]
pub mod mcp;

// Re-exports for convenience
pub use event_bus::{Event, EventBus};
pub use plugin_system::{Plugin, PluginRegistry};
pub use state::StateManager;
