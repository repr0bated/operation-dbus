# MCP Introspection Flow: How D-Bus Methods Become Individual MCP Tools

## Overview
You're absolutely right! The system uses **D-Bus introspection** to automatically present each D-Bus method as a separate MCP tool to the client (Cursor/Claude). This happens through the **bridge** component, not requiring manual tool registration.

## The Complete Flow

### 1. MCP Bridge Initialization (`src/mcp/bridge.rs`)

When the bridge starts, it introspects the D-Bus service:

```rust
// Line 74-86: Uses busctl to get XML introspection
let mut cmd = std::process::Command::new("busctl");
cmd.arg("introspect")
   .arg("--xml-interface")
   .arg(&service_name)  // e.g., "org.freedesktop.systemd1"
   .arg(&path);         // e.g., "/org/freedesktop/systemd1"

let xml = String::from_utf8_lossy(&output.stdout).to_string();

// Line 95: Parse XML to structured data
let data = IntrospectionParser::parse_xml(&xml);
```

### 2. XML to Structured Data (`src/mcp/introspection_parser.rs`)

The parser converts D-Bus XML introspection into structured method data:

**Input (D-Bus XML):**
```xml
<interface name="org.freedesktop.systemd1.Manager">
  <method name="StartUnit">
    <arg name="name" type="s" direction="in"/>
    <arg name="mode" type="s" direction="in"/>
    <arg name="job" type="o" direction="out"/>
  </method>
  <method name="StopUnit">
    <arg name="name" type="s" direction="in"/>
    <arg name="mode" type="s" direction="in"/>
    <arg name="job" type="o" direction="out"/>
  </method>
  <method name="RestartUnit">
    <arg name="name" type="s" direction="in"/>
    <arg name="mode" type="s" direction="in"/>
    <arg name="job" type="o" direction="out"/>
  </method>
</interface>
```

**Output (Structured Rust Data):**
```rust
IntrospectionData {
    interfaces: vec![InterfaceInfo {
        name: "org.freedesktop.systemd1.Manager",
        methods: vec![
            MethodInfo {
                name: "StartUnit",
                inputs: vec![
                    ArgInfo { name: "name", type_sig: "s", type_name: "string" },
                    ArgInfo { name: "mode", type_sig: "s", type_name: "string" }
                ],
                outputs: vec![
                    ArgInfo { name: "job", type_sig: "o", type_name: "object_path" }
                ]
            },
            MethodInfo { name: "StopUnit", inputs: [...], outputs: [...] },
            MethodInfo { name: "RestartUnit", inputs: [...], outputs: [...] },
            // ... ALL methods from the interface
        ],
        properties: vec![...],
        signals: vec![...]
    }]
}
```

### 3. Methods to MCP Tools (`src/mcp/bridge.rs` lines 175-216)

The bridge transforms **each method** into an **individual MCP tool**:

```rust
fn handle_tools_list(&self, id: Option<Value>) -> McpResponse {
    let tools: Vec<Value> = self
        .methods
        .iter()
        .map(|method| {
            let tool_name = self.method_to_tool_name(&method.name);
            
            // Build input schema from D-Bus method arguments
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();
            
            for input in &method.inputs {
                properties.insert(
                    input.name.clone(),
                    json!({
                        "type": IntrospectionParser::dbus_type_to_mcp_schema(&input.type_sig).get("type").unwrap(),
                        "description": format!("{} ({})", input.name, input.type_name)
                    }),
                );
                required.push(input.name.clone());
            }
            
            // Each method becomes an MCP tool
            json!({
                "name": tool_name,
                "description": format!("Call D-Bus method: {}.{}", self.service_name, method.name),
                "inputSchema": {
                    "type": "object",
                    "properties": properties,
                    "required": required
                }
            })
        })
        .collect();
    
    // Return tools list to MCP client
    McpResponse {
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}
```

### 4. What the MCP Client Sees

