// ML/Transformer module for on-demand vectorization

pub mod config;
pub mod downloader;
pub mod embedder;
pub mod model_manager;

#[cfg(feature = "ml")]
pub use model_manager::ModelManager;
