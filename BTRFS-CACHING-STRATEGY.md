# BTRFS-Native Caching Strategy

Leveraging BTRFS subvolumes, snapshots, and compression for unlimited cache with instant retrieval.

## The BTRFS Advantage

### Why BTRFS is Perfect for Caching

**1. Copy-on-Write (CoW)**
- Instant snapshots (no data duplication)
- Deduplicated blocks (common data stored once)
- Nearly instant subvolume clones

**2. Built-in Compression**
- Transparent compression (zstd, lzo, zlib)
- 3-5x compression ratio for JSON/text
- Automatic decompression on read

**3. Subvolume Isolation**
- Independent mount points
- Separate snapshot policies
- Easy cleanup (delete entire subvolume)

**4. Page Cache Integration**
- Linux kernel page cache works perfectly
- Recently accessed files stay in RAM
- No explicit in-memory cache needed

**5. SSD-Optimized**
- SSD-specific optimizations
- TRIM support
- Discard for freed space

## Architecture: BTRFS Cache Layer

### Subvolume Structure

```
/var/lib/op-dbus/ (BTRFS filesystem root)
│
├─ @blockchain/                     # Main blockchain (already exists)
│  ├─ timing/                       # Timing data
│  └─ vectors/                      # Vector embeddings
│
├─ @cache/                          # NEW: Cache subvolume
│  ├─ embeddings/                   # Vector embedding cache
│  │  ├─ index.db                   # SQLite index for lookups
│  │  └─ vectors/
│  │     ├─ sha256-hash-1.vec       # Binary vector data
│  │     ├─ sha256-hash-2.vec
│  │     └─ ...
│  │
│  ├─ blocks/                       # Cached block data
│  │  ├─ by-number/
│  │  │  ├─ 0000000.json           # Block 0 (genesis)
│  │  │  ├─ 0000001.json
│  │  │  └─ ...
│  │  └─ by-hash/                   # Symlinks to by-number
│  │     ├─ a1b2c3...json -> ../by-number/0000042.json
│  │     └─ ...
│  │
│  ├─ queries/                      # Query result cache
│  │  ├─ net/                       # Per-plugin caches
│  │  │  └─ latest.json             # Latest query result
│  │  ├─ lxc/
│  │  └─ systemd/
│  │
│  └─ diffs/                        # Diff computation cache
│     ├─ current-hash_desired-hash.json
│     └─ ...
│
└─ @cache-snapshots/                # Snapshot subvolume for cache
   ├─ cache@2025-01-15-10:00
   ├─ cache@2025-01-15-11:00
   └─ ...
```

### BTRFS Properties

```bash
# Create cache subvolume with compression
btrfs subvolume create /var/lib/op-dbus/@cache
btrfs property set /var/lib/op-dbus/@cache compression zstd

# Set up automatic snapshots (optional)
btrfs subvolume snapshot /var/lib/op-dbus/@cache \
    /var/lib/op-dbus/@cache-snapshots/cache@$(date +%Y-%m-%d-%H:%M)
```

## Implementation: Hybrid BTRFS + Page Cache

### Strategy: "Infinite" Disk Cache + Kernel Page Cache

**The Magic:**
1. Store ALL cache data in BTRFS subvolume (unlimited size)
2. BTRFS compresses data automatically (3-5x compression)
3. Linux page cache keeps hot data in RAM
4. No need for explicit LRU eviction - kernel handles it!

### 1. Vector Embedding Cache (BTRFS-backed)

