# MCP Developer Guide

## Extending and Customizing the MCP Integration

### Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Creating Custom Agents](#creating-custom-agents)
3. [Adding New Tools](#adding-new-tools)
4. [Extending Discovery](#extending-discovery)
5. [Custom Bridges](#custom-bridges)
6. [Testing Framework](#testing-framework)
7. [Performance Optimization](#performance-optimization)
8. [Security Considerations](#security-considerations)

---

## Architecture Overview

### Core Design Principles

1. **Modularity**: Each component is independent and replaceable
2. **Type Safety**: Strong typing with JSON schemas
3. **Security First**: Sandboxed agents with limited permissions
4. **Async by Default**: Non-blocking I/O throughout
5. **Zero Trust**: All inputs validated, all operations authenticated

### Code Organization

```
src/mcp/
├── mod.rs                 # Module declarations
├── lib.rs                 # Shared utilities
├── main.rs               # MCP server entry point
├── orchestrator.rs       # Agent management
├── bridge.rs             # Generic D-Bus bridge
├── discovery.rs          # Service discovery
├── discovery_enhanced.rs # Advanced discovery
├── introspection_parser.rs # XML parsing
├── json_introspection.rs # JSON conversion
├── web_bridge.rs         # WebSocket support
├── web_main.rs           # Web server
└── agents/               # Agent implementations
    ├── executor.rs       # Command execution
    ├── file.rs          # File operations
    ├── monitor.rs       # System monitoring
    ├── network.rs       # Network management
    └── systemd.rs       # Systemd control
```

---

## Creating Custom Agents

### Step 1: Define Agent Structure

Create `src/mcp/agents/custom.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zbus::{dbus_interface, Connection, ConnectionBuilder, SignalContext};

/// Custom agent for specialized operations
pub struct CustomAgent {
    agent_id: String,
    config: AgentConfig,
    state: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AgentConfig {
    max_operations: usize,
    timeout_seconds: u64,
    allowed_operations: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Task {
    id: String,
    operation: String,
    params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TaskResult {
    id: String,
    status: String,
    output: Option<String>,
    error: Option<String>,
}
```

### Step 2: Implement D-Bus Interface

```rust
#[dbus_interface(name = "org.dbusmcp.Agent.Custom")]
impl CustomAgent {
    /// Execute a task
    async fn execute_task(&mut self, task_json: String) -> String {
        let task: Task = match serde_json::from_str(&task_json) {
            Ok(t) => t,
            Err(e) => {
                return serde_json::to_string(&TaskResult {
                    id: "unknown".to_string(),
                    status: "error".to_string(),
                    output: None,
                    error: Some(format!("Failed to parse task: {}", e)),
                }).unwrap_or_default();
            }
        };

        // Validate operation
        if !self.config.allowed_operations.contains(&task.operation) {
            return serde_json::to_string(&TaskResult {
                id: task.id,
                status: "error".to_string(),
                output: None,
                error: Some(format!("Operation '{}' not allowed", task.operation)),
            }).unwrap_or_default();
        }

        // Execute operation
        match self.handle_operation(&task).await {
            Ok(output) => {
                serde_json::to_string(&TaskResult {
                    id: task.id,
                    status: "success".to_string(),
                    output: Some(output),
                    error: None,
                }).unwrap_or_default()
            }
            Err(e) => {
                serde_json::to_string(&TaskResult {
                    id: task.id,
                    status: "error".to_string(),
                    output: None,
                    error: Some(e.to_string()),
                }).unwrap_or_default()
            }
        }
    }

    /// Get agent status
    fn get_status(&self) -> String {
        serde_json::to_string(&serde_json::json!({
            "agent_id": self.agent_id,
            "type": "custom",
            "state": self.state,
            "config": self.config,
        })).unwrap_or_default()
    }

    /// Shutdown agent
    fn shutdown(&mut self) -> bool {
        eprintln!("Agent {} shutting down", self.agent_id);
        true
    }

    /// Signal: Task completed
    #[dbus_interface(signal)]
    async fn task_completed(
        ctxt: &SignalContext<'_>,
        task_id: String,
        status: String,
    ) -> zbus::Result<()>;
}
```

### Step 3: Implement Agent Logic

```rust
impl CustomAgent {
    pub fn new(agent_id: String) -> Self {
        let config = Self::load_config();
        Self {
            agent_id,
            config,
            state: HashMap::new(),
        }
    }

    fn load_config() -> AgentConfig {
        // Load from file or use defaults
        if let Ok(config_str) = std::fs::read_to_string("mcp-configs/agents/custom.json") {
            serde_json::from_str(&config_str).unwrap_or_default()
        } else {
            AgentConfig {
                max_operations: 100,
                timeout_seconds: 30,
                allowed_operations: vec!["example".to_string()],
            }
        }
    }

    async fn handle_operation(&mut self, task: &Task) -> Result<String, Box<dyn std::error::Error>> {
        match task.operation.as_str() {
            "example" => self.example_operation(task).await,
            "another_example" => self.another_operation(task).await,
            _ => Err(format!("Unknown operation: {}", task.operation).into()),
        }
    }

    async fn example_operation(&mut self, task: &Task) -> Result<String, Box<dyn std::error::Error>> {
        // Implement your custom logic here
        let param = task.params.get("input")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'input' parameter")?;

        // Process the input
        let result = format!("Processed: {}", param.to_uppercase());

        // Update state
        self.state.insert("last_operation".to_string(), task.operation.clone());
        self.state.insert("last_result".to_string(), result.clone());

        Ok(result)
    }

    async fn another_operation(&mut self, task: &Task) -> Result<String, Box<dyn std::error::Error>> {
        // Another operation implementation
        Ok("Another operation completed".to_string())
    }
}
```

### Step 4: Create Main Entry Point

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent_id = std::env::var("AGENT_ID")
        .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

    eprintln!("Starting Custom Agent with ID: {}", agent_id);

    let agent = CustomAgent::new(agent_id.clone());

    let connection = ConnectionBuilder::session()?
        .name(format!("org.dbusmcp.Agent.Custom.{}", agent_id))?
        .serve_at(
            format!("/org/dbusmcp/Agent/Custom/{}", agent_id),
            agent,
        )?
        .build()
        .await?;

    eprintln!("Custom Agent ready on D-Bus");

    // Keep running
    std::future::pending::<()>().await;
    Ok(())
}
```

### Step 5: Register with Cargo.toml

Add to `Cargo.toml`:

```toml
[[bin]]
name = "dbus-agent-custom"
path = "src/mcp/agents/custom.rs"
required-features = ["mcp"]
```

### Step 6: Create Configuration

Create `mcp-configs/agents/custom.json`:

```json
{
  "max_operations": 100,
  "timeout_seconds": 30,
  "allowed_operations": [
    "example",
    "another_example"
  ],
  "resource_limits": {
    "max_memory_mb": 50,
    "max_cpu_percent": 25
  }
}
```

### Step 7: Register with Orchestrator

Update orchestrator configuration to include the new agent:

```json
{
  "allowed_agents": [
    "systemd",
    "file",
    "network",
    "monitor",
    "executor",
    "custom"  // Add your agent here
  ]
}
```

---

## Adding New Tools

### Tool Definition Pattern

Tools are defined with JSON schemas for type safety:

```rust
// In src/mcp/main.rs or a dedicated tools module

fn define_custom_tool() -> Tool {
    Tool {
        name: "custom_operation".to_string(),
        description: "Perform a custom operation".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input to process"
                },
                "options": {
                    "type": "object",
                    "properties": {
                        "uppercase": {
                            "type": "boolean",
                            "default": false
                        },
                        "reverse": {
                            "type": "boolean",
                            "default": false
                        }
                    }
                }
            },
            "required": ["input"]
        }),
    }
}
```

### Tool Implementation

```rust
async fn handle_custom_tool(params: Value) -> Result<Value, McpError> {
    // Validate parameters
    let input = params.get("input")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: -32602,
            message: "Missing 'input' parameter".to_string(),
        })?;

    let uppercase = params
        .get("options")
        .and_then(|o| o.get("uppercase"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let reverse = params
        .get("options")
        .and_then(|o| o.get("reverse"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Process input
    let mut result = input.to_string();
    
    if uppercase {
        result = result.to_uppercase();
    }
    
    if reverse {
        result = result.chars().rev().collect();
    }

    Ok(json!({
        "content": [{
            "type": "text",
            "text": result
        }]
    }))
}
```

### Registering Tools

```rust
impl McpServer {
    fn get_tools(&self) -> Vec<Tool> {
        vec![
            // Existing tools
            self.define_systemd_tools(),
            self.define_file_tools(),
            // Add your custom tool
            define_custom_tool(),
        ].into_iter().flatten().collect()
    }

    async fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> McpResponse {
        let params = match params {
            Some(p) => p,
            None => return self.error_response(id, -32602, "Missing params"),
        };

        let tool_name = params.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("");

        let result = match tool_name {
            // Existing tools
            "systemd_status" => self.handle_systemd_status(params).await,
            "file_read" => self.handle_file_read(params).await,
            // Add your custom tool
            "custom_operation" => handle_custom_tool(params).await,
            _ => {
                return self.error_response(
                    id, 
                    -32004, 
                    &format!("Tool '{}' not found", tool_name)
                );
            }
        };

        match result {
            Ok(content) => self.success_response(id, content),
            Err(e) => self.error_response(id, e.code, &e.message),
        }
    }
}
```

---

## Extending Discovery

### Custom Service Categorization

```rust
// In src/mcp/discovery_enhanced.rs

fn categorize_service_custom(service_name: &str) -> ServiceCategory {
    // Add custom patterns
    if service_name.contains("myapp") {
        return ServiceCategory::Custom("myapp".to_string());
    }

    // Check for specific interfaces
    if has_interface(service_name, "com.mycompany.Interface") {
        return ServiceCategory::Custom("mycompany".to_string());
    }

    // Default categorization
    categorize_service_default(service_name)
}
```

### Custom Introspection

```rust
async fn introspect_custom_service(
    connection: &Connection,
    service: &str,
) -> Result<ServiceInfo, Box<dyn std::error::Error>> {
    let proxy = Proxy::new(
        connection,
        service,
        "/com/mycompany/Object",
        Duration::from_secs(5),
    ).await?;

    // Get custom properties
    let version: String = proxy.get_property("Version").await?;
    let features: Vec<String> = proxy.get_property("Features").await?;

    Ok(ServiceInfo {
        name: service.to_string(),
        version: Some(version),
        features,
        interfaces: introspect_interfaces(&proxy).await?,
    })
}
```

### Discovery Filters

```rust
struct DiscoveryFilter {
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    interface_requirements: Vec<String>,
}

impl DiscoveryFilter {
    fn matches(&self, service: &ServiceInfo) -> bool {
        // Check exclude patterns first
        for pattern in &self.exclude_patterns {
            if service.name.contains(pattern) {
                return false;
            }
        }

        // Check include patterns
        let mut included = self.include_patterns.is_empty();
        for pattern in &self.include_patterns {
            if service.name.contains(pattern) {
                included = true;
                break;
            }
        }

        if !included {
            return false;
        }

        // Check interface requirements
        for required in &self.interface_requirements {
            if !service.interfaces.iter().any(|i| i.name == *required) {
                return false;
            }
        }

        true
    }
}
```

---

## Custom Bridges

### Creating a Specialized Bridge

```rust
// src/mcp/bridges/custom_bridge.rs

use super::introspection_parser::IntrospectionParser;
use serde_json::{json, Value};
use zbus::Connection;

pub struct CustomBridge {
    connection: Connection,
    service_name: String,
    cache: HashMap<String, Value>,
}

impl CustomBridge {
    pub async fn new(service_name: String) -> Result<Self, Box<dyn std::error::Error>> {
        let connection = Connection::session().await?;
        
        Ok(Self {
            connection,
            service_name,
            cache: HashMap::new(),
        })
    }

    pub async fn discover_methods(&mut self) -> Result<Vec<MethodInfo>, Box<dyn std::error::Error>> {
        // Custom discovery logic
        let introspection = self.introspect().await?;
        
        // Parse and enhance method information
        let methods = self.parse_methods(introspection);
        
        // Add custom metadata
        let enhanced_methods = self.enhance_methods(methods);
        
        // Cache for performance
        self.cache_methods(enhanced_methods.clone());
        
        Ok(enhanced_methods)
    }

    async fn introspect(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Perform D-Bus introspection
        let proxy = zbus::Proxy::new(
            &self.connection,
            &self.service_name,
            "/",
            "org.freedesktop.DBus.Introspectable",
        ).await?;

        proxy.call("Introspect", &()).await
    }

    fn parse_methods(&self, xml: String) -> Vec<MethodInfo> {
        // Parse XML and extract methods
        IntrospectionParser::parse(&xml)
            .unwrap_or_default()
            .interfaces
            .into_iter()
            .flat_map(|i| i.methods)
            .collect()
    }

    fn enhance_methods(&self, methods: Vec<MethodInfo>) -> Vec<MethodInfo> {
        // Add custom enhancements
        methods.into_iter().map(|mut method| {
            // Add descriptions based on method names
            method.description = Some(self.generate_description(&method.name));
            
            // Add parameter validation
            method.validation = Some(self.generate_validation(&method));
            
            method
        }).collect()
    }

    fn generate_description(&self, method_name: &str) -> String {
        // Generate human-readable descriptions
        match method_name {
            "Start" => "Start the service or operation".to_string(),
            "Stop" => "Stop the service or operation".to_string(),
            "GetStatus" => "Retrieve current status information".to_string(),
            _ => format!("Execute {} operation", method_name),
        }
    }

    fn generate_validation(&self, method: &MethodInfo) -> ValidationRules {
        // Generate validation rules based on method signature
        ValidationRules {
            required_params: method.in_args.iter()
                .filter(|a| !a.optional)
                .map(|a| a.name.clone())
                .collect(),
            param_types: method.in_args.iter()
                .map(|a| (a.name.clone(), a.type_sig.clone()))
                .collect(),
        }
    }
}
```

---

## Testing Framework

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_agent_creation() {
        let agent = CustomAgent::new("test-id".to_string());
        assert_eq!(agent.agent_id, "test-id");
    }

    #[test]
    async fn test_task_execution() {
        let mut agent = CustomAgent::new("test-id".to_string());
        
        let task = Task {
            id: "task-1".to_string(),
            operation: "example".to_string(),
            params: {
                let mut p = HashMap::new();
                p.insert("input".to_string(), json!("test"));
                p
            },
        };

        let result = agent.handle_operation(&task).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: TEST");
    }

    #[test]
    async fn test_invalid_operation() {
        let mut agent = CustomAgent::new("test-id".to_string());
        
        let task = Task {
            id: "task-2".to_string(),
            operation: "invalid".to_string(),
            params: HashMap::new(),
        };

        let result = agent.handle_operation(&task).await;
        assert!(result.is_err());
    }
}
```

### Integration Testing

Create `tests/mcp_integration.rs`:

```rust
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_mcp_server_lifecycle() {
    // Start MCP server
    let mut server = Command::new("target/debug/dbus-mcp")
        .spawn()
        .expect("Failed to start MCP server");

    // Wait for startup
    sleep(Duration::from_secs(2)).await;

    // Test connection
    let client = MCPClient::new();
    let init_result = client.initialize().await;
    assert!(init_result.is_ok());

    // Test tool listing
    let tools = client.list_tools().await;
    assert!(!tools.unwrap().is_empty());

    // Cleanup
    server.kill().expect("Failed to stop server");
}

#[tokio::test]
async fn test_agent_orchestration() {
    // Start orchestrator
    let mut orchestrator = Command::new("target/debug/dbus-orchestrator")
        .spawn()
        .expect("Failed to start orchestrator");

    sleep(Duration::from_secs(2)).await;

    // Connect via D-Bus
    let connection = Connection::session().await.unwrap();
    let proxy = OrchestratorProxy::new(&connection).await.unwrap();

    // Spawn agent
    let agent_id = proxy.spawn_agent("custom".to_string()).await.unwrap();
    assert!(!agent_id.is_empty());

    // Send task
    let task = json!({
        "id": "test-task",
        "operation": "example",
        "params": {"input": "test"}
    });

    let result = proxy.send_task(agent_id.clone(), task.to_string()).await;
    assert!(result.is_ok());

    // Cleanup
    proxy.kill_agent(agent_id).await.ok();
    orchestrator.kill().ok();
}
```

### Mock Testing

```rust
use mockall::*;

#[automock]
trait AgentInterface {
    async fn execute_task(&self, task: String) -> String;
    fn get_status(&self) -> String;
}

#[test]
async fn test_with_mock_agent() {
    let mut mock = MockAgentInterface::new();
    
    mock.expect_execute_task()
        .with(predicate::eq("test_task".to_string()))
        .times(1)
        .returning(|_| "success".to_string());

    let result = mock.execute_task("test_task".to_string()).await;
    assert_eq!(result, "success");
}
```

---

## Performance Optimization

### Connection Pooling

```rust
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

static CONNECTION_POOL: Lazy<Arc<RwLock<ConnectionPool>>> = 
    Lazy::new(|| Arc::new(RwLock::new(ConnectionPool::new())));

struct ConnectionPool {
    connections: HashMap<String, Connection>,
    max_connections: usize,
}

impl ConnectionPool {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            max_connections: 10,
        }
    }

    async fn get_connection(&mut self, name: &str) -> Result<Connection, Error> {
        if let Some(conn) = self.connections.get(name) {
            if conn.is_alive() {
                return Ok(conn.clone());
            }
        }

        // Create new connection
        let conn = Connection::session().await?;
        
        // Store if under limit
        if self.connections.len() < self.max_connections {
            self.connections.insert(name.to_string(), conn.clone());
        }

        Ok(conn)
    }
}
```

### Caching Strategy

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

struct MethodCache {
    cache: LruCache<String, CachedMethod>,
}

struct CachedMethod {
    result: Value,
    timestamp: Instant,
    ttl: Duration,
}

impl MethodCache {
    fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    fn get(&mut self, key: &str) -> Option<Value> {
        if let Some(cached) = self.cache.get(key) {
            if cached.timestamp.elapsed() < cached.ttl {
                return Some(cached.result.clone());
            }
        }
        None
    }

    fn put(&mut self, key: String, result: Value, ttl: Duration) {
        self.cache.put(key, CachedMethod {
            result,
            timestamp: Instant::now(),
            ttl,
        });
    }
}
```

### Async Optimization

```rust
use futures::stream::{self, StreamExt};

async fn parallel_tool_execution(tools: Vec<ToolCall>) -> Vec<Result<Value, Error>> {
    // Execute tools in parallel with concurrency limit
    stream::iter(tools)
        .map(|tool| async move {
            execute_tool(tool).await
        })
        .buffer_unordered(5) // Max 5 concurrent executions
        .collect()
        .await
}
```

---

## Security Considerations

### Input Validation

```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Validate, Deserialize)]
struct SecureInput {
    #[validate(length(min = 1, max = 255))]
    #[validate(regex = "SAFE_PATTERN")]
    name: String,
    
    #[validate(range(min = 0, max = 65535))]
    port: u16,
    
    #[validate(custom = "validate_path")]
    path: String,
}

fn validate_path(path: &str) -> Result<(), ValidationError> {
    // Prevent directory traversal
    if path.contains("..") {
        return Err(ValidationError::new("invalid_path"));
    }
    
    // Check against allowlist
    let allowed_prefixes = vec!["/home", "/tmp", "/var/log"];
    if !allowed_prefixes.iter().any(|p| path.starts_with(p)) {
        return Err(ValidationError::new("forbidden_path"));
    }
    
    Ok(())
}
```

### Sandboxing

```rust
use nix::unistd::{setuid, setgid, Uid, Gid};
use nix::sys::resource::{setrlimit, Resource, Rlimit};

fn sandbox_agent() -> Result<(), Box<dyn std::error::Error>> {
    // Drop privileges
    let uid = Uid::from_raw(1000); // Non-root user
    let gid = Gid::from_raw(1000);
    
    setgid(gid)?;
    setuid(uid)?;
    
    // Set resource limits
    let memory_limit = Rlimit {
        rlim_cur: 100 * 1024 * 1024, // 100 MB
        rlim_max: 100 * 1024 * 1024,
    };
    setrlimit(Resource::RLIMIT_AS, memory_limit)?;
    
    // CPU time limit
    let cpu_limit = Rlimit {
        rlim_cur: 30, // 30 seconds
        rlim_max: 30,
    };
    setrlimit(Resource::RLIMIT_CPU, cpu_limit)?;
    
    Ok(())
}
```

### Audit Logging

```rust
use tracing::{info, warn, error};
use serde_json::json;

struct AuditLogger;

impl AuditLogger {
    fn log_operation(&self, operation: &str, user: &str, params: &Value, result: &Result<Value, Error>) {
        let log_entry = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "operation": operation,
            "user": user,
            "params": params,
            "success": result.is_ok(),
            "error": result.as_ref().err().map(|e| e.to_string()),
        });

        match result {
            Ok(_) => info!("Audit: {}", log_entry),
            Err(_) => warn!("Audit: {}", log_entry),
        }
    }
}
```

---

## Best Practices

### Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum MCPError {
    #[error("D-Bus error: {0}")]
    DBus(#[from] zbus::Error),
    
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
    
    #[error("Operation not permitted: {0}")]
    NotPermitted(String),
    
    #[error("Timeout after {0} seconds")]
    Timeout(u64),
    
    #[error("Agent error: {0}")]
    Agent(String),
}

// Use Result type alias for cleaner code
type MCPResult<T> = Result<T, MCPError>;
```

### Logging

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
```

### Configuration Management

```rust
use config::{Config, File, Environment};

#[derive(Debug, Deserialize)]
struct MCPConfig {
    server: ServerConfig,
    agents: AgentConfig,
    security: SecurityConfig,
}

impl MCPConfig {
    fn load() -> Result<Self, config::ConfigError> {
        Config::builder()
            .add_source(File::with_name("mcp-configs/config.toml"))
            .add_source(Environment::with_prefix("MCP"))
            .build()?
            .try_deserialize()
    }
}
```

---

This completes the comprehensive developer guide for extending the MCP integration.