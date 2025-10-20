// ML/Transformer module for on-demand vectorization

pub mod config;
pub mod downloader;
pub mod embedder;
pub mod model_manager;

pub use config::{ExecutionProvider, VectorizationConfig, VectorizationLevel};
pub use downloader::ModelDownloader;
pub use embedder::TextEmbedder;
pub use model_manager::ModelManager;
