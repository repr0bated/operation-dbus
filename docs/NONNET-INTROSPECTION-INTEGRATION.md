# Integrating D-Bus Introspection with NonNet DB

## Current Architecture

```
┌─────────────────────────────────────────────────────┐
│                   JSON-RPC Clients                   │
└──────────────┬──────────────────────┬────────────────┘
               │                      │
               ▼                      ▼
         ┌──────────┐          ┌──────────┐
         │  OVSDB   │          │ NonNet DB│
         │          │          │          │
         │ Network  │          │ Plugin   │
         │ Config   │          │ State    │
         └──────────┘          └──────────┘
               │                      │
               ▼                      ▼
         JSON-RPC API          In-Memory State
```

## Proposed: Add Introspection Cache

```
┌─────────────────────────────────────────────────────┐
│                   JSON-RPC Clients                   │
└──────────┬──────────────────────┬──────────┬─────────┘
           │                      │          │
           ▼                      ▼          ▼
     ┌──────────┐          ┌──────────┐  ┌──────────┐
     │  OVSDB   │          │ NonNet DB│  │ SQLite   │
     │          │          │  +Proxy  │  │          │
     │ Network  │          │          │  │D-Bus     │
     │ Config   │          │ Plugin   │  │Intro-    │
     └──────────┘          │ State    │  │spection  │
                           └────┬─────┘  └──────────┘
                                │             │
                                └─────────────┘
                               Introspection Queries
```

## Option 1: Direct SQLite Access (Recommended)

**Pros:**
- Fastest (no RPC overhead)
- Full SQL query power
- Simplest implementation
- Separate concerns (state vs metadata)

**Usage:**
```rust
let cache = IntrospectionCache::new("/var/cache/dbus-introspection.db")?;
let methods = cache.get_methods_json("org.freedesktop.systemd1", "Manager")?;
```

**CLI:**
```bash
# Direct SQL queries
sqlite3 /var/cache/dbus-introspection.db \
  "SELECT method_name, signature_json FROM service_methods WHERE service_name='org.freedesktop.systemd1'"
```

---

## Option 2: Extend NonNet DB (Unified Interface)

**Pros:**
- Single JSON-RPC interface for all databases
- Consistent query format
- NetworkManager-style interface

**Cons:**
- RPC overhead (~5-10ms)
- More complex implementation
- Mixes runtime state with static metadata

### Implementation

Add to `src/nonnet_db/mod.rs`:

```rust
use crate::mcp::introspection_cache::IntrospectionCache;

pub struct NonNetDbServer {
    state: Arc<StateManager>,
    introspection_cache: Arc<IntrospectionCache>,
}

impl NonNetDbServer {
    pub fn new(state: Arc<StateManager>, cache_path: &str) -> Result<Self> {
        let cache = IntrospectionCache::new(cache_path)?;
        Ok(Self {
            state,
            introspection_cache: Arc::new(cache),
        })
    }
}

async fn handle_request(server: &NonNetDbServer, req: Value) -> Result<Value> {
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    let result = match method {
        "list_dbs" => json!(["OpNonNet", "OpIntrospection"]),

        "transact" => {
            let db = params.get(0).and_then(|v| v.as_str()).unwrap_or("OpNonNet");

            match db {
                "OpNonNet" => handle_transact_select(&server.state, ops).await?,
                "OpIntrospection" => handle_introspection_query(&server.introspection_cache, ops)?,
                _ => json!([{"error": "unknown db"}])
            }
        }
        _ => json!({"error": format!("unknown method: {}", method)}),
    };

    Ok(json!({"result": result, "id": id}))
}

fn handle_introspection_query(cache: &IntrospectionCache, ops: Value) -> Result<Value> {
    // ops: [{"op": "select", "table": "methods", "where": [...]}]
    for op in ops.as_array().unwrap_or(&vec![]) {
        let operation = op.get("op").and_then(|o| o.as_str()).unwrap_or("");

        match operation {
            "select" => {
                let table = op.get("table").and_then(|t| t.as_str()).unwrap_or("");

                match table {
                    "methods" => {
                        let service = extract_where_value(op, "service_name")?;
                        let interface = extract_where_value(op, "interface_name")?;

                        let methods = cache.get_methods_json(&service, &interface)?;
                        return Ok(json!([{"rows": methods}]));
                    }
                    "services" => {
                        let stats = cache.get_stats()?;
                        return Ok(json!([{"rows": stats}]));
                    }
                    _ => return Ok(json!([{"error": "unknown table"}]))
                }
            }
            _ => return Ok(json!([{"error": "only SELECT operations supported"}]))
        }
    }

    Ok(json!([{}]))
}
```

