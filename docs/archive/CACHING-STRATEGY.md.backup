# op-dbus Caching Strategy

Complete caching architecture for blockchain, vectorization, and memory optimization.

## Current State Analysis

### What We Have ✅

**1. Model Download Cache** (implemented)
- Location: `/var/lib/op-dbus/models/`
- What: Downloaded ONNX models from Hugging Face
- Strategy: Check if model exists locally before downloading
- Persistence: Disk-based, survives restarts
- Code: `src/ml/downloader.rs`

**2. Lazy Model Loading** (implemented)
- Strategy: `OnceCell` for singleton model instance
- What: Load model only on first vectorization request
- Memory: Model stays in RAM once loaded
- Code: `src/ml/model_manager.rs`

### What We DON'T Have ❌

**1. Blockchain Block Cache**
- Currently: Every query reads from disk
- Problem: Slow for repeated queries
- Missing: LRU cache for recent blocks

**2. Vector Embedding Cache**
- Currently: Re-vectorize same text every time
- Problem: Wasteful for common strings
- Missing: Cache for frequently vectorized strings

**3. Query Result Cache**
- Currently: `op-dbus query` always queries all plugins
- Problem: Slow for unchanged state
- Missing: TTL cache for plugin state

**4. Diff Computation Cache**
- Currently: Recompute diff every time
- Problem: Wasteful if state hasn't changed
- Missing: Cache keyed on (current_hash, desired_hash)

## Caching Architecture Design

### Layer 1: In-Memory Caches (Hot Path)

```
┌─────────────────────────────────────────────────┐
│ In-Memory Caches (LRU)                          │
├─────────────────────────────────────────────────┤
│                                                 │
│  1. Vector Embedding Cache                      │
│     Key: String (text to vectorize)             │
│     Value: Vec<f32> (embedding)                 │
│     Size: 10,000 entries (~40MB)               │
│     Hit Rate: ~80% (common strings)             │
│                                                 │
│  2. Blockchain Block Cache                      │
│     Key: u64 (block number)                     │
│     Value: BlockEvent                           │
│     Size: 1,000 blocks (~10MB)                  │
│     Hit Rate: ~90% (recent blocks)              │
│                                                 │
│  3. Plugin State Cache                          │
│     Key: String (plugin name)                   │
│     Value: (Value, timestamp)                   │
│     TTL: 5 seconds                              │
│     Size: ~1MB                                  │
│                                                 │
│  4. Diff Computation Cache                      │
│     Key: (current_hash, desired_hash)           │
│     Value: Vec<StateDiff>                       │
│     TTL: 60 seconds                             │
│     Size: 100 entries (~5MB)                    │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Layer 2: Disk Caches (Warm Path)

```
/var/lib/op-dbus/
├─ models/           # Model download cache (✅ exists)
│  ├─ MiniLM-L3/
│  └─ MPNet/
│
├─ blockchain/       # Blockchain storage (✅ exists)
│  ├─ timing/
│  └─ vectors/
│
└─ cache/            # NEW: Structured cache directory
   ├─ embeddings/    # Persistent vector cache
   │  └─ *.vec      # LRU eviction policy
   ├─ blocks/        # Block metadata cache
   │  └─ *.json     # Indexed by block number
   └─ queries/       # Query result cache
      └─ *.json     # TTL-based eviction
```

## Implementation Strategy

### 1. Vector Embedding Cache

**Problem:** Re-vectorizing common strings wastes GPU/CPU cycles

**Example:**
```rust
// Without cache: 10ms per vectorization
manager.embed("Created container 100");  // 10ms
manager.embed("Created container 101");  // 10ms
manager.embed("Created container 102");  // 10ms
// Total: 30ms

// With cache: First is 10ms, rest are instant
cache.get_or_embed("Created container 100");  // 10ms (cache miss)
cache.get_or_embed("Created container 101");  // <1ms (cache hit on pattern)
cache.get_or_embed("Created container 102");  // <1ms (cache hit on pattern)
// Total: ~12ms (2.5x faster)
```

**Implementation:**

```rust
// src/ml/embedding_cache.rs

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub struct EmbeddingCache {
    cache: Mutex<LruCache<String, Vec<f32>>>,
    disk_cache_dir: PathBuf,
}

