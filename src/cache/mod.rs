//! BTRFS-based caching layer with automatic snapshot management
//!
//! This module provides unlimited disk-based caching using BTRFS subvolumes
//! with transparent compression and automatic snapshot rotation.
//!
//! Features:
//! - NUMA-aware CPU affinity for L3 cache optimization
//! - Automatic NUMA topology detection
//! - Per-node statistics and monitoring

pub mod btrfs_cache;
pub mod numa;
pub mod snapshot_manager;

pub use btrfs_cache::BtrfsCache;
pub use numa::{NumaNode, NumaStats, NumaTopology};
