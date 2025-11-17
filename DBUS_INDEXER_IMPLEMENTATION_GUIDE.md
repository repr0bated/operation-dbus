# D-Bus Indexer Implementation Guide
## Utilizing All Methods from d_bus_introspection_with_zbus.md

This document maps the techniques from `d_bus_introspection_with_zbus.md` to our implementation and identifies what's missing.

## Status: What We've Implemented

### ✅ Implemented (in feature branch)

From the d_bus_introspection_with_zbus.md guide:

**1. Connecting to Session and System Buses** (lines 3-19)
- ✅ **File**: `src/mcp/system_introspection.rs:36`
- ✅ **Method**: `Connection::system().await`
- ✅ **Usage**: `SystemIntrospector::new()` establishes connection

**2. Listing All Services** (lines 14-17)
- ✅ **File**: `src/mcp/system_introspection.rs:44`
- ✅ **Method**: `DBusProxy::new(&connection).list_names().await`
- ✅ **Usage**: `list_all_services()` returns `Vec<String>` of service names

**3. Recursive Object Path Discovery** (lines 21-54)
- ✅ **File**: `src/mcp/system_introspection.rs:101`
- ✅ **Method**: `Proxy::new()` + `introspect()` recursively
- ✅ **Usage**: `discover_object_paths()` walks object tree
- ✅ **Improvement**: Removed 100-object limit in dbus_indexer.rs

**4. XML Parsing** (lines 24-52)
- ⚠️ **Status**: Partial - we extract child nodes but don't fully parse with `zbus_xml`
- ✅ **File**: `src/mcp/system_introspection.rs:150` (extract_child_nodes)
- ❌ **Missing**: Full use of `zbus_xml::Node` for interface/method extraction

**5. Error Handling for Non-Introspectable Objects** (lines 61-82)
- ✅ **File**: `src/mcp/system_introspection.rs:136`
- ✅ **Method**: Try/catch on `introspect_service_at_path`
- ✅ **Usage**: Continues on error, logs warnings

### ❌ NOT Implemented (Missing Optimization Opportunities)

**6. ObjectManager.GetManagedObjects** (lines 87-100)
- ❌ **Status**: NOT IMPLEMENTED
- ❌ **Impact**: **HUGE** performance improvement for services like:
  - systemd/logind
  - NetworkManager
  - BlueZ
  - UDisks2
- ❌ **Benefit**: Single call gets ALL objects instead of N introspection calls

**7. Properties Introspection via GetAll** (lines 101-103)
- ❌ **Status**: NOT IMPLEMENTED
- ❌ **Impact**: Can't get runtime properties for non-introspectable objects
- ❌ **Benefit**: Workaround for objects that fail introspection

**8. Session Bus Support** (line 9)
- ❌ **Status**: Only system bus implemented
- ❌ **Impact**: Can't index user-specific D-Bus services
- ❌ **Benefit**: Complete picture includes both buses

**9. Using zbus_xml::Node** (lines 29-52)
- ❌ **Status**: Manual XML parsing instead
- ❌ **Impact**: Harder to extract method signatures, property types
- ❌ **Benefit**: Structured access to interfaces/methods/properties

**10. Permission Elevation Detection** (lines 62-67)
- ⚠️ **Status**: Partial - we catch errors but don't distinguish AccessDenied
- ❌ **Impact**: Can't suggest running with sudo
- ❌ **Benefit**: Better user guidance

---

## Implementation Plan: Utilize All Methods

### Phase 1: ObjectManager Support (HIGH PRIORITY)

**Why**: Massively speeds up indexing for modern D-Bus services

