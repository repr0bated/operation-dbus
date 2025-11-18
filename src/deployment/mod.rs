//! Deployment image management with BTRFS snapshots and symlink deduplication

pub mod image_manager;

pub use image_manager::{ImageManager, ImageMetadata, FileEntry};