impl EmbeddingCache {
    pub fn new(capacity: usize, disk_cache_dir: PathBuf) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap();
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
            disk_cache_dir,
        }
    }

    /// Get embedding from cache or compute it
    pub fn get_or_embed<F>(&self, text: &str, compute_fn: F) -> Result<Vec<f32>>
    where
        F: FnOnce(&str) -> Result<Vec<f32>>,
    {
        // Check in-memory cache
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        // Check disk cache
        if let Some(embedding) = self.load_from_disk(text)? {
            // Promote to in-memory cache
            let mut cache = self.cache.lock().unwrap();
            cache.put(text.to_string(), embedding.clone());
            return Ok(embedding);
        }

        // Compute embedding
        let embedding = compute_fn(text)?;

        // Store in both caches
        self.save_to_disk(text, &embedding)?;
        let mut cache = self.cache.lock().unwrap();
        cache.put(text.to_string(), embedding.clone());

        Ok(embedding)
    }

    fn load_from_disk(&self, text: &str) -> Result<Option<Vec<f32>>> {
        let hash = sha2::Sha256::digest(text.as_bytes());
        let filename = format!("{:x}.vec", hash);
        let path = self.disk_cache_dir.join("embeddings").join(filename);

        if !path.exists() {
            return Ok(None);
        }

        let data = std::fs::read(path)?;
        let embedding: Vec<f32> = bincode::deserialize(&data)?;
        Ok(Some(embedding))
    }

    fn save_to_disk(&self, text: &str, embedding: &[f32]) -> Result<()> {
        let hash = sha2::Sha256::digest(text.as_bytes());
        let filename = format!("{:x}.vec", hash);
        let cache_dir = self.disk_cache_dir.join("embeddings");
        std::fs::create_dir_all(&cache_dir)?;
        let path = cache_dir.join(filename);

        let data = bincode::serialize(embedding)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Clear all caches
    pub fn clear(&self) -> Result<()> {
        self.cache.lock().unwrap().clear();
        let cache_dir = self.disk_cache_dir.join("embeddings");
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.lock().unwrap();
        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
        }
    }
}

pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}
```

**Integration:**

```rust
// src/ml/model_manager.rs

impl ModelManager {
    pub fn embed_cached(&self, text: &str) -> Result<Vec<f32>> {
        if !self.is_enabled() {
            return Ok(Vec::new());
        }

        let cache = Self::embedding_cache();
        cache.get_or_embed(text, |t| {
            let embedder = self.get_or_load_embedder()?;
            embedder.embed(t)
        })
    }

    fn embedding_cache() -> &'static EmbeddingCache {
        static CACHE: OnceCell<EmbeddingCache> = OnceCell::new();
        CACHE.get_or_init(|| {
            let cache_dir = PathBuf::from("/var/lib/op-dbus/cache");
            EmbeddingCache::new(10000, cache_dir)
        })
    }
}
```

### 2. Blockchain Block Cache

**Problem:** Repeated blockchain queries read from disk every time

**Implementation:**

```rust
// src/blockchain/block_cache.rs

use lru::LruCache;
use std::sync::Mutex;

pub struct BlockCache {
    cache: Mutex<LruCache<u64, BlockEvent>>,
}