### JSON-RPC Query Example

```bash
# Query via NonNet DB JSON-RPC
echo '{"id": 1, "method": "transact", "params": ["OpIntrospection", [
  {
    "op": "select",
    "table": "methods",
    "where": [
      ["service_name", "==", "org.freedesktop.systemd1"],
      ["interface_name", "==", "org.freedesktop.systemd1.Manager"]
    ]
  }
]]}' | nc -U /run/op-dbus/nonnet.db.sock
```

**Response:**
```json
{
  "id": 1,
  "result": [{
    "rows": {
      "methods": [
        {"name": "StartUnit", "args": [...]},
        {"name": "StopUnit", "args": [...]}
      ]
    }
  }]
}
```

---

## Option 3: Separate Introspection RPC Server

Create dedicated introspection query server:

**Socket**: `/run/op-dbus/introspection.sock`
**Database**: `"OpIntrospection"`

```rust
// src/mcp/introspection_rpc.rs
pub async fn run_introspection_rpc(cache: Arc<IntrospectionCache>, socket_path: &str) -> Result<()> {
    // Similar to NonNet DB but dedicated to introspection queries
}
```

**Pros:**
- Clean separation
- Independent scaling
- Dedicated optimization

**Cons:**
- Another socket to manage
- More complex deployment

---

## Recommendation

**Use Direct SQLite Access (Option 1)**

**Rationale:**
1. **Performance**: No RPC overhead (~1-5ms vs ~10-20ms)
2. **Simplicity**: Fewer moving parts
3. **Flexibility**: Full SQL query power
4. **Separation**: Runtime state (NonNet) vs static metadata (SQLite)
5. **Tools**: Standard SQLite CLI/tools work

**When to use NonNet DB RPC:**
- Runtime plugin state queries
- Live systemd/login1/lxc status
- Dynamic configuration changes

**When to use Introspection Cache:**
- D-Bus interface discovery
- Method signature lookups
- Service capability queries
- Static metadata

---

## Combined Query Example

```rust
// Runtime state from NonNet DB
let systemd_state = nonnet_client.query("OpNonNet", "systemd")?;

// Static interface info from Introspection Cache
let systemd_methods = cache.get_methods_json(
    "org.freedesktop.systemd1",
    "org.freedesktop.systemd1.Manager"
)?;

// Combine: Show which methods are available + current state
json!({
    "state": systemd_state,
    "available_methods": systemd_methods,
    "can_start_unit": systemd_methods["methods"].as_array()
        .unwrap()
        .iter()
        .any(|m| m["name"] == "StartUnit")
})
```

---

## Migration Path

### Phase 1: Add SQLite Cache (No NonNet Changes)
```rust
// Initialize alongside NonNet DB
let cache = IntrospectionCache::new("/var/cache/dbus-introspection.db")?;

// Use directly in MCP tools
pub async fn query_dbus_methods(service: String, interface: String) -> Result<JsonValue> {
    cache.get_methods_json(&service, &interface)
}
```

### Phase 2: Optional RPC Bridge
```rust
// If unified interface desired later
impl NonNetDbServer {
    fn handle_introspection_db(&self, ops: Value) -> Result<Value> {
        // Proxy to introspection cache
    }
}
```

### Phase 3: Deprecate BTRFS Introspection
```bash
# Remove BTRFS introspection subvolumes
# All introspection now in SQLite
```