The AI assistant (Cursor/Claude) receives this JSON response:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "start_unit",
        "description": "Call D-Bus method: org.freedesktop.systemd1.StartUnit",
        "inputSchema": {
          "type": "object",
          "properties": {
            "name": {
              "type": "string",
              "description": "name (string)"
            },
            "mode": {
              "type": "string",
              "description": "mode (string)"
            }
          },
          "required": ["name", "mode"]
        }
      },
      {
        "name": "stop_unit",
        "description": "Call D-Bus method: org.freedesktop.systemd1.StopUnit",
        "inputSchema": {
          "type": "object",
          "properties": {
            "name": {"type": "string"},
            "mode": {"type": "string"}
          },
          "required": ["name", "mode"]
        }
      },
      {
        "name": "restart_unit",
        "description": "Call D-Bus method: org.freedesktop.systemd1.RestartUnit",
        "inputSchema": {...}
      }
      // ... ALL methods from systemd1 interface
    ]
  }
}
```

## Example: systemd1 Interface Exposure

### Services Introspected by Discovery (`src/mcp/discovery.rs` line 49-53)

```rust
let targets = vec![
    "org.freedesktop.systemd1",       // systemd control
    "org.freedesktop.NetworkManager",  // network management
    "org.freedesktop.login1",          // session management
];
```

### systemd1 Methods Exposed as Individual MCP Tools

From introspecting `org.freedesktop.systemd1.Manager`:

| D-Bus Method | MCP Tool Name | Parameters | Description |
|-------------|---------------|------------|-------------|
| `StartUnit` | `start_unit` | `name: string, mode: string` | Start a systemd unit |
| `StopUnit` | `stop_unit` | `name: string, mode: string` | Stop a systemd unit |
| `RestartUnit` | `restart_unit` | `name: string, mode: string` | Restart a systemd unit |
| `ReloadUnit` | `reload_unit` | `name: string, mode: string` | Reload a systemd unit |
| `EnableUnitFiles` | `enable_unit_files` | `files: array<string>` | Enable unit files |
| `DisableUnitFiles` | `disable_unit_files` | `files: array<string>` | Disable unit files |
| `GetUnit` | `get_unit` | `name: string` | Get unit object path |
| `ListUnits` | `list_units` | (none) | List all units |
| `ListUnitFiles` | `list_unit_files` | (none) | List all unit files |
| ... | ... | ... | **~100+ methods total** |

**Each one appears as a separate tool in your MCP menu!**

## Type Conversion: D-Bus ↔ MCP Schema

The `IntrospectionParser` automatically converts D-Bus types to JSON schema:

| D-Bus Type | MCP Schema Type | Example |
|-----------|----------------|---------|
| `s` | `string` | Service names, paths |
| `i` | `integer` | 32-bit signed integers |
| `u` | `integer` (minimum: 0) | 32-bit unsigned integers |
| `b` | `boolean` | true/false flags |
| `as` | `array<string>` | List of strings |
| `a{ss}` | `object` | String-to-string dictionary |
| `a{sv}` | `object` (variant values) | Generic property maps |
| `o` | `string` | D-Bus object paths |

See `src/mcp/introspection_parser.rs` lines 190-292 for complete mappings.

## The Power of Introspection

### Advantages:

1. **Zero Configuration**: No manual tool registration needed
2. **Always Up-to-Date**: Tools reflect current D-Bus interface
3. **Type Safety**: Automatic schema validation from D-Bus types
4. **Complete Exposure**: ALL methods automatically available
5. **Self-Documenting**: Tool descriptions from method names

### Security Implications:

⚠️ **This is why you see ALL Linux commands exposed in the MCP menu!**

```
D-Bus Service → Introspection → Bridge → MCP Client
    ↓               ↓              ↓         ↓
systemd1     → 100+ methods → 100+ tools → AI sees all
NetworkMgr   → 50+ methods  → 50+ tools  → AI sees all
login1       → 30+ methods  → 30+ tools  → AI sees all
OVS OVSDB    → 40+ methods  → 40+ tools  → AI sees all
```

**Every D-Bus method becomes an AI-callable tool automatically!**

## Current Architecture: Bridge-Based

```
┌──────────────────────────────────────────────────────────────┐
│  MCP Client (Cursor/Claude)                                   │
│  "Hey, what systemd tools do you have?"                       │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             │ MCP Protocol (JSON-RPC)
                             │