impl BlockCache {
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap();
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }

    pub fn get(&self, block_num: u64) -> Option<BlockEvent> {
        self.cache.lock().unwrap().get(&block_num).cloned()
    }

    pub fn put(&self, block_num: u64, block: BlockEvent) {
        self.cache.lock().unwrap().put(block_num, block);
    }

    pub fn invalidate(&self, block_num: u64) {
        self.cache.lock().unwrap().pop(&block_num);
    }

    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }
}
```

**Integration with StreamingBlockchain:**

```rust
impl StreamingBlockchain {
    fn block_cache() -> &'static BlockCache {
        static CACHE: OnceCell<BlockCache> = OnceCell::new();
        CACHE.get_or_init(|| BlockCache::new(1000))
    }

    pub async fn get_block(&self, block_num: u64) -> Result<BlockEvent> {
        // Check cache first
        let cache = Self::block_cache();
        if let Some(block) = cache.get(block_num) {
            return Ok(block);
        }

        // Load from disk
        let block = self.load_block_from_disk(block_num).await?;

        // Cache it
        cache.put(block_num, block.clone());

        Ok(block)
    }

    pub async fn add_footprint(&self, footprint: PluginFootprint) -> Result<String> {
        // ... existing code to write to disk ...

        // Invalidate cache for new blocks
        Self::block_cache().invalidate(block_num);

        Ok(hash)
    }
}
```

### 3. Plugin State Cache (TTL-based)

**Problem:** `op-dbus query` is slow when state hasn't changed

**Implementation:**

```rust
// src/state/query_cache.rs

use std::time::{Duration, Instant};
use std::sync::Mutex;
use std::collections::HashMap;

pub struct QueryCache {
    cache: Mutex<HashMap<String, CachedQuery>>,
    ttl: Duration,
}

struct CachedQuery {
    result: serde_json::Value,
    cached_at: Instant,
}

impl QueryCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    pub fn get(&self, plugin: &str) -> Option<serde_json::Value> {
        let cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(plugin) {
            if cached.cached_at.elapsed() < self.ttl {
                return Some(cached.result.clone());
            }
        }
        None
    }

    pub fn put(&self, plugin: String, result: serde_json::Value) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(plugin, CachedQuery {
            result,
            cached_at: Instant::now(),
        });
    }

    pub fn invalidate(&self, plugin: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(plugin);
    }

    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}
```

**Integration with StateManager:**

```rust
impl StateManager {
    fn query_cache() -> &'static QueryCache {
        static CACHE: OnceCell<QueryCache> = OnceCell::new();
        CACHE.get_or_init(|| QueryCache::new(Duration::from_secs(5)))
    }

    pub async fn query_plugin_state(&self, plugin_name: &str) -> Result<Value> {
        // Check cache first
        let cache = Self::query_cache();
        if let Some(cached) = cache.get(plugin_name) {
            log::debug!("Cache hit for plugin: {}", plugin_name);
            return Ok(cached);
        }

        // Query plugin
        let result = self.query_plugin_uncached(plugin_name).await?;

        // Cache result
        cache.put(plugin_name.to_string(), result.clone());

        Ok(result)
    }

    pub async fn apply_state(&self, desired: Value) -> Result<ApplyReport> {
        // ... apply changes ...

        // Invalidate affected plugin caches
        for plugin in affected_plugins {
            Self::query_cache().invalidate(&plugin);
        }

        Ok(report)
    }
}
```

### 4. Diff Computation Cache

**Problem:** Computing diff is expensive, especially for complex states

**Implementation:**

```rust
// src/state/diff_cache.rs

pub struct DiffCache {
    cache: Mutex<LruCache<(String, String), Vec<StateDiff>>>,
}

impl DiffCache {
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap();
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }

    pub fn get(&self, current_hash: &str, desired_hash: &str) -> Option<Vec<StateDiff>> {
        let key = (current_hash.to_string(), desired_hash.to_string());
        self.cache.lock().unwrap().get(&key).cloned()
    }

    pub fn put(&self, current_hash: String, desired_hash: String, diffs: Vec<StateDiff>) {
        let key = (current_hash, desired_hash);
        self.cache.lock().unwrap().put(key, diffs);
    }
}
```

## Cache Configuration

### Environment Variables

```bash
# Vector embedding cache
OPDBUS_EMBEDDING_CACHE_SIZE=10000      # Number of embeddings (default: 10000)
OPDBUS_EMBEDDING_DISK_CACHE=true      # Enable disk cache (default: true)

