//! State plugins - each manages a domain via native protocols
pub mod dnsresolver;
pub mod login1;
pub mod lxc;
pub mod net;
pub mod openflow;
pub mod packagekit;
pub mod pcidecl;
pub mod sessdecl;
pub mod systemd;

pub use dnsresolver::DnsResolverPlugin;
pub use login1::Login1Plugin;
pub use lxc::LxcPlugin;
pub use net::NetStatePlugin;
pub use openflow::OpenFlowPlugin;
pub use packagekit::PackageKitPlugin;
pub use pcidecl::PciDeclPlugin;
pub use sessdecl::SessDeclPlugin;
pub use systemd::SystemdStatePlugin;
