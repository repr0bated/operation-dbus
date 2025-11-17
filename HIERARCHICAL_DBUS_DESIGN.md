# Hierarchical D-Bus Abstraction Layer in BTRFS

## Problem Statement

D-Bus introspection is:
- **Fragmented**: No single view of the entire tree
- **Lazy**: Must recursively walk to discover objects
- **Slow**: Network calls for each object path
- **Incomplete**: Artificial limits (100 objects) and silent failures
- **Ephemeral**: Re-scanned on every query

This makes it impossible for AI/chatbots to have **complete system context**.

## Solution Architecture

### 1. BTRFS Subvolume Structure

```
/var/lib/op-dbus/
├── @dbus-index/              ← BTRFS subvolume (snapshotable)
│   ├── system/               ← System bus introspection
│   │   ├── services.json     ← Service → Object mapping
│   │   ├── hierarchy.db      ← SQLite full-text search
│   │   ├── org.freedesktop.systemd1/
│   │   │   ├── metadata.json
│   │   │   ├── units/        ← All 500+ systemd units
│   │   │   └── tree.json     ← Complete object hierarchy
│   │   ├── org.freedesktop.NetworkManager/
│   │   └── ...
│   └── session/              ← Session bus introspection
│       └── ...
├── @snapshots/               ← Historical D-Bus states
│   ├── 2025-11-16-pre-upgrade/
│   └── 2025-11-16-working/
└── @config/                  ← Configuration (from earlier design)
```

### 2. Index Generation Process

```rust
// src/mcp/dbus_indexer.rs

pub struct DbusIndexer {
    system_bus: Connection,
    index_path: PathBuf,  // /var/lib/op-dbus/@dbus-index
}

impl DbusIndexer {
    /// Full system scan - runs once, saves to BTRFS
    pub async fn build_complete_index(&self) -> Result<IndexStats> {
        // 1. Discover ALL services (no limits)
        let services = self.discover_all_services().await?;

        // 2. For each service, discover ALL objects (recursive, no limit)
        for service in services {
            let objects = self.discover_all_objects_unlimited(&service).await?;

            // 3. For each object, introspect ALL interfaces
            for object in objects {
                let interfaces = self.introspect_complete(&service, &object).await?;

                // 4. Write to hierarchical file structure
                self.write_to_index(&service, &object, &interfaces)?;
            }
        }

        // 5. Build SQLite FTS index for fast queries
        self.build_search_index()?;

        // 6. Generate metadata (service graph, dependency map)
        self.generate_metadata()?;

        Ok(stats)
    }

    /// Incremental update - only changed services
    pub async fn update_index(&self) -> Result<()> {
        let current = self.list_services().await?;
        let cached = self.load_cached_services()?;

        // Only re-scan services that changed
        let diff = current.difference(&cached);
        for service in diff {
            self.reindex_service(service).await?;
        }
    }
}
```

### 3. Query Interface for AI

```rust
// src/mcp/dbus_query.rs

pub struct DbusQueryEngine {
    index_path: PathBuf,
    db: SqliteConnection,
}

impl DbusQueryEngine {
    /// Fast lookup - no D-Bus calls!
    pub fn find_object(&self, query: &str) -> Result<Vec<DbusObject>> {
        // Full-text search on cached index
        self.db.query(
            "SELECT * FROM dbus_objects WHERE path LIKE ? OR name LIKE ?",
            &[query, query]
        )
    }

    /// Get complete service hierarchy
    pub fn get_service_tree(&self, service: &str) -> Result<ServiceTree> {
        // Load pre-built tree from JSON
        let path = self.index_path.join("system").join(service).join("tree.json");
        serde_json::from_reader(File::open(path)?)
    }

    /// List all methods on an object (instant, no D-Bus call)
    pub fn list_methods(&self, service: &str, object: &str) -> Result<Vec<Method>> {
        self.db.query(
            "SELECT * FROM methods WHERE service = ? AND object = ?",
            &[service, object]
        )
    }
}
```

### 4. AI Context Provider Integration

```rust
// src/mcp/ai_context_provider.rs (enhanced)

impl AiContextProvider {
    pub async fn get_complete_dbus_context(&self) -> Result<Value> {
        // BEFORE: Scanned D-Bus live (slow, incomplete)
        // AFTER: Read from BTRFS index (fast, complete)

        let query = DbusQueryEngine::new("/var/lib/op-dbus/@dbus-index")?;

        json!({
            "dbus": {
                "total_services": query.count_services()?,
                "total_objects": query.count_objects()?,  // ALL objects, not just 100
                "total_methods": query.count_methods()?,
                "service_tree": query.get_all_service_trees()?,
                "search_available": true  // Can search by name, path, interface
            }
        })
    }

    /// AI can search: "Find all systemd units containing 'ssh'"
    pub async fn search_dbus(&self, query: &str) -> Result<Vec<DbusObject>> {
        let engine = DbusQueryEngine::new("/var/lib/op-dbus/@dbus-index")?;
        engine.find_object(query)  // Instant SQLite FTS
    }
}
```

### 5. BTRFS Snapshot Workflow

