# OVSDB Schema for D-Bus Introspection Caching

## Why OVSDB for Introspection?

**Architectural Consistency:**
- Already have OVSDB deployed and running
- Persistent storage with proper systemd integration
- JSON-native (perfect for introspection data)
- Single database system instead of 3 (OVSDB, NonNet DB, SQLite)

**Feature Match:**
- **Change Notifications**: OVSDB can notify when services restart
- **Transactions**: Atomic updates when re-introspecting
- **JSON Queries**: Native JSON support for querying methods/properties
- **Schema Validation**: Ensures data integrity

## Database Layout

### Option 1: Add to Existing Open_vSwitch Database

Add new tables to `/etc/openvswitch/conf.db`:

```json
{
  "name": "DBus_Introspection",
  "version": "1.0.0",
  "tables": {
    "Service": {
      "columns": {
        "name": {"type": "string"},
        "interfaces": {"type": {"key": {"type": "uuid", "refTable": "Interface"}}, "min": 0, "max": "unlimited"}
      },
      "indexes": [["name"]]
    },
    "Interface": {
      "columns": {
        "name": {"type": "string"},
        "object_path": {"type": "string"},
        "cached_at": {"type": "integer"},
        "methods": {"type": {"key": "string", "value": "string", "min": 0, "max": "unlimited"}},
        "properties": {"type": {"key": "string", "value": "string", "min": 0, "max": "unlimited"}},
        "signals": {"type": {"key": "string", "value": "string", "min": 0, "max": "unlimited"}}
      },
      "indexes": [["name", "object_path"]]
    }
  }
}
```

### Option 2: Separate OVSDB Instance for Introspection

Run dedicated OVSDB instance:
- **Socket**: `/run/op-dbus/introspection.db.sock`
- **Database file**: `/var/cache/dbus-introspection.ovsdb`
- **Schema**: Custom introspection schema

```bash
# Start dedicated OVSDB server
ovsdb-tool create /var/cache/dbus-introspection.ovsdb introspection.ovsschema
ovsdb-server --remote=punix:/run/op-dbus/introspection.db.sock \
  --pidfile=/run/op-dbus/introspection.db.pid \
  /var/cache/dbus-introspection.ovsdb
```

## Schema Design

### Complete Introspection Schema

```json
{
  "name": "DBusIntrospection",
  "version": "1.0.0",
  "cksum": "0",
  "tables": {
    "Cache_Entry": {
      "columns": {
        "service_name": {
          "type": "string"
        },
        "object_path": {
          "type": "string"
        },
        "interface_name": {
          "type": "string"
        },
        "cached_at": {
          "type": "integer"
        },
        "introspection_json": {
          "type": "string"
        }
      },
      "indexes": [
        ["service_name"],
        ["service_name", "object_path", "interface_name"]
      ]
    },
    "Method": {
      "columns": {
        "service_name": {
          "type": "string"
        },
        "interface_name": {
          "type": "string"
        },
        "method_name": {
          "type": "string"
        },
        "in_args": {
          "type": "string"
        },
        "out_args": {
          "type": "string"
        },
        "signature_json": {
          "type": "string"
        }
      },
      "indexes": [
        ["service_name", "interface_name", "method_name"],
        ["method_name"]
      ]
    },
    "Property": {
      "columns": {
        "service_name": {
          "type": "string"
        },
        "interface_name": {
          "type": "string"
        },
        "property_name": {
          "type": "string"
        },
        "type_signature": {
          "type": "string"
        },
        "access": {
          "type": {"key": {"type": "string", "enum": ["set", ["read", "write", "readwrite"]]}}
        }
      },
      "indexes": [
        ["service_name", "interface_name", "property_name"]
      ]
    },
    "Signal": {
      "columns": {
        "service_name": {
          "type": "string"
        },
        "interface_name": {
          "type": "string"
        },
        "signal_name": {
          "type": "string"
        },
        "args_json": {
          "type": "string"
        }
      },
      "indexes": [
        ["service_name", "interface_name", "signal_name"]
      ]
    },
    "Statistics": {
      "columns": {
        "total_services": {
          "type": "integer"
        },
        "total_interfaces": {
          "type": "integer"
        },
        "total_methods": {
          "type": "integer"
        },
        "last_updated": {
          "type": "integer"
        }
      },
      "maxRows": 1
    }
  }
}
```

## Usage Examples

### Store Introspection (JSON-RPC)

```bash
ovsdb-client transact unix:/run/op-dbus/introspection.db.sock '[
  "DBusIntrospection",
  {
    "op": "insert",
    "table": "Cache_Entry",
    "row": {
      "service_name": "org.freedesktop.systemd1",
      "object_path": "/org/freedesktop/systemd1",
      "interface_name": "org.freedesktop.systemd1.Manager",
      "cached_at": 1699564800,
      "introspection_json": "{\"methods\": [...], \"properties\": [...]}"
    }
  },
  {
    "op": "insert",
    "table": "Method",
    "row": {
      "service_name": "org.freedesktop.systemd1",
      "interface_name": "org.freedesktop.systemd1.Manager",
      "method_name": "StartUnit",
      "signature_json": "{\"in_args\": [{\"name\": \"name\", \"type\": \"s\"}, {\"name\": \"mode\", \"type\": \"s\"}], \"out_args\": [{\"name\": \"job\", \"type\": \"o\"}]}"
    }
  }
]'
```

### Query Methods (JSON-RPC)

