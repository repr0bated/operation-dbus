//! op-inspector: Inspector Gadget - Universal Object Inspector
//!
//! Features:
//! - Inspect ANY data structure (JSON, XML, binary, Docker, DBus, Proxmox)
//! - AI-powered gap filling for incomplete introspections
//! - Schema generation and validation
//! - Knowledge base integration
//! - Proxmox LXC template introspection (4500+ editable elements)
//! - GCloud CLI introspection (100+ command groups, all flags/args)

mod introspective_gadget;
pub mod gcloud;

// Re-export main types
pub use introspective_gadget::*;
pub use gcloud::{GCloudParser, GCloudSchema, GCloudCommand, GCloudFlag, GCloudArg, GCloudStats, introspect_gcloud};

use op_introspection::IntrospectionService;
use std::sync::Arc;

/// Simplified Inspector Gadget wrapper
pub struct InspectorGadget {
    introspection: Arc<IntrospectionService>,
}

impl InspectorGadget {
    pub fn new(introspection: Arc<IntrospectionService>) -> Self {
        Self { introspection }
    }

    pub fn introspection(&self) -> Arc<IntrospectionService> {
        Arc::clone(&self.introspection)
    }
}
