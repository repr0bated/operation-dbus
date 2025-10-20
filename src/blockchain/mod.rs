//! Blockchain module - immutable log with hash footprints
pub mod plugin_footprint;
pub mod streaming_blockchain;

pub use plugin_footprint::PluginFootprint;
pub use streaming_blockchain::StreamingBlockchain;