```bash
# Get all methods for a service
ovsdb-client transact unix:/run/op-dbus/introspection.db.sock '[
  "DBusIntrospection",
  {
    "op": "select",
    "table": "Method",
    "where": [
      ["service_name", "==", "org.freedesktop.systemd1"],
      ["interface_name", "==", "org.freedesktop.systemd1.Manager"]
    ]
  }
]'

# Search for methods by name pattern
ovsdb-client transact unix:/run/op-dbus/introspection.db.sock '[
  "DBusIntrospection",
  {
    "op": "select",
    "table": "Method",
    "where": [["method_name", "includes", "Start"]]
  }
]'
```

### Monitor for Changes

```bash
# Get notified when introspection data changes
ovsdb-client monitor unix:/run/op-dbus/introspection.db.sock \
  DBusIntrospection Cache_Entry
```

## Rust Implementation

```rust
use crate::native::ovsdb_jsonrpc::OvsdbClient;
use serde_json::json;

pub struct OvsdbIntrospectionCache {
    client: OvsdbClient,
    db_name: String,
}

impl OvsdbIntrospectionCache {
    pub fn new(socket_path: &str) -> Self {
        Self {
            client: OvsdbClient::new_with_socket(socket_path),
            db_name: "DBusIntrospection".to_string(),
        }
    }

    pub async fn store_introspection(
        &self,
        service_name: &str,
        object_path: &str,
        xml_data: &str,
    ) -> Result<()> {
        // Parse XML to JSON
        let node = zbus_xml::Node::from_reader(xml_data.as_bytes())?;
        let json_data = self.xml_to_json(&node)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Build OVSDB transaction
        for interface in node.interfaces() {
            let ops = json!([
                &self.db_name,
                {
                    "op": "insert",
                    "table": "Cache_Entry",
                    "row": {
                        "service_name": service_name,
                        "object_path": object_path,
                        "interface_name": interface.name,
                        "cached_at": timestamp,
                        "introspection_json": serde_json::to_string(&json_data)?
                    }
                }
            ]);

            self.client.transact(&ops).await?;
        }

        Ok(())
    }

    pub async fn get_methods_json(
        &self,
        service_name: &str,
        interface_name: &str,
    ) -> Result<Value> {
        let ops = json!([
            &self.db_name,
            {
                "op": "select",
                "table": "Method",
                "where": [
                    ["service_name", "==", service_name],
                    ["interface_name", "==", interface_name]
                ]
            }
        ]);

        let result = self.client.transact(&ops).await?;
        Ok(result)
    }

    pub async fn search_methods(&self, pattern: &str) -> Result<Vec<Value>> {
        let ops = json!([
            &self.db_name,
            {
                "op": "select",
                "table": "Method",
                "where": [["method_name", "includes", pattern]]
            }
        ]);

        let result = self.client.transact(&ops).await?;
        Ok(serde_json::from_value(result)?)
    }
}
```

## Comparison: OVSDB vs SQLite

| Feature | OVSDB | SQLite |
|---------|-------|--------|
| **Setup** | Schema required | Zero config |
| **Queries** | JSON-RPC select | Full SQL |
| **Indexes** | Schema-defined | CREATE INDEX |
| **Notifications** | Built-in monitor | Requires triggers |
| **Transactions** | Native | Native |
| **JSON Support** | Native (strings) | Native (JSON1 ext) |
| **Pattern Matching** | Limited | Full LIKE/REGEXP |
| **Deployment** | Need ovsdb-server | Embedded library |
| **Persistence** | File-based | File-based |
| **Integration** | Already deployed | New dependency |

## Recommendation

**Use OVSDB if:**
- ✅ Want architectural consistency (single database system)
- ✅ Need change notifications (service restart detection)
- ✅ Already managing OVSDB infrastructure
- ✅ Simple query patterns (by service/interface/method name)

**Use SQLite if:**
- ✅ Need complex ad-hoc queries (JOIN, GROUP BY, etc.)
- ✅ Want zero external dependencies
- ✅ Don't need real-time notifications
- ✅ Prefer embedded library over separate process

## Migration Path

### Phase 1: Prototype with OVSDB
1. Create `introspection.ovsschema` file
2. Initialize OVSDB instance
3. Implement `OvsdbIntrospectionCache` in Rust
4. Test with systemd introspection

### Phase 2: Integrate with MCP
1. Add OVSDB cache to MCP tools
2. Update introspection endpoints to query OVSDB
3. Add change monitoring for service restarts

### Phase 3: Production Deployment
1. Add systemd service for introspection OVSDB
2. Migrate existing introspection data
3. Remove BTRFS introspection layer
4. Document OVSDB management procedures

## Systemd Service

```ini
[Unit]
Description=D-Bus Introspection OVSDB Server
After=network.target

[Service]
Type=forking
ExecStartPre=/usr/bin/ovsdb-tool create /var/cache/dbus-introspection.ovsdb /etc/op-dbus/introspection.ovsschema
ExecStart=/usr/bin/ovsdb-server \
  --remote=punix:/run/op-dbus/introspection.db.sock \
  --pidfile=/run/op-dbus/introspection.db.pid \
  --log-file=/var/log/op-dbus/introspection-db.log \
  --detach \
  /var/cache/dbus-introspection.ovsdb

ExecStop=/usr/bin/ovs-appctl -t /run/op-dbus/introspection.db.pid exit
PIDFile=/run/op-dbus/introspection.db.pid
Restart=on-failure

[Install]
WantedBy=multi-user.target
```
