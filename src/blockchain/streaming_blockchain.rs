#![allow(unused_imports)]
//! Streaming blockchain with vectorization and dual btrfs subvolumes
//!
//! This module provides a streaming blockchain implementation that:
//! 1. Automatically generates hashed footprints for all object modifications
//! 2. Stores timing and vector data in separate btrfs subvolumes
//! 3. Creates snapshots for each block
//! 4. Streams vector data to remote vector databases via btrfs send/receive

use crate::blockchain::PluginFootprint;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockEvent {
    pub timestamp: u64,
    pub category: String,
    pub action: String,
    pub data: serde_json::Value,
    pub hash: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Copy)]
pub enum SnapshotInterval {
    PerOperation,
    EveryMinute,
    Every5Minutes,
    Every15Minutes,
    Every30Minutes,
    Hourly,
    Daily,
    Weekly,
}

impl SnapshotInterval {
    /// Parse from environment variable or string
    pub fn from_env() -> Self {
        match std::env::var("OPDBUS_SNAPSHOT_INTERVAL")
            .unwrap_or_else(|_| "every-15-minutes".to_string())
            .to_lowercase()
            .as_str()
        {
            "per-op" | "per-operation" | "per_operation" => SnapshotInterval::PerOperation,
            "every-minute" | "1-minute" | "1min" => SnapshotInterval::EveryMinute,
            "every-5-minutes" | "5-minutes" | "5min" => SnapshotInterval::Every5Minutes,
            "every-15-minutes" | "15-minutes" | "15min" => SnapshotInterval::Every15Minutes,
            "every-30-minutes" | "30-minutes" | "30min" => SnapshotInterval::Every30Minutes,
            "hourly" | "1-hour" | "1h" => SnapshotInterval::Hourly,
            "daily" | "1-day" | "1d" => SnapshotInterval::Daily,
            "weekly" | "1-week" | "1w" => SnapshotInterval::Weekly,
            _ => {
                warn!("Invalid OPDBUS_SNAPSHOT_INTERVAL, defaulting to every-15-minutes");
                SnapshotInterval::Every15Minutes
            }
        }
    }
}

impl Default for SnapshotInterval {
    fn default() -> Self {
        SnapshotInterval::Every15Minutes // Default to every 15 minutes for production
    }
}

pub struct StreamingBlockchain {
    base_path: PathBuf,
    timing_subvol: PathBuf,
    vector_subvol: PathBuf,
    snapshot_interval: SnapshotInterval,
    last_snapshot_time: Arc<RwLock<Instant>>,
}

