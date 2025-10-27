//! BTRFS-backed cache with SQLite index and compression
//!
//! Provides unlimited disk-based caching with:
//! - BTRFS transparent compression (zstd)
//! - SQLite index for O(1) lookups
//! - Linux page cache for hot data
//! - Automatic snapshot management

use anyhow::{Context, Result};
use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::snapshot_manager::{SnapshotConfig, SnapshotManager};

pub struct BtrfsCache {
    cache_dir: PathBuf,
    index: Mutex<rusqlite::Connection>,
    snapshot_manager: SnapshotManager,
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

        Ok(Self {
            cache_dir,
            index: Mutex::new(index),
            snapshot_manager,
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
            let data = std::fs::read(&path)
                .context(format!("Failed to read cached embedding: {:?}", path))?;

            let vector: Vec<f32> =
                bincode::deserialize(&data).context("Failed to deserialize cached embedding")?;

            return Ok(Some(vector));
        }

        Ok(None)
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
