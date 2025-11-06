//! MCP (Model Context Protocol) integration module
//!
//! This module provides MCP server functionality with D-Bus orchestration
//! for Linux system automation.

pub mod agents {
    pub mod executor;
    pub mod file;
    pub mod monitor;
    pub mod network;
    pub mod systemd;
}

// Core MCP modules
pub mod bridge;
pub mod discovery;
pub mod discovery_enhanced;
pub mod introspection_parser;
pub mod json_introspection;
pub mod orchestrator;

// Refactored modules for loose coupling
pub mod agent_registry;
pub mod tool_registry;

// Introspection tools for hardware, CPU features, ISP analysis
pub mod introspection_tools;

// MCP Manager (web interface for orchestration)
pub mod manager;

// Chat interface
pub mod chat_server;

#[cfg(feature = "mcp")]
pub mod web_bridge;
#[cfg(feature = "mcp")]
pub mod web_bridge_improved;

// lib.rs is a small utility module for re-exports
pub mod lib;