**Implementation**:
```rust
// src/mcp/dbus_indexer.rs

impl DbusIndexer {
    /// Try ObjectManager first (10x faster!)
    async fn index_service_with_object_manager(&self, service_name: &str) -> Result<ServiceIndex> {
        // Try common ObjectManager paths
        let om_paths = vec![
            "/",
            format!("/{}", service_name.replace(".", "/")),
        ];

        for path in om_paths {
            if let Ok(managed) = self.call_get_managed_objects(service_name, &path).await {
                log::info!("   ✨ Using ObjectManager for {} ({} objects)",
                    service_name, managed.len());
                return self.index_from_object_manager(service_name, managed);
            }
        }

        // Fallback to recursive introspection
        log::debug!("   🔍 Falling back to recursive introspection for {}", service_name);
        self.index_service_via_introspection(service_name).await
    }

    async fn call_get_managed_objects(
        &self,
        service: &str,
        path: &str
    ) -> Result<HashMap<String, HashMap<String, HashMap<String, Value>>>> {
        let proxy = Proxy::new(
            &self.connection,
            service,
            path,
            "org.freedesktop.DBus.ObjectManager"
        ).await?;

        let managed: HashMap<_, _> = proxy.call("GetManagedObjects", &()).await?;
        Ok(managed)
    }
}
```

**Services that benefit**:
- systemd (org.freedesktop.systemd1) - **500+ units**
- NetworkManager (org.freedesktop.NetworkManager) - **20+ devices**
- BlueZ (org.bluez) - **10+ adapters/devices**
- UDisks2 (org.freedesktop.UDisks2) - **20+ drives**

**Expected speedup**: 10-100x faster for these services

### Phase 2: Session Bus Support (MEDIUM PRIORITY)

**Why**: User-specific services (KDE, GNOME, audio, notifications)

**Implementation**:
```rust
// src/mcp/dbus_indexer.rs

pub enum BusType {
    System,
    Session,
}

impl DbusIndexer {
    pub async fn new_with_bus(index_root: PathBuf, bus: BusType) -> Result<Self> {
        let connection = match bus {
            BusType::System => Connection::system().await?,
            BusType::Session => Connection::session().await?,
        };
        // ...
    }

    /// Index BOTH system and session buses
    pub async fn build_complete_index_all_buses(&self) -> Result<DbusIndex> {
        let mut all_services = HashMap::new();

        // Index system bus
        log::info!("📡 Indexing system bus...");
        let system_indexer = DbusIndexer::new_with_bus(index_root, BusType::System).await?;
        let system_index = system_indexer.build_complete_index().await?;
        all_services.extend(system_index.services);

        // Index session bus
        log::info!("📡 Indexing session bus...");
        let session_indexer = DbusIndexer::new_with_bus(index_root, BusType::Session).await?;
        let session_index = session_indexer.build_complete_index().await?;
        all_services.extend(session_index.services);

        // Combine
        Ok(DbusIndex {
            version: "1.0.0".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            services: all_services,
            statistics: /* combined stats */,
        })
    }
}
```

**New command**:
```bash
op-dbus index build --bus system    # System bus only (default)
op-dbus index build --bus session   # Session bus only
op-dbus index build --bus all       # Both buses
```

### Phase 3: Full zbus_xml Parsing (MEDIUM PRIORITY)

**Why**: Extract method signatures, property types, signal definitions

**Implementation**:
```rust
// Add to Cargo.toml:
// zbus_xml = "3"

use zbus_xml::Node;

impl DbusIndexer {
    async fn index_object(&self, service: &str, path: &str) -> Result<ObjectIndex> {
        let xml = self.introspector.introspect_service_at_path(service, path).await?;

        // Parse with zbus_xml instead of manual parsing
        let node = Node::from_reader(xml.as_bytes())?;

        let mut methods = Vec::new();
        let mut properties = Vec::new();
        let mut interfaces = Vec::new();

        for interface in node.interfaces() {
            interfaces.push(interface.name().to_string());

            // Extract methods with full signatures
            for method in interface.methods() {
                methods.push(MethodIndex {
                    name: method.name().to_string(),
                    interface: interface.name().to_string(),
                    inputs: method.args()
                        .filter(|a| a.direction() == Some("in"))
                        .map(|a| a.ty().to_string())
                        .collect(),
                    outputs: method.args()
                        .filter(|a| a.direction() == Some("out"))
                        .map(|a| a.ty().to_string())
                        .collect(),
                });
            }

            // Extract properties with types
            for property in interface.properties() {
                properties.push(PropertyIndex {
                    name: property.name().to_string(),
                    interface: interface.name().to_string(),
                    type_signature: property.ty().to_string(),
                    access: property.access().to_string(),
                });
            }
        }

        Ok(ObjectIndex {
            path: path.to_string(),
            interfaces,
            methods,
            properties,
        })
    }
}
```

