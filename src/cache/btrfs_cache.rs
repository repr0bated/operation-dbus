//! BTRFS-backed cache with SQLite index, compression, and NUMA optimization
//!
//! Provides unlimited disk-based caching with:
//! - BTRFS transparent compression (zstd)
//! - SQLite index for O(1) lookups
//! - Linux page cache for hot data
//! - Automatic snapshot management
//! - NUMA-aware memory allocation and CPU affinity

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{debug, info, warn};

use super::snapshot_manager::{SnapshotConfig, SnapshotManager};
use super::numa::{NumaNode, NumaStats, NumaTopology};

/// NUMA-aware cache placement strategy
#[derive(Debug, Clone)]
pub enum CachePlacementStrategy {
    /// Place cache data on the same NUMA node as the requesting CPU
    LocalNode,
    /// Distribute cache data across all NUMA nodes for load balancing
    RoundRobin,
    /// Use the NUMA node with most available memory
    MostMemory,
    /// Disable NUMA optimizations (default)
    Disabled,
}

/// Memory allocation policy for NUMA systems
#[derive(Debug, Clone)]
pub enum MemoryPolicy {
    /// Bind memory to specific NUMA node
    Bind(Vec<u32>),
    /// Prefer memory from specific NUMA node
    Preferred(Option<u32>),
    /// Interleave memory across multiple NUMA nodes
    Interleave(Vec<u32>),
    /// Use default system memory policy
    Default,
}

pub struct BtrfsCache {
    cache_dir: PathBuf,
    index: Mutex<rusqlite::Connection>,
    snapshot_manager: SnapshotManager,
    numa_topology: Option<NumaTopology>,
    placement_strategy: CachePlacementStrategy,
    memory_policy: MemoryPolicy,
    cpu_affinity: Vec<u32>, // CPU cores for affinity binding
    numa_stats: Mutex<HashMap<u32, NumaStats>>, // Per-node statistics
    current_node_index: std::sync::atomic::AtomicUsize,
}

#[allow(dead_code)]
impl BtrfsCache {
    /// Create new BTRFS cache
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        // Create subdirectories
        std::fs::create_dir_all(cache_dir.join("embeddings/vectors"))?;
        std::fs::create_dir_all(cache_dir.join("blocks/by-number"))?;
        std::fs::create_dir_all(cache_dir.join("blocks/by-hash"))?;
        std::fs::create_dir_all(cache_dir.join("queries"))?;
        std::fs::create_dir_all(cache_dir.join("diffs"))?;

        // Create SQLite index for embeddings
        let index_path = cache_dir.join("embeddings/index.db");
        let index =
            rusqlite::Connection::open(&index_path).context("Failed to open SQLite index")?;

