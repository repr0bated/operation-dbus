#![allow(unused_imports)]
//! Streaming blockchain with vectorization and dual btrfs subvolumes
//!
//! This module provides a streaming blockchain implementation that:
//! 1. Automatically generates hashed footprints for all object modifications
//! 2. Stores timing and vector data in separate btrfs subvolumes
//! 3. Creates snapshots for each block
//! 4. Streams vector data to remote vector databases via btrfs send/receive

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};
use crate::plugin_footprint::PluginFootprint;
use tokio::time::{sleep, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockEvent {
    pub timestamp: u64,
    pub category: String,
    pub action: String,
    pub data: serde_json::Value,
    pub hash: String,
    pub vector: Vec<f32>,
}

pub struct StreamingBlockchain {
    base_path: PathBuf,
    timing_subvol: PathBuf,
    vector_subvol: PathBuf,
}

impl StreamingBlockchain {
    pub async fn new(base_path: impl AsRef<Path>) -> Result<Self> {
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

        self.create_snapshot(&event.hash).await?;
        info!("Plugin footprint added with hash: {}", event.hash);
        Ok(event.hash)
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

    pub async fn stream_vectors(&self, block_hash: &str, remote: &str) -> Result<()> {
        let vector_snapshot = self.base_path.join("snapshots").join(format!("vectors-{}", block_hash));
        
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

    pub async fn stream_to_replicas(&self, block_hash: &str, replicas: &[String]) -> Result<()> {
        let vector_snapshot = self.base_path.join("snapshots").join(format!("vectors-{}", block_hash));
        
        let mut tee_args = Vec::new();
        for replica in replicas {
            tee_args.push(format!(">(ssh {} 'btrfs receive /var/lib/blockchain/vectors/')", replica));
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
            anyhow::bail!("Multi-replica stream failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(())
    }
}
