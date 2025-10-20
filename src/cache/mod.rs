//! BTRFS-based caching layer with automatic snapshot management
//!
//! This module provides unlimited disk-based caching using BTRFS subvolumes
//! with transparent compression and automatic snapshot rotation.

pub mod btrfs_cache;
pub mod snapshot_manager;

pub use btrfs_cache::{BtrfsCache, CacheStats};
pub use snapshot_manager::{SnapshotManager, SnapshotConfig};
