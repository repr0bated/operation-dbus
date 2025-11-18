//! MCP (Model Context Protocol) integration module
//!
//! This module provides MCP server functionality with D-Bus orchestration
//! for Linux system automation.

pub mod agents {
    pub mod executor;
    pub mod file;
    pub mod monitor;
    pub mod network;
    pub mod packagekit;
    pub mod systemd;
}

// Core MCP modules
pub mod bridge;
pub mod discovery;
// pub mod discovery_enhanced;  // File not found
pub mod hybrid_dbus_bridge;
pub mod hybrid_scanner;
pub mod introspection_parser;
pub mod json_introspection;
pub mod orchestrator;
pub mod system_introspection;

// Refactored modules for loose coupling
pub mod agent_registry;
pub mod tool_registry;

// MCP tools
pub mod tools {
    pub mod introspection;
}

// Chat interface
pub mod chat_server;
pub mod ai_context_provider;
pub mod ollama;

// Flow-based workflows
pub mod workflows;

// D-Bus indexer for hierarchical abstraction
pub mod dbus_indexer;

// Introspection cache with JSON/SQLite storage
pub mod introspection_cache;

// Introspection tools for MCP
pub mod introspection_tools;

// Embedded resources for MCP
pub mod resources;

#[cfg(feature = "mcp")]
pub mod web_bridge;
#[cfg(feature = "mcp")]
pub mod web_bridge_improved;

// lib.rs is a small utility module for re-exports
pub mod lib;
