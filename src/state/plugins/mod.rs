//! State plugins - each manages a domain via native protocols
pub mod login1;
pub mod lxc;
pub mod net;
#[cfg(feature = "openflow")]
pub mod openflow;
pub mod sessdecl;
pub mod systemd;

pub mod dnsresolver;
pub mod pcidecl;
pub mod packagekit;
pub use dnsresolver::DnsResolverPlugin;
pub use login1::Login1Plugin;
pub use lxc::LxcPlugin;
pub use net::NetStatePlugin;
pub use pcidecl::PciDeclPlugin;
pub use packagekit::PackageKitPlugin;
pub use sessdecl::SessDeclPlugin;
pub use systemd::SystemdStatePlugin;

#[cfg(feature = "openflow")]
pub use openflow::OpenFlowPlugin;
