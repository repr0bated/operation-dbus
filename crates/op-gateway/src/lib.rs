//! op-gateway: MCP Gateway with WireGuard authentication and smart routing

mod error;
pub mod wireguard_auth;
pub mod encrypted_storage;
pub mod mcp_gateway;

pub use error::*;
pub use wireguard_auth::*;
pub use encrypted_storage::*;
pub use mcp_gateway::*;