```rust
// src/ml/btrfs_embedding_cache.rs

use anyhow::Result;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};

pub struct BtrfsEmbeddingCache {
    cache_dir: PathBuf,
    index: rusqlite::Connection,
}

impl BtrfsEmbeddingCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        // SQLite index for fast lookups
        let index_path = cache_dir.join("index.db");
        let index = rusqlite::Connection::open(&index_path)?;

        // Create index table
        index.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                text_hash TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                vector_file TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                accessed_at INTEGER NOT NULL,
                access_count INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        // Create index for hot/cold data analysis
        index.execute(
            "CREATE INDEX IF NOT EXISTS idx_accessed
             ON embeddings(accessed_at DESC)",
            [],
        )?;

        Ok(Self { cache_dir, index })
    }

    /// Get or compute embedding
    pub fn get_or_embed<F>(&mut self, text: &str, compute_fn: F) -> Result<Vec<f32>>
    where
        F: FnOnce(&str) -> Result<Vec<f32>>,
    {
        let text_hash = self.hash_text(text);

        // Check if cached
        if let Some(vector) = self.load_from_btrfs(&text_hash)? {
            // Update access statistics
            self.update_access(&text_hash)?;
            return Ok(vector);
        }

        // Compute embedding
        let vector = compute_fn(text)?;

        // Store in BTRFS cache
        self.save_to_btrfs(text, &text_hash, &vector)?;

        Ok(vector)
    }

    fn hash_text(&self, text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn load_from_btrfs(&self, text_hash: &str) -> Result<Option<Vec<f32>>> {
        // Lookup in SQLite index
        let vector_file: Option<String> = self.index
            .query_row(
                "SELECT vector_file FROM embeddings WHERE text_hash = ?1",
                [text_hash],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(file) = vector_file {
            let path = self.cache_dir.join("vectors").join(&file);

            // Read from BTRFS (automatically decompressed by kernel)
            // Linux page cache will keep hot files in RAM!
            let data = std::fs::read(&path)?;
            let vector: Vec<f32> = bincode::deserialize(&data)?;

            return Ok(Some(vector));
        }

        Ok(None)
    }

    fn save_to_btrfs(&mut self, text: &str, text_hash: &str, vector: &[f32]) -> Result<()> {
        let vectors_dir = self.cache_dir.join("vectors");
        std::fs::create_dir_all(&vectors_dir)?;

        let vector_file = format!("{}.vec", text_hash);
        let path = vectors_dir.join(&vector_file);

        // Write to BTRFS (automatically compressed by kernel)
        let data = bincode::serialize(vector)?;
        std::fs::write(&path, data)?;

        // Add to SQLite index
        let now = chrono::Utc::now().timestamp();
        self.index.execute(
            "INSERT INTO embeddings (text_hash, text, vector_file, created_at, accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![text_hash, text, vector_file, now, now],
        )?;

        Ok(())
    }

    fn update_access(&self, text_hash: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.index.execute(
            "UPDATE embeddings
             SET accessed_at = ?1, access_count = access_count + 1
             WHERE text_hash = ?2",
            rusqlite::params![now, text_hash],
        )?;
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let total: i64 = self.index.query_row(
            "SELECT COUNT(*) FROM embeddings",
            [],
            |row| row.get(0),
        )?;

        let hot_threshold = chrono::Utc::now().timestamp() - 3600; // 1 hour
        let hot: i64 = self.index.query_row(
            "SELECT COUNT(*) FROM embeddings WHERE accessed_at > ?1",
            [hot_threshold],
            |row| row.get(0),
        )?;

        // Calculate disk usage (BTRFS reports compressed size)
        let vectors_dir = self.cache_dir.join("vectors");
        let disk_usage = Self::dir_size(&vectors_dir)?;

        Ok(CacheStats {
            total_entries: total as usize,
            hot_entries: hot as usize,
            disk_usage_bytes: disk_usage,
        })
    }

    fn dir_size(path: &Path) -> Result<u64> {
        let mut size = 0u64;
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                size += metadata.len();
            }
        }
        Ok(size)
    }

    /// Clean old entries (optional - only if disk space is critical)
    pub fn cleanup_old(&mut self, days: i64) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp() - (days * 86400);

        // Find old entries
        let mut stmt = self.index.prepare(
            "SELECT text_hash, vector_file FROM embeddings
             WHERE accessed_at < ?1"
        )?;

        let old_entries: Vec<(String, String)> = stmt
            .query_map([cutoff], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let count = old_entries.len();

        // Delete files
        for (_hash, file) in &old_entries {
            let path = self.cache_dir.join("vectors").join(file);
            let _ = std::fs::remove_file(path); // Ignore errors
        }

        // Delete from index
        self.index.execute(
            "DELETE FROM embeddings WHERE accessed_at < ?1",
            [cutoff],
        )?;

        Ok(count)
    }
}

pub struct CacheStats {
    pub total_entries: usize,
    pub hot_entries: usize,
    pub disk_usage_bytes: u64,
}
```

### 2. Blockchain Block Cache (BTRFS-backed)

