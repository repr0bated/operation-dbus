# PlugTree Pattern - Hierarchical Plugin Architecture

## Concept

**PlugTree** is a design pattern for plugins that manage collections of independent sub-resources (called **pluglets**).

```
Plugin (PlugTree parent)
 ├─ Pluglet:A (child resource)
 ├─ Pluglet:B (child resource)
 └─ Pluglet:C (child resource)
```

## Why Use PlugTree?

**Without PlugTree:**
```bash
# Dangerous - applies ALL containers at once
op-dbus apply state.json --plugin lxc
```

**With PlugTree:**
```bash
# Safe - only affects container 100
op-dbus apply-container 100

# Or query just one
op-dbus query-container 101
```

## Architecture

### Parent Plugin (Collection)
- Manages discovery of all pluglets
- Provides bulk operations
- Introspects common properties

### Pluglet (Individual Resource)
- Autonomous lifecycle (create/modify/delete)
- Unique configuration
- Independent state

## Implementation Example: LXC Plugin

```rust
use crate::state::plugtree::PlugTree;

#[async_trait]
impl PlugTree for LxcPlugin {
    fn pluglet_type(&self) -> &str {
        "container"  // What kind of sub-resources
    }

    fn pluglet_id_field(&self) -> &str {
        "id"  // Field name for unique identifier
    }

    fn extract_pluglet_id(&self, resource: &Value) -> Result<String> {
        resource.get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing id"))
    }

    async fn apply_pluglet(&self, pluglet_id: &str, desired: &Value) -> Result<ApplyResult> {
        let container: ContainerInfo = serde_json::from_value(desired.clone())?;
        self.apply_container_state(&container).await
    }

    async fn query_pluglet(&self, pluglet_id: &str) -> Result<Option<Value>> {
        let containers = self.discover_from_ovs().await?;
        Ok(containers.into_iter()
            .find(|c| c.id == pluglet_id)
            .map(|c| serde_json::to_value(c).unwrap()))
    }

    async fn list_pluglet_ids(&self) -> Result<Vec<String>> {
        let containers = self.discover_from_ovs().await?;
        Ok(containers.into_iter().map(|c| c.id).collect())
    }
}
```

## When to Use PlugTree

### ✅ Use PlugTree When:
- Plugin manages a **collection** of similar resources
- No native protocol for granular operations
- Resources need individual lifecycle management

### ❌ Don't Use PlugTree When:
- Native protocol already provides granular operations (e.g., OVS JSON-RPC)
- Plugin is read-only introspection (e.g., login1)
- Single resource, not a collection

## Use Cases

| Plugin | Pluglet Type | ID Field | Should Use PlugTree? | Reason |
|--------|-------------|----------|---------------------|---------|
| **lxc** | container | id | ✅ **YES** | No native API, need granular control |
| **net** | interface | name | ❌ **NO** | OVS JSON-RPC already has add-port/del-br |
| **systemd** | unit | name | ⚠️ **MAYBE** | D-Bus can control units, but PlugTree could simplify |
| **login1** | session | id | ❌ **NO** | Read-only introspection only |

## State File Structure

### LXC (Multiple Containers)
```json
{
  "lxc": {
    "containers": [
      {"id": "100", "properties": {"network_type": "netmaker"}},
      {"id": "101", "properties": {"network_type": "bridge"}},
      {"id": "102", "properties": {"network_type": "netmaker"}}
    ]
  }
}
```

### Network (Multiple Interfaces)
```json
{
  "net": {
    "interfaces": [
      {"name": "vmbr0", "type": "ovs-bridge"},
      {"name": "mesh", "type": "ovs-bridge"}
    ]
  }
}
```

## Commands

### Parent-Level (Bulk)
```bash
op-dbus query --plugin lxc              # All containers
op-dbus diff state.json --plugin lxc    # Show all changes
op-dbus apply state.json --plugin lxc   # Apply all (careful!)
```

### Pluglet-Level (Individual)
```bash
op-dbus apply-container 100             # Only container 100
op-dbus query-container 100             # Query container 100
op-dbus delete-container 100            # Delete container 100
```

## Benefits

1. **Safety** - Change one resource without affecting others
2. **Granularity** - Tune individual sub-resources
3. **Isolation** - Failures don't cascade
4. **Clarity** - Clear parent/child relationship
5. **Reusability** - Same pattern across all collection-based plugins

## Future Extensions

Could add to other plugins:

- **NetPlugin**: per-interface operations
- **SystemdPlugin**: per-unit operations  
- **NetmakerPlugin**: per-node operations

All using the same PlugTree pattern!

