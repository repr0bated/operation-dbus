//! D-Bus Interface for the Mirror Service

use zbus::interface;
use std::sync::Arc;
use crate::DbusMirror;

pub struct DbusMirrorInterface {
    mirror: Arc<DbusMirror>,
}

impl DbusMirrorInterface {
    pub fn new(mirror: Arc<DbusMirror>) -> Self {
        Self { mirror }
    }
}

#[interface(name = "org.opdbus.MirrorV1")]
impl DbusMirrorInterface {
    /// Force a reconciliation/sync of the D-Bus tree from databases
    async fn reconcile(&self) -> zbus::fdo::Result<()> {
        self.mirror.reconcile().await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    /// Get current mirror statistics
    async fn get_stats(&self) -> zbus::fdo::Result<String> {
        let stats = simd_json::json!({
            "projected_objects": self.mirror.projected_count(),
        });
        Ok(simd_json::to_string(&stats).unwrap_or_default())
    }
}
