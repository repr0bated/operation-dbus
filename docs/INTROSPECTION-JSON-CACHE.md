# D-Bus Introspection JSON Cache Design

## Problem Statement

**Current Issue**: D-Bus introspection returns XML, but entire codebase uses JSON.

**Anti-Pattern**: Using BTRFS for introspection metadata storage conflates:
- System snapshots (BTRFS strength)
- Metadata indexing (database strength)

**Solution**: Convert XML → JSON once, cache in SQLite with structured tables.

---

## Architecture

### Storage Backend: SQLite with JSON

**Why SQLite (not BTRFS)?**
- ✅ Structured queries on methods/properties/signals
- ✅ Fast indexed lookups by service/interface/method name
- ✅ JSON native support (TEXT column with JSON functions)
- ✅ Single-file deployment
- ✅ Zero configuration
- ✅ Already in dependency tree (`rusqlite`)

**Why not OVSDB?**
- OVSDB designed for network configuration (Open vSwitch)
- Introspection cache is application metadata, not network state
- SQLite simpler for this use case

---

## Database Schema

### Tables

#### 1. `introspection_cache`
Primary storage for full introspection JSON.

```sql
CREATE TABLE introspection_cache (
    service_name TEXT NOT NULL,
    object_path TEXT NOT NULL,
    interface_name TEXT NOT NULL,
    cached_at INTEGER NOT NULL,
    introspection_json TEXT NOT NULL,  -- Full JSON representation
    PRIMARY KEY (service_name, object_path, interface_name)
);
```

#### 2. `service_methods`
Denormalized method lookup for fast queries.

```sql
CREATE TABLE service_methods (
    service_name TEXT NOT NULL,
    interface_name TEXT NOT NULL,
    method_name TEXT NOT NULL,
    signature_json TEXT NOT NULL,  -- {"in_args": [...], "out_args": [...]}
    PRIMARY KEY (service_name, interface_name, method_name)
);
```

#### 3. `service_properties`
Property metadata.

```sql
CREATE TABLE service_properties (
    service_name TEXT NOT NULL,
    interface_name TEXT NOT NULL,
    property_name TEXT NOT NULL,
    type_signature TEXT NOT NULL,  -- e.g., "as", "i", "s"
    access TEXT NOT NULL,           -- "read", "write", "readwrite"
    PRIMARY KEY (service_name, interface_name, property_name)
);
```

#### 4. `service_signals`
Signal definitions.

```sql
CREATE TABLE service_signals (
    service_name TEXT NOT NULL,
    interface_name TEXT NOT NULL,
    signal_name TEXT NOT NULL,
    signature_json TEXT NOT NULL,  -- {"args": [...]}
    PRIMARY KEY (service_name, interface_name, signal_name)
);
```

---

## XML → JSON Conversion

### Input (D-Bus XML)
```xml
<node>
    <interface name="org.freedesktop.systemd1.Manager">
        <method name="StartUnit">
            <arg name="name" type="s" direction="in"/>
            <arg name="mode" type="s" direction="in"/>
            <arg name="job" type="o" direction="out"/>
        </method>
        <property name="Version" type="s" access="read"/>
        <signal name="UnitNew">
            <arg name="id" type="s"/>
            <arg name="unit" type="o"/>
        </signal>
    </interface>
</node>
```

### Output (JSON)
```json
{
  "interfaces": [
    {
      "name": "org.freedesktop.systemd1.Manager",
      "methods": [
        {
          "name": "StartUnit",
          "args": [
            {"name": "name", "type": "s", "direction": "in"},
            {"name": "mode", "type": "s", "direction": "in"},
            {"name": "job", "type": "o", "direction": "out"}
          ]
        }
      ],
      "properties": [
        {"name": "Version", "type": "s", "access": "read"}
      ],
      "signals": [
        {
          "name": "UnitNew",
          "args": [
            {"name": "id", "type": "s"},
            {"name": "unit", "type": "o"}
          ]
        }
      ]
    }
  ],
  "nodes": []
}
```

---

## API Usage

### Rust API

```rust
use crate::mcp::introspection_cache::IntrospectionCache;

// Initialize cache
let cache = IntrospectionCache::new("/var/cache/dbus-introspection.db")?;

// Store introspection (converts XML → JSON automatically)
let xml_data = introspect_service("org.freedesktop.systemd1", "/")?;
cache.store_introspection("org.freedesktop.systemd1", "/", &xml_data)?;

// Retrieve as JSON
let json = cache.get_introspection_json(
    "org.freedesktop.systemd1",
    "/",
    Some("org.freedesktop.systemd1.Manager")
)?;

// Fast method lookup
let methods = cache.get_methods_json(
    "org.freedesktop.systemd1",
    "org.freedesktop.systemd1.Manager"
)?;

// Search methods across all services
let results = cache.search_methods("Start")?;
// Returns: [{"service": "org.freedesktop.systemd1", "method": "StartUnit", ...}, ...]

// Statistics
let stats = cache.get_stats()?;
// {"services": 42, "interfaces": 156, "methods": 1024, ...}
```

### D-Bus Integration