```rust
// src/blockchain/btrfs_block_cache.rs

pub struct BtrfsBlockCache {
    cache_dir: PathBuf,
}

impl BtrfsBlockCache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let by_number = cache_dir.join("by-number");
        let by_hash = cache_dir.join("by-hash");
        std::fs::create_dir_all(&by_number)?;
        std::fs::create_dir_all(&by_hash)?;
        Ok(Self { cache_dir })
    }

    pub fn get_by_number(&self, block_num: u64) -> Result<Option<BlockEvent>> {
        let path = self.cache_dir
            .join("by-number")
            .join(format!("{:07}.json", block_num));

        if !path.exists() {
            return Ok(None);
        }

        // Read from BTRFS (page cache will keep hot blocks in RAM)
        let data = std::fs::read_to_string(&path)?;
        let block: BlockEvent = serde_json::from_str(&data)?;
        Ok(Some(block))
    }

    pub fn put(&self, block_num: u64, block: &BlockEvent) -> Result<()> {
        let by_number = self.cache_dir.join("by-number");
        let path = by_number.join(format!("{:07}.json", block_num));

        // Write to BTRFS (automatically compressed)
        let json = serde_json::to_string_pretty(&block)?;
        std::fs::write(&path, json)?;

        // Create symlink by hash
        let by_hash = self.cache_dir.join("by-hash");
        let hash_link = by_hash.join(format!("{}.json", block.hash));
        let _ = std::os::unix::fs::symlink(&path, &hash_link); // Ignore if exists

        Ok(())
    }

    pub fn get_by_hash(&self, hash: &str) -> Result<Option<BlockEvent>> {
        let path = self.cache_dir
            .join("by-hash")
            .join(format!("{}.json", hash));

        if !path.exists() {
            return Ok(None);
        }

        // Follow symlink and read
        let data = std::fs::read_to_string(&path)?;
        let block: BlockEvent = serde_json::from_str(&data)?;
        Ok(Some(block))
    }
}
```

## Performance Characteristics

### BTRFS + Page Cache Magic

**First Access (Cold):**
```
1. Check SQLite index (fast, likely in page cache)
2. Read from BTRFS (disk I/O)
3. BTRFS decompresses on-the-fly
4. Kernel loads into page cache
Time: ~5ms (SSD) to ~20ms (HDD)
```

**Second Access (Warm - in page cache):**
```
1. Check SQLite index (page cache hit)
2. Read from "BTRFS" (actually page cache)
3. No disk I/O!
4. No decompression!
Time: ~0.1ms (RAM speed)
```

**Third+ Access (Hot - stays in page cache):**
```
Same as second access
Time: ~0.1ms
```

### Compression Benefits

**Vector Embeddings:**
```
Uncompressed: 384 floats × 4 bytes = 1,536 bytes per vector
ZSTD compressed: ~400-600 bytes (2.5-4x compression)

10,000 vectors:
- Uncompressed: 15 MB
- Compressed: 4-6 MB
- Savings: 9-11 MB (60-70% reduction)
```

**JSON Blocks:**
```
Typical block: ~10 KB JSON
ZSTD compressed: ~2-3 KB (3-5x compression)

1,000 blocks:
- Uncompressed: 10 MB
- Compressed: 2-3 MB
- Savings: 7-8 MB (70-80% reduction)
```

### Comparison: BTRFS vs In-Memory LRU

| Aspect | BTRFS Cache | In-Memory LRU |
|--------|-------------|---------------|
| **Capacity** | Unlimited (disk size) | Limited (RAM) |
| **Hot data speed** | ~0.1ms (page cache) | ~0.05ms (RAM) |
| **Cold data speed** | ~5ms (SSD read) | N/A (evicted) |
| **Persistence** | Survives restarts | Lost on restart |
| **Eviction** | Kernel manages | Manual LRU logic |
| **Compression** | Automatic (3-5x) | None |
| **Complexity** | Simple (filesystem) | Complex (LRU impl) |
| **Memory usage** | Adaptive (page cache) | Fixed (LRU size) |

## Hybrid Strategy: Best of Both Worlds

### Recommended Architecture

```
┌─────────────────────────────────────────────────┐
│ Application Layer                               │
├─────────────────────────────────────────────────┤
│                                                 │
│  Small Hot Cache (Optional)                     │
│  - Last 100 queries in HashMap                  │
│  - Instant lookup (<0.01ms)                     │
│  - No persistence needed                        │
│       ↓ (on miss)                               │
│                                                 │
│  BTRFS Cache Layer                              │
│  - Unlimited capacity                           │
│  - SQLite index for lookups                     │
│  - BTRFS subvolume with zstd compression        │
│       ↓ (on disk)                               │
│                                                 │
└─────────────────────────────────────────────────┘
         ↕ (automatic)
┌─────────────────────────────────────────────────┐
│ Linux Kernel Page Cache                         │
│ - Hot data stays in RAM (free!)                │
│ - Kernel LRU eviction (smart!)                 │
│ - No manual management needed                  │
└─────────────────────────────────────────────────┘
```

### Why This is Better

**1. Unlimited Cache**
- Store every embedding ever computed
- Store every block ever created
- Never worry about eviction
- Disk is cheap!

**2. Automatic Memory Management**
- Kernel page cache uses available RAM
- Hot data naturally stays in memory
- Cold data on disk, no manual eviction
- Perfect LRU behavior for free

