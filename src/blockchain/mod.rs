//! Blockchain module - immutable log with hash footprints
pub mod plugin_footprint;

#[cfg(feature = "streaming-blockchain")]
pub mod streaming_blockchain;

pub use plugin_footprint::PluginFootprint;

#[cfg(feature = "streaming-blockchain")]
pub use streaming_blockchain::StreamingBlockchain;
