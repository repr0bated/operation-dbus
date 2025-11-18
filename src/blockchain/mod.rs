//! Blockchain module - immutable log with hash footprints
pub mod plugin_footprint;

#[cfg(feature = "streaming-blockchain")]
pub mod streaming_blockchain;

#[cfg(all(feature = "streaming-blockchain", feature = "cache"))]
pub mod btrfs_numa_integration;

pub use plugin_footprint::PluginFootprint;

#[cfg(feature = "streaming-blockchain")]
pub use streaming_blockchain::StreamingBlockchain;

#[cfg(all(feature = "streaming-blockchain", feature = "cache"))]
pub use btrfs_numa_integration::OptimizedBlockchain;