impl StreamingBlockchain {
    pub async fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_with_interval(base_path, SnapshotInterval::from_env()).await
    }

    pub async fn new_with_interval(
        base_path: impl AsRef<Path>,
        snapshot_interval: SnapshotInterval,
    ) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let timing_subvol = base_path.join("timing");
        let vector_subvol = base_path.join("vectors");

        tokio::fs::create_dir_all(&base_path).await?;
        Self::create_subvolume(&timing_subvol).await?;
        Self::create_subvolume(&vector_subvol).await?;

        Ok(Self {
            base_path,
            timing_subvol,
            vector_subvol,
            snapshot_interval,
            last_snapshot_time: Arc::new(RwLock::new(Instant::now())),
        })
    }

    async fn create_subvolume(path: &Path) -> Result<()> {
        if !path.exists() {
            let output = Command::new("btrfs")
                .args(["subvolume", "create"])
                .arg(path)
                .output()
                .await
                .context("Failed to execute btrfs command")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("btrfs subvolume create failed: {}", stderr);
            }
        }
        Ok(())
    }

    pub async fn add_footprint(&self, footprint: PluginFootprint) -> Result<String> {
        let data = serde_json::json!({
            "plugin_id": footprint.plugin_id,
            "operation": footprint.operation,
            "data_hash": footprint.data_hash,
            "metadata": footprint.metadata
        });

        let event = BlockEvent {
            timestamp: footprint.timestamp,
            category: footprint.plugin_id.clone(),
            action: footprint.operation.clone(),
            data,
            hash: footprint.content_hash.clone(),
            vector: footprint.vector_features,
        };

        let timing_file = self.timing_subvol.join(format!("{}.json", event.hash));
        let timing_data = serde_json::json!({
            "timestamp": event.timestamp,
            "category": event.category,
            "action": event.action,
            "hash": event.hash,
            "data": event.data,
            "plugin_footprint": true
        });
        tokio::fs::write(&timing_file, serde_json::to_string_pretty(&timing_data)?).await?;

        let vector_file = self.vector_subvol.join(format!("{}.vec", event.hash));
        let vector_data = serde_json::json!({
            "hash": event.hash,
            "vector": event.vector,
            "metadata": {
                "category": event.category,
                "action": event.action,
                "timestamp": event.timestamp,
                "plugin_id": footprint.plugin_id,
                "data_hash": footprint.data_hash
            }
        });
        tokio::fs::write(&vector_file, serde_json::to_string(&vector_data)?).await?;

        // Only create snapshot if interval requires it
        self.create_snapshot_if_needed(&event.hash).await?;
        info!("Plugin footprint added with hash: {}", event.hash);
        Ok(event.hash)
    }

    /// Add multiple footprints in batch (for bulk operations)
    pub async fn add_footprints_batch(
        &self,
        footprints: Vec<PluginFootprint>,
    ) -> Result<Vec<String>> {
        let mut hashes = Vec::new();

        for footprint in footprints {
            let hash = self.add_footprint(footprint).await?;
            hashes.push(hash);
        }

        // Create a batch snapshot after processing all footprints
        if !hashes.is_empty() {
            let batch_hash = format!("batch-{}", hashes.len());
            self.create_snapshot(&batch_hash).await?;
            info!("Created batch snapshot for {} footprints", hashes.len());
        }

        Ok(hashes)
    }

    pub async fn start_footprint_receiver(
        &self,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<PluginFootprint>,
    ) -> Result<()> {
        info!("Starting plugin footprint receiver");

        while let Some(footprint) = receiver.recv().await {
            if let Err(e) = self.add_footprint(footprint).await {
                tracing::error!("Failed to add plugin footprint: {}", e);
                // Continue processing other footprints instead of failing completely
            }
        }

        info!("Plugin footprint receiver shutting down");
        Ok(())
    }

    /// Create snapshot only if the time interval has elapsed
    async fn create_snapshot_if_needed(&self, block_hash: &str) -> Result<()> {
        match self.snapshot_interval {
            SnapshotInterval::PerOperation => {
                // Always create snapshot (original behavior)
                self.create_snapshot(block_hash).await
            }
            SnapshotInterval::EveryMinute => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(60) {
                    // 1 minute has passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
            SnapshotInterval::Every5Minutes => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(300) {
                    // 5 minutes have passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
            SnapshotInterval::Every15Minutes => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(900) {
                    // 15 minutes have passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
            SnapshotInterval::Every30Minutes => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(1800) {
                    // 30 minutes have passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
            SnapshotInterval::Hourly => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(3600) {
                    // 1 hour has passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
            SnapshotInterval::Daily => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(86400) {
                    // 24 hours have passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
            SnapshotInterval::Weekly => {
                let now = Instant::now();
                let last_snapshot = *self.last_snapshot_time.read().await;

                if now.duration_since(last_snapshot) >= Duration::from_secs(604800) {
                    // 7 days have passed
                    self.create_snapshot(block_hash).await?;
                    *self.last_snapshot_time.write().await = now;
                }
                Ok(())
            }
        }
    }

    async fn create_snapshot(&self, block_hash: &str) -> Result<()> {
        let snapshot_dir = self.base_path.join("snapshots");
        tokio::fs::create_dir_all(&snapshot_dir).await?;

        let timing_snapshot = snapshot_dir.join(format!("timing-{}", block_hash));
        Command::new("btrfs")
            .args(["subvolume", "snapshot", "-r"])
            .arg(&self.timing_subvol)
            .arg(&timing_snapshot)
            .output()
            .await
            .context("Failed to create timing snapshot")?;

        let vector_snapshot = snapshot_dir.join(format!("vectors-{}", block_hash));
        Command::new("btrfs")
            .args(["subvolume", "snapshot", "-r"])
            .arg(&self.vector_subvol)
            .arg(&vector_snapshot)
            .output()
            .await
            .context("Failed to create vector snapshot")?;

        debug!("Created snapshots for block: {}", block_hash);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn stream_vectors(&self, block_hash: &str, remote: &str) -> Result<()> {
        let vector_snapshot = self
            .base_path
            .join("snapshots")
            .join(format!("vectors-{}", block_hash));

        info!("Streaming vectors for block {} to {}", block_hash, remote);

        let output = Command::new("bash")
            .arg("-c")
            .arg(format!(
                "btrfs send {} | ssh {} 'btrfs receive /var/lib/blockchain/vectors/'",
                vector_snapshot.display(),
                remote
            ))
            .output()
            .await
            .context("Failed to stream vectors")?;

        if !output.status.success() {
            anyhow::bail!("Stream failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn stream_to_replicas(&self, block_hash: &str, replicas: &[String]) -> Result<()> {
        let vector_snapshot = self
            .base_path
            .join("snapshots")
            .join(format!("vectors-{}", block_hash));

        let mut tee_args = Vec::new();
        for replica in replicas {
            tee_args.push(format!(
                ">(ssh {} 'btrfs receive /var/lib/blockchain/vectors/')",
                replica
            ));
        }

        let cmd = format!(
            "btrfs send {} | tee {} > /dev/null",
            vector_snapshot.display(),
            tee_args.join(" ")
        );

        info!("Streaming to {} replicas", replicas.len());

        let output = Command::new("bash")
            .arg("-c")
            .arg(&cmd)
            .output()
            .await
            .context("Failed to stream to replicas")?;

        if !output.status.success() {
            anyhow::bail!(
                "Multi-replica stream failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get current snapshot interval configuration
    pub fn snapshot_interval(&self) -> SnapshotInterval {
        self.snapshot_interval
    }

    /// Set snapshot interval (for runtime configuration)
    pub fn set_snapshot_interval(&mut self, interval: SnapshotInterval) {
        self.snapshot_interval = interval;
    }
}
