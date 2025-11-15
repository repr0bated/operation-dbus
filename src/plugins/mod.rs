// src/plugins/mod.rs - Plugin modules

pub mod lxc;
pub mod net;

#[cfg(feature = "packagekit")]
pub mod packagekit;

// Export plugin types
pub use lxc as lxc_plugin;
pub use net as network;

#[cfg(feature = "packagekit")]
pub use packagekit::PackageKitPlugin;

// Re-export for apply command
pub mod network {
    use serde::{Deserialize, Serialize};
    use anyhow::Result;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NetworkPlugin {
        // TODO: Add network configuration
    }

    impl NetworkPlugin {
        pub async fn apply(&self) -> Result<()> {
            // TODO: Implement network configuration
            Ok(())
        }
    }
}