# Blockchain block cache
OPDBUS_BLOCK_CACHE_SIZE=1000           # Number of blocks (default: 1000)

# Plugin query cache
OPDBUS_QUERY_CACHE_TTL=5               # Seconds (default: 5)

# Diff cache
OPDBUS_DIFF_CACHE_SIZE=100             # Number of diffs (default: 100)

# Global cache directory
OPDBUS_CACHE_DIR=/var/lib/op-dbus/cache  # Default location
```

### CLI Commands for Cache Management

```bash
# Clear all caches
op-dbus cache clear

# Clear specific cache
op-dbus cache clear --embedding
op-dbus cache clear --blocks
op-dbus cache clear --queries

# Show cache statistics
op-dbus cache stats

# Example output:
# Embedding Cache: 8,432 / 10,000 entries (84% full, 42% hit rate)
# Block Cache: 234 / 1,000 blocks (23% full, 91% hit rate)
# Query Cache: 4 / ∞ entries (5s TTL, 67% hit rate)
# Diff Cache: 12 / 100 entries (12% full)
```

## Performance Impact

### Before Caching

```
Query net plugin:          50ms  (OVSDB + Netlink queries)
Query lxc plugin:          30ms  (List containers)
Vectorize "Created...":    10ms  (GPU inference)
Get block 100:             5ms   (Disk read)
Compute diff:              20ms  (Compare states)

Total for repeated query:  115ms
```

### After Caching

```
Query net plugin:          50ms  (first) → 0ms (cached, 5s TTL)
Query lxc plugin:          30ms  (first) → 0ms (cached)
Vectorize "Created...":    10ms  (first) → <1ms (cached)
Get block 100:             5ms   (first) → <1ms (cached LRU)
Compute diff:              20ms  (first) → <1ms (cached)

Total for repeated query:  115ms (first) → ~2ms (cached)
Speedup: 57x faster
```

### Memory Usage Estimates

```
Embedding Cache:  10,000 × 384 floats × 4 bytes = ~15 MB
Block Cache:      1,000 × ~10 KB = ~10 MB
Query Cache:      Variable, ~1 MB typical
Diff Cache:       100 × ~50 KB = ~5 MB

Total: ~31 MB for all caches
```

## Implementation Priority

### Phase 1: High Impact (Implement First)
1. ✅ **Vector Embedding Cache**
   - Huge performance gain for ML features
   - Both in-memory + disk persistence
   - Common patterns cache very well

2. ✅ **Plugin Query Cache**
   - Simple TTL-based
   - Immediate speedup for `op-dbus query`
   - Low memory overhead

### Phase 2: Medium Impact
3. **Blockchain Block Cache**
   - Speeds up blockchain queries
   - Good hit rate for recent blocks

4. **Diff Computation Cache**
   - Speeds up repeated `op-dbus diff`
   - Useful in CI/CD pipelines

### Phase 3: Optimizations
5. Cache warming strategies
6. Predictive pre-caching
7. Cache eviction policies tuning
8. Metrics and monitoring

## Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
lru = "0.12"           # LRU cache implementation
bincode = "1.3"        # Fast binary serialization for disk cache
```

## Monitoring & Observability

```rust
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub capacity: usize,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    pub fn utilization(&self) -> f64 {
        self.size as f64 / self.capacity as f64
    }
}
```

## Summary

**What to implement:**
1. ✅ Vector embedding cache (in-memory + disk)
2. ✅ Plugin query cache (TTL-based)
3. ✅ Blockchain block cache (LRU)
4. ✅ Diff computation cache (LRU)

**Benefits:**
- 50-100x faster for repeated queries
- Reduced GPU/CPU load for vectorization
- Lower disk I/O for blockchain queries
- Better responsiveness

**Memory cost:**
- ~31 MB total for all caches
- Configurable via environment variables
- Acceptable for server use case

**Next step:** Implement Phase 1 (vector embedding + query caches)