**3. Compression Wins**
- BTRFS compresses transparently
- 3-5x space savings
- Decompression is fast (zstd)
- More cache fits on disk

**4. Persistence**
- Cache survives restarts
- No "cold start" problem
- Historical data always available

**5. Simplicity**
- No LRU implementation needed
- No complex eviction policies
- Kernel handles everything
- Just write files!

## Installation Integration

Update `install.sh` to create cache subvolume:

```bash
# In install.sh (BTRFS section)

if df -T /var/lib 2>/dev/null | grep -q btrfs; then
    echo "Setting up BTRFS cache subvolume..."

    CACHE_SUBVOL="/var/lib/op-dbus/@cache"

    # Create cache subvolume if doesn't exist
    if ! sudo btrfs subvolume show "$CACHE_SUBVOL" >/dev/null 2>&1; then
        sudo btrfs subvolume create "$CACHE_SUBVOL"
        echo "✓ Created cache subvolume"
    fi

    # Enable zstd compression for cache
    sudo btrfs property set "$CACHE_SUBVOL" compression zstd
    echo "✓ Enabled ZSTD compression on cache subvolume"

    # Set up cache directory structure
    sudo mkdir -p "$CACHE_SUBVOL"/{embeddings,blocks/{by-number,by-hash},queries,diffs}
    echo "✓ Created cache directory structure"
fi
```

## CLI Commands

```bash
# Show cache statistics
op-dbus cache stats

# Output:
# BTRFS Cache Statistics:
#
# Embeddings:
#   Total entries: 45,234
#   Hot (< 1 hour): 1,234
#   Disk usage: 18.2 MB (compressed)
#   Uncompressed: ~70 MB (4.2x compression)
#
# Blocks:
#   Total blocks: 2,341
#   Disk usage: 4.8 MB (compressed)
#   Uncompressed: ~23 MB (4.8x compression)
#
# Total cache size: 23 MB (compressed)
# Total saved space: ~70 MB (3.8x avg compression)

# Clean old entries (optional, rarely needed)
op-dbus cache clean --older-than 90d

# BTRFS snapshot of cache (for backup)
op-dbus cache snapshot

# Output:
# ✓ Created snapshot: @cache-snapshots/cache@2025-01-15-14:30
```

## Configuration

```bash
# Environment variables
OPDBUS_CACHE_COMPRESSION=zstd    # zstd, lzo, zlib, or none
OPDBUS_CACHE_SUBVOL=/var/lib/op-dbus/@cache

# Optional: auto-cleanup
OPDBUS_CACHE_CLEANUP_DAYS=90     # Delete entries older than 90 days
```

## Performance Expectations

### Real-World Numbers (SSD)

**Cold start (after reboot):**
```
First embedding lookup:  ~5ms  (disk read + decompress)
Second embedding:        ~0.1ms (page cache hit)
Block lookup:            ~5ms  (first) → ~0.1ms (cached)
```

**Hot system (normal operation):**
```
Embedding lookup:  ~0.1ms (page cache)
Block lookup:      ~0.1ms (page cache)
Query result:      ~0.1ms (page cache)

Effectively as fast as in-memory cache!
```

### Storage Efficiency

**10,000 embeddings:**
- Uncompressed: 15 MB
- ZSTD compressed: 4-6 MB
- Savings: 9-11 MB

**10,000 blocks:**
- Uncompressed: 100 MB
- ZSTD compressed: 20-30 MB
- Savings: 70-80 MB

**Total for 10K of each:**
- Uncompressed: 115 MB
- Compressed: 24-36 MB
- Actual disk usage: ~30 MB
- **You can cache 100K+ entries easily!**

## Advantages Over Pure In-Memory

1. **Unlimited capacity** - only limited by disk
2. **Persistence** - survives restarts
3. **Automatic** - kernel handles hot/cold
4. **Compressed** - 3-5x more fits on disk
5. **Simple** - just write files
6. **Fast** - page cache keeps hot data in RAM
7. **Historical** - never lose old data

## Summary

**BTRFS caching strategy:**
- ✅ Unlimited cache capacity (disk-limited)
- ✅ Automatic compression (3-5x space savings)
- ✅ Linux page cache keeps hot data in RAM
- ✅ ~0.1ms for hot data (RAM speed)
- ✅ ~5ms for cold data (SSD read)
- ✅ No manual eviction needed
- ✅ Persistent across restarts
- ✅ Simple filesystem-based implementation

**Perfect for op-dbus because:**
- Server has plenty of disk space
- BTRFS already used for blockchain
- Natural fit with existing architecture
- Simpler than complex LRU caching
- Better performance characteristics

**Next step:** Implement BTRFS-backed caching for embeddings and blocks!