```rust
// In dbus_indexer.rs or similar
let cache = IntrospectionCache::new("/var/cache/dbus-introspection.db")?;

for service in list_services()? {
    // Check if cached
    if cache.get_introspection_json(&service, "/", None)?.is_some() {
        continue;  // Skip re-introspection
    }

    // Introspect and cache
    let xml = introspect(&service, "/")?;
    cache.store_introspection(&service, "/", &xml)?;
}

// Later: Query directly as JSON
let all_systemd_methods = cache.get_methods_json(
    "org.freedesktop.systemd1",
    "org.freedesktop.systemd1.Manager"
)?;
```

### MCP Tool Integration

```json
{
  "method": "tools/call",
  "params": {
    "name": "query_dbus_methods",
    "arguments": {
      "service": "org.freedesktop.NetworkManager",
      "interface": "org.freedesktop.NetworkManager"
    }
  }
}
```

**Response** (JSON, not XML):
```json
{
  "methods": [
    {
      "name": "GetDevices",
      "args": [{"name": "devices", "type": "ao", "direction": "out"}]
    },
    {
      "name": "ActivateConnection",
      "args": [
        {"name": "connection", "type": "o", "direction": "in"},
        {"name": "device", "type": "o", "direction": "in"}
      ]
    }
  ]
}
```

---

## Performance Characteristics

### Caching Strategy
- **First introspection**: ~100-500ms (XML fetch + parse + convert + store)
- **Cached queries**: ~1-5ms (indexed SQLite read)
- **Search operations**: ~5-20ms (full-text pattern match)

### Comparison

| Operation | BTRFS Approach | SQLite JSON Cache |
|-----------|----------------|-------------------|
| Initial introspection | Write XML to file | Parse XML → JSON, store in DB |
| Query by service | Read XML file, parse | Indexed SELECT (~1ms) |
| Method search | Scan all XML files | Indexed LIKE query (~10ms) |
| JSON conversion | On every read | Once at write time |
| Disk overhead | ~1-5 KB per interface (XML) | ~0.5-2 KB per interface (compressed) |
| Query complexity | Filesystem I/O + XML parse | Single SQL query |

---

## Cache Management

### Expiration
```rust
// Clear cache older than 7 days
cache.clear_old_cache(7)?;
```

### Invalidation
```rust
// Manual refresh on D-Bus service restart
dbus_connection.monitor_signal("NameOwnerChanged", |signal| {
    let service_name = signal.args[0];
    cache.invalidate(service_name)?;
});
```

### Automatic Refresh
```rust
// Refresh if older than 24 hours
let cached = cache.get_introspection_json(service, path, None)?;
if let Some(json) = cached {
    let age = json["cached_at"].as_i64().unwrap();
    if age < (now() - 86400) {
        // Re-introspect
    }
}
```

---

## Migration from BTRFS

### Before (BTRFS)
```rust
// Introspection stored in BTRFS subvolume
let introspection_path = "/mnt/dbus_introspection/systemd";
fs::write(introspection_path, xml_data)?;

// Later: Read and parse XML every time
let xml = fs::read_to_string(introspection_path)?;
let node = zbus_xml::Node::from_reader(xml.as_bytes())?;
// Convert to JSON manually...
```

### After (SQLite)
```rust
// One-time conversion
cache.store_introspection("org.freedesktop.systemd1", "/", &xml_data)?;

// Later: Read as JSON directly
let json = cache.get_introspection_json("org.freedesktop.systemd1", "/", None)?;
// Already in JSON format, no parsing needed
```

---

## Benefits

### 1. **JSON-Native**
All components get JSON directly, no XML parsing overhead.

### 2. **Fast Queries**
Indexed lookups by service/interface/method name in milliseconds.

### 3. **Structured Search**
```sql
-- Find all methods with "Start" in the name
SELECT * FROM service_methods WHERE method_name LIKE '%Start%';

-- Find all read-only properties
SELECT * FROM service_properties WHERE access = 'read';

-- Services with most methods
SELECT service_name, COUNT(*) FROM service_methods GROUP BY service_name;
```

### 4. **Single File**
SQLite database is a single file, easy to backup/restore/version.

### 5. **Cross-Platform**
Works on any system with SQLite (not BTRFS-specific).

### 6. **Atomic Updates**
SQLite transactions ensure consistency during cache updates.

---

## Security Considerations

- **Read-only cache**: Most operations are reads, minimal write contention
- **File permissions**: Restrict cache file to application user
- **SQL injection**: All queries use parameterized statements
- **Disk space**: Bounded by number of D-Bus services (~1-10 MB typical)

---

## Future Enhancements

### 1. In-Memory Cache
```rust
// LRU cache for hot queries
let lru = LruCache::new(100);
```

### 2. Compression
```rust
// Compress JSON for large interfaces
let compressed = zstd::encode(json_str)?;
```

### 3. Network Sync
```rust
// Share cache across multiple instances
let cache = IntrospectionCache::new_remote("postgresql://...")?;
```

### 4. Version Tracking
```sql
ALTER TABLE introspection_cache ADD COLUMN version TEXT;
```