```bash
# Initial index build (one-time, slow)
op-dbus index build --output /var/lib/op-dbus/@dbus-index

# Snapshot working state
btrfs subvolume snapshot /var/lib/op-dbus/@dbus-index \
    /var/lib/op-dbus/@snapshots/$(date +%F)-baseline

# After system changes
systemctl start new-service
op-dbus index update  # Fast incremental

# Compare D-Bus state before/after
op-dbus index diff \
    /var/lib/op-dbus/@snapshots/2025-11-16-baseline \
    /var/lib/op-dbus/@dbus-index

# Rollback to known-good D-Bus state
mv /var/lib/op-dbus/@dbus-index /var/lib/op-dbus/@dbus-index.broken
btrfs subvolume snapshot /var/lib/op-dbus/@snapshots/2025-11-16-baseline \
    /var/lib/op-dbus/@dbus-index
```

## Benefits

### For AI/Chatbot

✅ **Complete Context**: Sees ALL services, objects, methods (not limited to 100)
✅ **Fast Queries**: SQLite FTS instead of recursive D-Bus calls
✅ **Reliable**: Doesn't fail silently on errors
✅ **Searchable**: "Find all units with 'network' in the name"
✅ **Consistent**: Same view every time (until you update)

### For System Management

✅ **Snapshot D-Bus State**: Before/after system changes
✅ **Diff Tool**: See exactly what changed
✅ **Rollback**: Restore known-good D-Bus configuration
✅ **Share**: `btrfs send` index to other nodes
✅ **Audit**: Historical record of D-Bus changes

### For Performance

✅ **One-time Cost**: Slow full scan happens once
✅ **Incremental Updates**: Only re-scan changed services
✅ **No Runtime D-Bus Load**: Queries hit disk, not D-Bus
✅ **Parallel Queries**: Multiple AI requests don't overwhelm D-Bus

## Implementation Plan

### Phase 1: Basic Indexer (1-2 days)

- [ ] Remove 100-object limit from system_introspection.rs
- [ ] Add unlimited recursive discovery
- [ ] Write results to JSON files in BTRFS subvolume
- [ ] Create subvolume at `/var/lib/op-dbus/@dbus-index`

### Phase 2: Search Engine (1 day)

- [ ] Create SQLite schema for FTS
- [ ] Index all services/objects/methods
- [ ] Add query interface

### Phase 3: AI Integration (1 day)

- [ ] Update AiContextProvider to use index
- [ ] Add search tools for chatbot
- [ ] Test with DeepSeek integration

### Phase 4: Management Tools (1 day)

- [ ] `op-dbus index build` command
- [ ] `op-dbus index update` command
- [ ] `op-dbus index diff` command
- [ ] `op-dbus index search <query>` command

## Example Usage

```bash
# Build initial index (run once on golden master)
op-dbus index build
# [████████████████] 100% - Indexed 1,247 services, 18,432 objects

# AI can now query instantly
curl http://localhost:8080/api/chat -d '{
  "message": "List all systemd units related to networking"
}'
# → AI searches index, returns ALL matching units (not just first 100)

# After installing new package
apt install nginx
systemctl start nginx

# Update index (fast - only scans new services)
op-dbus index update
# [██████] Updated 1 service (nginx.service)

# Snapshot before making changes
btrfs subvolume snapshot /var/lib/op-dbus/@dbus-index \
    /var/lib/op-dbus/@snapshots/pre-nginx-config

# Make changes...
systemctl edit nginx

# Compare what changed
op-dbus index diff @snapshots/pre-nginx-config @dbus-index
# Changed: org.freedesktop.systemd1 /org/freedesktop/systemd1/unit/nginx_2eservice
#   Property: ActiveState = "active" (was "inactive")
```

## Why BTRFS?

1. **Snapshots**: Free copy-on-write snapshots of entire index
2. **Compression**: D-Bus XML is highly compressible (zstd)
3. **Deduplication**: Similar introspection XMLs share blocks
4. **Send/Receive**: Ship index to other nodes
5. **Atomic**: Snapshot is atomic, not partial

## Storage Requirements

**Estimated size:**
- Full system D-Bus index: ~50-100 MB (compressed)
- Per-service metadata: ~100 KB
- SQLite FTS index: ~20 MB
- **Total: ~150 MB** (trivial on modern systems)

## Alternative: Why Not Just SQLite?

You COULD put everything in SQLite without BTRFS, but you lose:
- ❌ Snapshot/rollback capability
- ❌ Easy send/receive to other nodes
- ❌ Hierarchical file organization (harder to browse)
- ❌ Compression (SQLite doesn't compress)

BTRFS gives you **both** structure AND snapshotability.

## Conclusion

This design solves the fundamental problem: **D-Bus introspection is incomplete and ephemeral**.

By building a **complete, persistent, searchable index in a BTRFS subvolume**, you give the AI:
- Full system visibility
- Fast queries
- Consistent results
- Historical context

And you get operational benefits:
- D-Bus state snapshots
- Change tracking
- Rollback capability
- Cross-node sharing

**This is exactly what your system needs to make the AI truly effective.**
