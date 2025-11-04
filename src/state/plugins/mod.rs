//! State plugins - each manages a domain via native protocols
pub mod login1;
pub mod lxc;
pub mod net;
pub mod openflow;
pub mod sessdecl;
pub mod systemd;

pub mod dnsresolver;
pub mod pcidecl;
pub use dnsresolver::DnsResolverPlugin;
pub use login1::Login1Plugin;
pub use lxc::LxcPlugin;
pub use net::NetStatePlugin;
pub use openflow::OpenFlowPlugin;
pub use pcidecl::PciDeclPlugin;
pub use sessdecl::SessDeclPlugin;
pub use systemd::SystemdStatePlugin;
