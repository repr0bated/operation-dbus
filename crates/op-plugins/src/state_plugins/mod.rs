//! State plugins - each manages a domain via native protocols
//!
//! These plugins implement the StatePlugin trait from op-state

// pub mod dnsresolver;
// pub mod full_system;
// pub mod keyring;
// pub mod login1;
// pub mod lxc;
pub mod mcp;
pub mod net;
// pub mod netmaker;
// pub mod openflow;
// pub mod openflow_obfuscation;
// pub mod packagekit;
// pub mod pcidecl;
// pub mod privacy;
// pub mod privacy_router;
pub mod sessdecl;
pub mod adc;
pub mod endpoint;
pub mod gcloud_adc;
pub mod keypair;
pub mod proxy_server;
pub mod systemd;
// pub mod systemd_networkd;

pub mod agent_config;
pub mod ovsdb_bridge;
pub mod hardware;
pub mod proxmox;
pub mod software;
pub mod users;
pub mod wireguard;
pub mod web_ui;

// Re-export plugin types
// pub use dnsresolver::DnsResolverPlugin;
// pub use full_system::FullSystemPlugin;
// pub use login1::Login1Plugin;
// pub use lxc::LxcPlugin;
pub use mcp::McpStatePlugin;
pub use mcp::{ToolDefinition, ExecutionResult};
pub use net::NetStatePlugin;
// pub use netmaker::NetmakerPlugin;
// pub use openflow::OpenFlowPlugin;
// pub use openflow_obfuscation::OpenFlowObfuscationPlugin;
// pub use packagekit::PackageKitPlugin;
// pub use pcidecl::PciDeclPlugin;
// pub use privacy::PrivacyPlugin;
// pub use privacy_router::PrivacyRouterPlugin;
pub use sessdecl::SessDeclPlugin;
pub use adc::AdcPlugin;
pub use endpoint::EndpointPlugin;
pub use gcloud_adc::GcloudAdcPlugin;
pub use keypair::KeypairPlugin;
pub use proxy_server::ProxyServerPlugin;
pub use systemd::SystemdStatePlugin;
pub use agent_config::AgentConfigPlugin;
pub use hardware::HardwarePlugin;
pub use ovsdb_bridge::OvsBridgePlugin;
pub use proxmox::ProxmoxPlugin;
pub use software::SoftwarePlugin;
pub use users::UsersPlugin;
pub use wireguard::WireGuardPlugin;
pub use web_ui::WebUiPlugin;
// pub use systemd_networkd::SystemdNetworkdPlugin; // TODO: Plugin not yet implemented