**Benefit**: Can now search by method signature or property type!

### Phase 4: Properties Fallback for Non-Introspectable (LOW PRIORITY)

**Why**: Get data from objects that don't support Introspect

**Implementation**:
```rust
impl DbusIndexer {
    async fn index_object(&self, service: &str, path: &str) -> Result<ObjectIndex> {
        // Try introspection first
        match self.introspect_object(service, path).await {
            Ok(index) => return Ok(index),
            Err(e) => {
                log::warn!("   Introspection failed for {}: {}", path, e);

                // Try Properties.GetAll as fallback
                if let Ok(props) = self.get_all_properties(service, path).await {
                    log::info!("   ✅ Got properties via GetAll");
                    return Ok(ObjectIndex {
                        path: path.to_string(),
                        interfaces: vec!["org.freedesktop.DBus.Properties".to_string()],
                        methods: Vec::new(),
                        properties: props,
                    });
                }

                return Err(e);
            }
        }
    }

    async fn get_all_properties(&self, service: &str, path: &str) -> Result<Vec<PropertyIndex>> {
        let proxy = Proxy::new(&self.connection, service, path, "org.freedesktop.DBus.Properties").await?;

        // Get all properties
        let props: HashMap<String, Value> = proxy.call("GetAll", &("",)).await?;

        Ok(props.iter().map(|(name, value)| PropertyIndex {
            name: name.clone(),
            interface: "unknown".to_string(),
            type_signature: value.value_signature().to_string(),
            access: "read".to_string(),  // Assume read-only
        }).collect())
    }
}
```

### Phase 5: Error Classification (LOW PRIORITY)

**Why**: Better diagnostics and user guidance

**Implementation**:
```rust
#[derive(Debug)]
enum IntrospectionError {
    UnknownMethod,      // Object doesn't implement Introspectable
    AccessDenied,       // Permission issue (suggest sudo)
    UnknownObject,      // Object disappeared
    Timeout,            // Service not responding
    Other(String),
}

impl DbusIndexer {
    fn classify_error(&self, error: &zbus::Error) -> IntrospectionError {
        let error_str = error.to_string();

        if error_str.contains("UnknownMethod") {
            IntrospectionError::UnknownMethod
        } else if error_str.contains("AccessDenied") {
            IntrospectionError::AccessDenied
        } else if error_str.contains("UnknownObject") {
            IntrospectionError::UnknownObject
        } else if error_str.contains("Timeout") {
            IntrospectionError::Timeout
        } else {
            IntrospectionError::Other(error_str)
        }
    }

    async fn index_service(&self, service: &str) -> Result<ServiceIndex> {
        match self.try_index_service(service).await {
            Ok(index) => Ok(index),
            Err(e) => {
                match self.classify_error(&e) {
                    IntrospectionError::AccessDenied => {
                        log::warn!("   ⚠️  Access denied for {}. Try running with sudo?", service);
                    }
                    IntrospectionError::UnknownMethod => {
                        log::debug!("   ℹ️  {} doesn't implement Introspectable", service);
                    }
                    _ => {}
                }
                Err(e)
            }
        }
    }
}
```

---

## Summary Checklist

From d_bus_introspection_with_zbus.md, we need to implement:

- [x] Connect to D-Bus (system bus)
- [x] List all services
- [x] Recursive introspection
- [x] Handle errors gracefully
- [ ] **ObjectManager.GetManagedObjects** ← HIGH PRIORITY
- [ ] Session bus support
- [ ] Full `zbus_xml::Node` parsing
- [ ] Properties.GetAll fallback
- [ ] Error classification

**Next Steps**:

1. Merge feature branch to master (get dbus_indexer.rs available)
2. Implement ObjectManager support (Phase 1)
3. Add session bus indexing (Phase 2)
4. Enhance with full zbus_xml parsing (Phase 3)

**Expected Impact**:

- **10-100x faster** indexing for ObjectManager-enabled services
- **Complete coverage** of both system and session buses
- **Better diagnostics** with error classification
- **More data** via Properties.GetAll fallback

This will make our hierarchical D-Bus abstraction layer truly **comprehensive** and **production-ready**.