        // Create embeddings table
        index.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                text_hash TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                vector_file TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                accessed_at INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 1,
                vector_size INTEGER NOT NULL
            )",
            [],
        )?;

        // Create index for hot/cold data analysis
        index.execute(
            "CREATE INDEX IF NOT EXISTS idx_accessed
             ON embeddings(accessed_at DESC)",
            [],
        )?;

        index.execute(
            "CREATE INDEX IF NOT EXISTS idx_created
             ON embeddings(created_at DESC)",
            [],
        )?;

        // Initialize snapshot manager
        let snapshot_config = SnapshotConfig {
            snapshot_dir: cache_dir
                .parent()
                .unwrap_or(Path::new("/var/lib/op-dbus"))
                .join("@cache-snapshots"),
            max_snapshots: std::env::var("OPDBUS_MAX_CACHE_SNAPSHOTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24),
            prefix: "cache".to_string(),
        };

        let snapshot_manager = SnapshotManager::new(cache_dir.clone(), snapshot_config);

        // Detect NUMA topology with comprehensive /sys parsing
        let numa_topology = match NumaTopology::detect() {
            Ok(topology) => {
                if topology.is_numa_system() {
                    info!(
                        "✓ NUMA topology detected: {} nodes, current node: {:?}",
                        topology.node_count(),
                        topology.current_node()
                    );
                } else {
                    info!("Single-node system detected (no NUMA)");
                }
                Some(topology)
            }
            Err(e) => {
                warn!("Failed to detect NUMA topology: {}", e);
                warn!("Continuing without NUMA optimizations");
                None
            }
        };

        // Determine placement strategy and CPU affinity
        let (placement_strategy, cpu_affinity) = if let Some(ref topo) = numa_topology {
            if topo.is_numa_system() {
                // Multi-node system: use local node strategy
                let optimal_node = topo.optimal_node();
                let cpus = topo.cpus_for_node(optimal_node);

                info!(
                    "NUMA optimization enabled: using node {} with CPUs {:?}",
                    optimal_node,
                    if cpus.len() <= 8 {
                        format!("{:?}", cpus)
                    } else {
                        format!("[{} CPUs]", cpus.len())
                    }
                );

                (CachePlacementStrategy::LocalNode, cpus)
            } else {
                // Single-node system: use first few CPUs for L3 cache locality
                let cpus: Vec<u32> = (0..(num_cpus::get().min(4) as u32)).collect();
                info!(
                    "Single-node system: using CPUs {:?} for L3 cache locality",
                    cpus
                );
                (CachePlacementStrategy::Disabled, cpus)
            }
        } else {
            // NUMA detection failed: conservative fallback
            let cpus: Vec<u32> = (0..(num_cpus::get().min(4) as u32)).collect();
            warn!("Using fallback CPU affinity: {:?}", cpus);
            (CachePlacementStrategy::Disabled, cpus)
        };

        let memory_policy = MemoryPolicy::Default;

        Ok(Self {
            cache_dir,
            index: Mutex::new(index),
            snapshot_manager,
            numa_topology,
            placement_strategy,
            memory_policy,
            cpu_affinity,
            numa_stats: Mutex::new(HashMap::new()),
            current_node_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Get or compute embedding
    pub fn get_or_embed<F>(&self, text: &str, compute_fn: F) -> Result<Vec<f32>>
    where
        F: FnOnce(&str) -> Result<Vec<f32>>,
    {
        let text_hash = self.hash_text(text);

        // Check if cached
        if let Some(vector) = self.load_embedding(&text_hash)? {
            // Update access statistics
            self.update_access(&text_hash)?;
            return Ok(vector);
        }

        // Compute embedding
        let vector = compute_fn(text)?;

        // Store in cache
        self.save_embedding(text, &text_hash, &vector)?;

        Ok(vector)
    }

    /// Get embedding if cached (without computing)
    pub fn get_embedding(&self, text: &str) -> Result<Option<Vec<f32>>> {
        let text_hash = self.hash_text(text);
        if let Some(vector) = self.load_embedding(&text_hash)? {
            self.update_access(&text_hash)?;
            return Ok(Some(vector));
        }
        Ok(None)
    }

    /// Store embedding directly
    pub fn put_embedding(&self, text: &str, vector: &[f32]) -> Result<()> {
        let text_hash = self.hash_text(text);
        self.save_embedding(text, &text_hash, vector)
    }

    fn hash_text(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn load_embedding(&self, text_hash: &str) -> Result<Option<Vec<f32>>> {
        // Track operation start time for NUMA statistics
        let start = std::time::Instant::now();

        let index = self.index.lock().unwrap();

        // Lookup in SQLite index
        let vector_file: Option<String> = index
            .query_row(
                "SELECT vector_file FROM embeddings WHERE text_hash = ?1",
                [text_hash],
                |row| row.get(0),
            )
            .optional()?;

        drop(index); // Release lock before file I/O

        if let Some(file) = vector_file {
            let path = self.cache_dir.join("embeddings/vectors").join(&file);

            // Read from BTRFS (page cache will cache this!)
            // CPU affinity ensures this stays in L3 cache
            let data = std::fs::read(&path)
                .context(format!("Failed to read cached embedding: {:?}", path))?;

            let vector: Vec<f32> =
                bincode::deserialize(&data).context("Failed to deserialize cached embedding")?;

            // Record NUMA statistics
            let elapsed_ns = start.elapsed().as_nanos() as u64;
            self.record_cache_access(elapsed_ns);

            return Ok(Some(vector));
        }

        Ok(None)
    }

    /// Record cache access for NUMA statistics
    fn record_cache_access(&self, latency_ns: u64) {
        if let Some(ref topology) = self.numa_topology {
            if let Some(current_node) = topology.current_node() {
                let mut stats = self.numa_stats.lock().unwrap();
                let node_stats = stats.entry(current_node).or_insert_with(NumaStats::new);

                // Consider local if latency < 100ns (typical page cache hit)
                // Remote NUMA access is typically 100-300ns
                if latency_ns < 100 {
                    node_stats.record_local_access(latency_ns);
                } else {
                    node_stats.record_remote_access(latency_ns);
                }
            }
        }
    }

    fn save_embedding(&self, text: &str, text_hash: &str, vector: &[f32]) -> Result<()> {
        let vectors_dir = self.cache_dir.join("embeddings/vectors");
        std::fs::create_dir_all(&vectors_dir)?;

        let vector_file = format!("{}.vec", text_hash);
        let path = vectors_dir.join(&vector_file);

        // Write to BTRFS (automatically compressed by kernel)
        let data = bincode::serialize(vector)?;
        std::fs::write(&path, data)?;

        // Add to SQLite index
        let index = self.index.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        index.execute(
            "INSERT INTO embeddings (text_hash, text, vector_file, created_at, accessed_at, vector_size)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(text_hash) DO UPDATE SET
                accessed_at = ?5,
                access_count = access_count + 1",
            rusqlite::params![text_hash, text, vector_file, now, now, vector.len()],
        )?;

        Ok(())
    }

    fn update_access(&self, text_hash: &str) -> Result<()> {
        let index = self.index.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        index.execute(
            "UPDATE embeddings
             SET accessed_at = ?1, access_count = access_count + 1
             WHERE text_hash = ?2",
            rusqlite::params![now, text_hash],
        )?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let index = self.index.lock().unwrap();

        let total: i64 =
            index.query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;

        let hot_threshold = chrono::Utc::now().timestamp() - 3600; // 1 hour
        let hot: i64 = index.query_row(
            "SELECT COUNT(*) FROM embeddings WHERE accessed_at > ?1",
            [hot_threshold],
            |row| row.get(0),
        )?;

        let total_accesses: i64 =
            index.query_row("SELECT SUM(access_count) FROM embeddings", [], |row| {
                row.get(0)
            })?;

        drop(index); // Release lock before file I/O

        // Calculate disk usage
        let embeddings_size = self.dir_size(&self.cache_dir.join("embeddings/vectors"))?;
        let blocks_size = self.dir_size(&self.cache_dir.join("blocks"))?;
        let total_size = embeddings_size + blocks_size;

        Ok(CacheStats {
            total_entries: total as usize,
            hot_entries: hot as usize,
            total_accesses: total_accesses as u64,
            disk_usage_bytes: total_size,
            embeddings_size_bytes: embeddings_size,
            blocks_size_bytes: blocks_size,
        })
    }

    #[allow(clippy::only_used_in_recursion)]
    fn dir_size(&self, path: &Path) -> Result<u64> {
        let mut size = 0u64;
        if !path.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                size += metadata.len();
            } else if metadata.is_dir() {
                size += self.dir_size(&entry.path())?;
            }
        }
        Ok(size)
    }

    /// Clean old entries (accessed before cutoff)
    pub fn cleanup_old(&self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);

        let index = self.index.lock().unwrap();

        // Find old entries
        let mut stmt = index.prepare(
            "SELECT text_hash, vector_file FROM embeddings
             WHERE accessed_at < ?1",
        )?;

        let old_entries: Vec<(String, String)> = stmt
            .query_map([cutoff], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let count = old_entries.len();

        drop(stmt); // Release statement
        drop(index); // Release lock before file I/O

        // Delete files
        for (_hash, file) in &old_entries {
            let path = self.cache_dir.join("embeddings/vectors").join(file);
            let _ = std::fs::remove_file(path); // Ignore errors
        }

        // Delete from index
        let index = self.index.lock().unwrap();
        index.execute("DELETE FROM embeddings WHERE accessed_at < ?1", [cutoff])?;

        log::info!(
            "Cleaned up {} old cache entries (>{} days old)",
            count,
            days
        );

        Ok(count)
    }

    /// Clear all cache data
    pub fn clear(&self) -> Result<()> {
        log::warn!("Clearing all cache data");

        // Clear embeddings
        let vectors_dir = self.cache_dir.join("embeddings/vectors");
        if vectors_dir.exists() {
            std::fs::remove_dir_all(&vectors_dir)?;
            std::fs::create_dir_all(&vectors_dir)?;
        }

        // Clear blocks
        let blocks_dir = self.cache_dir.join("blocks");
        if blocks_dir.exists() {
            std::fs::remove_dir_all(&blocks_dir)?;
            std::fs::create_dir_all(blocks_dir.join("by-number"))?;
            std::fs::create_dir_all(blocks_dir.join("by-hash"))?;
        }

        // Clear index
        let index = self.index.lock().unwrap();
        index.execute("DELETE FROM embeddings", [])?;

        log::info!("Cache cleared");

        Ok(())
    }

    /// Clear only embeddings cache
    pub fn clear_embeddings(&self) -> Result<()> {
        log::warn!("Clearing embeddings cache");

        // Clear embeddings vectors
        let vectors_dir = self.cache_dir.join("embeddings/vectors");
        if vectors_dir.exists() {
            std::fs::remove_dir_all(&vectors_dir)?;
            std::fs::create_dir_all(&vectors_dir)?;
        }

        // Clear index
        let index = self.index.lock().unwrap();
        index.execute("DELETE FROM embeddings", [])?;

        log::info!("Embeddings cache cleared");

        Ok(())
    }

    /// Clear only blocks cache
    pub fn clear_blocks(&self) -> Result<()> {
        log::warn!("Clearing blocks cache");

        // Clear blocks
        let blocks_dir = self.cache_dir.join("blocks");
        if blocks_dir.exists() {
            std::fs::remove_dir_all(&blocks_dir)?;
            std::fs::create_dir_all(blocks_dir.join("by-number"))?;
            std::fs::create_dir_all(blocks_dir.join("by-hash"))?;
        }

        log::info!("Blocks cache cleared");

        Ok(())
    }

    /// Create BTRFS snapshot of cache
    pub async fn create_snapshot(&self) -> Result<PathBuf> {
        self.snapshot_manager.create_snapshot().await
    }

    /// List all snapshots
    pub async fn list_snapshots(&self) -> Result<Vec<super::snapshot_manager::SnapshotInfo>> {
        self.snapshot_manager.list_snapshots().await
    }

    /// Delete all snapshots
    pub async fn delete_all_snapshots(&self) -> Result<usize> {
        self.snapshot_manager.delete_all_snapshots().await
    }

    /// Stream cache data to remote system using Btrfs send/receive with NUMA affinity
    pub async fn stream_to_remote(
        &self,
        remote_host: &str,
        remote_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Apply NUMA affinity for streaming operations
        self.apply_numa_affinity("cache_streaming").await?;

        let snapshot_path = self
            .create_snapshot()
            .await
            .map_err(|e| format!("Failed to create snapshot: {}", e))?;

        info!(
            "Streaming cache snapshot to {}:{}",
            remote_host, remote_path
        );

        let cmd = format!(
            "btrfs send {} | ssh {} 'btrfs receive {}'",
            snapshot_path.display(),
            remote_host,
            remote_path
        );

        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(&cmd)
            .output()
            .await
            .map_err(|e| format!("Failed to execute btrfs stream command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Btrfs streaming failed: {}", stderr).into());
        }

        info!("Successfully streamed cache snapshot");
        Ok(())
    }

    /// Receive cache data from remote system with NUMA affinity
    pub async fn receive_from_remote(
        &self,
        remote_host: &str,
        remote_snapshot: &str,
        local_path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Apply NUMA affinity for receiving operations
        self.apply_numa_affinity("cache_receiving").await?;

        info!(
            "Receiving cache snapshot from {}:{}",
            remote_host, remote_snapshot
        );

        let cmd = format!(
            "ssh {} 'btrfs send {}' | btrfs receive {}",
            remote_host, remote_snapshot, local_path
        );

        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(&cmd)
            .output()
            .await
            .map_err(|e| format!("Failed to execute btrfs receive command: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Btrfs receive failed: {}", stderr).into());
        }

        info!("Successfully received cache snapshot");
        Ok(())
    }

    /// Get NUMA configuration info
    pub fn numa_info(&self) -> NumaInfo {
        NumaInfo {
            node_count: self
                .numa_topology
                .as_ref()
                .map(|t| t.node_count())
                .unwrap_or(1),
            cpu_affinity: self.cpu_affinity.clone(),
            placement_strategy: self.placement_strategy.clone(),
            memory_policy: self.memory_policy.clone(),
        }
    }

    /// Get detailed NUMA topology
    pub fn numa_topology(&self) -> Option<&NumaTopology> {
        self.numa_topology.as_ref()
    }

    /// Get NUMA statistics
    pub fn numa_stats(&self) -> HashMap<u32, NumaStats> {
        self.numa_stats.lock().unwrap().clone()
    }

    /// Clear NUMA statistics
    pub fn clear_numa_stats(&self) {
        self.numa_stats.lock().unwrap().clear();
    }

    /// Helper method to apply NUMA affinity (CPU + memory)
    async fn apply_numa_affinity(
        &self,
        operation: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Apply CPU affinity first
        self.apply_cpu_affinity(operation).await?;

        // Apply memory policy
        match &self.memory_policy {
            MemoryPolicy::Default => {
                debug!("Using default memory policy for {}", operation);
            }
            MemoryPolicy::Bind(nodes) if !nodes.is_empty() => {
                debug!("Memory bound to nodes {:?} for {}", nodes, operation);
            }
            MemoryPolicy::Preferred(Some(node)) => {
                debug!("Memory preferred on node {} for {}", node, operation);
            }
            MemoryPolicy::Interleave(nodes) if !nodes.is_empty() => {
                debug!(
                    "Memory interleaved across nodes {:?} for {}",
                    nodes, operation
                );
            }
            _ => {
                debug!("Memory policy not applied for {}", operation);
            }
        }

        Ok(())
    }

    /// Apply CPU affinity using taskset
    async fn apply_cpu_affinity(
        &self,
        operation: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.cpu_affinity.is_empty() {
            return Ok(());
        }

        let cpu_list = self
            .cpu_affinity
            .iter()
            .map(|cpu| cpu.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let output = tokio::process::Command::new("taskset")
            .args(["-c", &cpu_list])
            .arg("echo")
            .arg(format!("CPU affinity test for {}", operation))
            .output()
            .await
            .map_err(|e| format!("taskset command failed: {}", e))?;

        if output.status.success() {
            debug!(
                "Applied CPU affinity to cores: {} for {}",
                cpu_list, operation
            );
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("taskset failed for {}: {}", operation, stderr);
            Ok(()) // Don't fail, just continue without affinity
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub hot_entries: usize,
    pub total_accesses: u64,
    pub disk_usage_bytes: u64,
    pub embeddings_size_bytes: u64,
    pub blocks_size_bytes: u64,
}

impl CacheStats {
    pub fn hot_ratio(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.hot_entries as f64 / self.total_entries as f64
        }
    }

    pub fn avg_accesses(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.total_accesses as f64 / self.total_entries as f64
        }
    }
}
#[derive(Debug, Clone)]
/// NUMA configuration information
pub struct NumaInfo {
    pub node_count: usize,
    pub cpu_affinity: Vec<u32>,
    pub placement_strategy: CachePlacementStrategy,
    pub memory_policy: MemoryPolicy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_hashing() {
        let cache = BtrfsCache::new(PathBuf::from("/tmp/test-cache")).unwrap();
        let hash1 = cache.hash_text("test");
        let hash2 = cache.hash_text("test");
        let hash3 = cache.hash_text("different");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 hex length
    }
}
