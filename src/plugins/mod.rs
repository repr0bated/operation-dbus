// src/plugins/mod.rs - Plugin modules

pub mod lxc;
pub mod net;
pub mod network;

#[cfg(feature = "packagekit")]
pub mod packagekit;

// Export plugin types
pub use lxc as lxc_plugin;
pub use net as legacy_network;
pub use network::NetworkPlugin;

#[cfg(feature = "packagekit")]
pub use packagekit::PackageKitPlugin;