┌────────────────────────────▼─────────────────────────────────┐
│  DbusMcpBridge (src/mcp/bridge.rs)                           │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ 1. Introspect D-Bus service on startup               │   │
│  │ 2. Parse XML to MethodInfo structs                   │   │
│  │ 3. Generate MCP tool for each method                 │   │
│  │ 4. Handle tools/list request                         │   │
│  │ 5. Handle tools/call → execute D-Bus method          │   │
│  └──────────────────────────────────────────────────────┘   │
└────────────────────────────┬─────────────────────────────────┘
                             │
                             │ D-Bus Protocol
                             │
┌────────────────────────────▼─────────────────────────────────┐
│  D-Bus System Services                                        │
│  ├─ org.freedesktop.systemd1   (100+ methods)               │
│  ├─ org.freedesktop.NetworkManager (50+ methods)            │
│  ├─ org.freedesktop.login1     (30+ methods)                │
│  └─ org.openvswitch.ovsdb      (40+ methods)                │
└──────────────────────────────────────────────────────────────┘
```

## Discovery Service Role

The discovery service (`src/mcp/discovery.rs`) generates **MCP server configs** for each D-Bus service:

```rust
// Discovery output: /tmp/mcp-servers/systemd1.json
{
  "command": "./target/debug/dbus-mcp-bridge",
  "args": ["--service", "org.freedesktop.systemd1"],
  "env": {
    "DBUS_SERVICE": "org.freedesktop.systemd1",
    "MCP_NAME": "systemd1"
  }
}
```

The AI client loads these configs and connects to separate MCP bridges for each service, which then introspect and expose all methods.

## OVS Introspection Example

For OVSDB, if you exposed `org.openvswitch.ovsdb` via D-Bus, introspection would reveal:

```rust
// OVSDB D-Bus Interface (hypothetical)
interface "org.openvswitch.ovsdb.Database" {
    method "Transact" {
        arg "operations" type "a{sv}" direction "in"
        arg "result" type "a{sv}" direction "out"
    }
    method "ListDatabases" {
        arg "databases" type "as" direction "out"
    }
    // ... more methods
}
```

**Each becomes an MCP tool:**
- `transact` - Execute OVSDB transaction
- `list_databases` - List available databases
- `get_schema` - Get database schema
- `monitor` - Monitor database changes

## Recommendation: Introspection + Security Layer

The introspection approach is powerful but needs a security wrapper:

```rust
pub struct SecureToolBridge {
    introspected_tools: Vec<ToolInfo>,
    security_policy: SecurityPolicy,
}

impl SecureToolBridge {
    async fn handle_tools_list(&self) -> Vec<ToolInfo> {
        // Filter introspected tools by security policy
        self.introspected_tools
            .iter()
            .filter(|tool| self.security_policy.is_allowed_for_ai(tool))
            .cloned()
            .collect()
    }
    
    async fn handle_tools_call(&self, tool: &str, params: Value) -> Result<Value> {
        // Check if operation is safe for AI
        if self.security_policy.requires_approval(tool) {
            return Err("Operation requires human approval");
        }
        
        // Execute with audit logging
        self.audit_log.record(tool, params);
        self.execute_dbus_method(tool, params).await
    }
}
```

## Conclusion

The **bridge introspects D-Bus services** and **automatically exposes each method as an individual MCP tool**. This is:

✅ **Automatic**: No manual configuration
✅ **Dynamic**: Reflects current D-Bus state
✅ **Complete**: All methods exposed

⚠️ **Security Risk**: ALL D-Bus methods accessible to AI
⚠️ **Need Filtering**: Add security policy layer

**The screenshot you showed lists individual MCP tools because each D-Bus method was introspected and converted into a separate tool by the bridge.**
