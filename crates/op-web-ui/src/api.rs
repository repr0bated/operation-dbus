//! API client for backend communication

use serde::{Deserialize, Serialize};
use gloo_net::http::Request;

// ============================================================================
// Types
// ============================================================================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemService {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub sub_state: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub uptime: u64,
    #[serde(default)]
    pub cpu: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DbusService {
    pub name: String,
    pub category: String,
    pub bus: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemDiagnostics {
    pub hostname: String,
    pub load: LoadAvg,
    pub memory: MemoryInfo,
    pub uptime: UptimeInfo,
    pub cpu_cores: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadAvg {
    #[serde(rename = "1min")]
    pub one_min: f64,
    #[serde(rename = "5min")]
    pub five_min: f64,
    #[serde(rename = "15min")]
    pub fifteen_min: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_kb: u64,
    pub percent_used: u32,
    pub formatted: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UptimeInfo {
    pub seconds: u64,
    pub formatted: String,
}

pub use op_plugins::state_plugins::ToolDefinition;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_name: String,
    pub args: simd_json::OwnedValue,
    pub timestamp: String,
    pub status: String,
    #[serde(default)]
    pub result: Option<simd_json::OwnedValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: String,
    #[serde(default)]
    pub actions_taken: Vec<ActionTaken>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionTaken {
    pub tool: String,
    pub params: simd_json::OwnedValue,
    pub success: bool,
    #[serde(default)]
    pub execution_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub tool_name: String,
    pub args: simd_json::OwnedValue,
    pub rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpGroup {
    pub id: String,
    pub name: String,
    pub agents: Vec<String>,
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub tunables: simd_json::OwnedValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemPromptConfig {
    pub immutable_section: String,
    pub tunable_section: String,
}

// ============================================================================
// Aggregate data for initial load
// ============================================================================

pub struct AllData {
    pub services: Vec<SystemService>,
    pub dbus_services: Vec<DbusService>,
    pub diagnostics: SystemDiagnostics,
    pub tools: Vec<ToolDefinition>,
}

// ============================================================================
// API Functions
// ============================================================================

pub async fn fetch_all_data() -> Result<AllData, String> {
    let services_result: Result<Vec<SystemService>, String> = fetch_services().await;
    let dbus_result: Result<Vec<DbusService>, String> = fetch_dbus_services().await;
    let diag_result: Result<SystemDiagnostics, String> = fetch_diagnostics().await;
    let tools_result: Result<Vec<ToolDefinition>, String> = fetch_tools().await;

    Ok(AllData {
        services: services_result.unwrap_or_default(),
        dbus_services: dbus_result.unwrap_or_default(),
        diagnostics: diag_result?,
        tools: tools_result.unwrap_or_default(),
    })
}

pub async fn fetch_services() -> Result<Vec<SystemService>, String> {
    // Use /api/status endpoint which exists
    #[derive(Deserialize)]
    struct StatusResponse {
        services: Option<Vec<SystemService>>
    }

    let resp = Request::get("/api/status")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<StatusResponse>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.services.unwrap_or_default())
}

pub async fn fetch_dbus_services() -> Result<Vec<DbusService>, String> {
    // Return empty for now - endpoint doesn't exist
    Ok(vec![])
}

pub async fn fetch_diagnostics() -> Result<SystemDiagnostics, String> {
    // Use /api/status endpoint
    #[derive(Deserialize)]
    struct StatusResponse {
        system: Option<SystemInfo>
    }

    #[derive(Deserialize)]
    struct SystemInfo {
        hostname: String,
        load_average: Option<[f64; 3]>,
        memory_total_mb: Option<u64>,
        memory_used_mb: Option<u64>,
        uptime_secs: Option<u64>,
        cpu_count: Option<u32>,
    }

    let resp = Request::get("/api/status")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<StatusResponse>()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(sys) = resp.system {
        let load = sys.load_average.unwrap_or([0.0, 0.0, 0.0]);
        let mem_total = sys.memory_total_mb.unwrap_or(0) * 1024;
        let mem_used = sys.memory_used_mb.unwrap_or(0) * 1024;
        let uptime_secs = sys.uptime_secs.unwrap_or(0);

        Ok(SystemDiagnostics {
            hostname: sys.hostname,
            load: LoadAvg {
                one_min: load[0],
                five_min: load[1],
                fifteen_min: load[2],
            },
            memory: MemoryInfo {
                total_kb: mem_total,
                available_kb: mem_total - mem_used,
                used_kb: mem_used,
                percent_used: if mem_total > 0 { ((mem_used as f64 / mem_total as f64) * 100.0) as u32 } else { 0 },
                formatted: format!("{} / {} MB", mem_used / 1024, mem_total / 1024),
            },
            uptime: UptimeInfo {
                seconds: uptime_secs,
                formatted: format!("{}d {}h", uptime_secs / 86400, (uptime_secs % 86400) / 3600),
            },
            cpu_cores: sys.cpu_count.unwrap_or(1),
        })
    } else {
        Err("No system info in status response".to_string())
    }
}

pub async fn fetch_tools() -> Result<Vec<ToolDefinition>, String> {
    #[derive(Deserialize)]
    struct Response {
        tools: Vec<SimpleTool>
    }

    #[derive(Deserialize)]
    struct SimpleTool {
        name: String,
        description: String,
        category: Option<String>,
    }

    let resp = Request::get("/api/tools")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Response>()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.tools.into_iter().map(|t| ToolDefinition {
        name: t.name,
        description: t.description,
        input_schema: simd_json::json!({}),
        plugin: t.category.unwrap_or_else(|| "other".to_string()),
    }).collect())
}

pub async fn send_chat(message: &str) -> Result<ChatResponse, String> {
    #[derive(Serialize)]
    struct ChatRequest<'a> { message: &'a str }
    
    Request::post("/api/chat")
        .header("Content-Type", "application/json")
        .body(simd_json::to_string(&ChatRequest { message }).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<ChatResponse>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn execute_tool(tool_name: &str, args: simd_json::OwnedValue) -> Result<simd_json::OwnedValue, String> {
    #[derive(Serialize)]
    struct ToolRequest<'a> {
        tool_name: &'a str,
        arguments: simd_json::OwnedValue,
    }
    
    #[derive(Deserialize)]
    struct ToolResponse {
        success: bool,
        result: Option<simd_json::OwnedValue>,
        error: Option<String>,
    }
    
    let resp = Request::post("/api/tools/execute")
        .header("Content-Type", "application/json")
        .body(simd_json::to_string(&ToolRequest { tool_name, arguments: args }).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<ToolResponse>()
        .await
        .map_err(|e| e.to_string())?;
    
    if resp.success {
        Ok(resp.result.unwrap_or_else(|| simd_json::json!(null)))
    } else {
        Err(resp.error.unwrap_or_else(|| "Unknown error".to_string()))
    }
}

pub async fn fetch_system_prompt() -> Result<SystemPromptConfig, String> {
    // Placeholder - would fetch from /api/system/prompt
    Ok(SystemPromptConfig {
        immutable_section: r#"You are the OP-DBUS cognitive control plane.
You do NOT execute tools directly - you request execution through MCP.
Database state is AUTHORITATIVE. You may only read, never assume.
Tools are the ONLY legal mutation mechanism."#.to_string(),
        tunable_section: r#"# Current Context
- System: Linux
- Mode: Production
- Streaming: Enabled"#.to_string(),
    })
}

pub async fn fetch_mcp_groups() -> Result<Vec<McpGroup>, String> {
    // Placeholder - would fetch from /api/mcp/groups
    Ok(vec![
        McpGroup {
            id: "grp-1".to_string(),
            name: "Core Agents".to_string(),
            agents: vec!["rust-pro".to_string(), "backend-architect".to_string()],
            active: true,
        },
        McpGroup {
            id: "grp-2".to_string(),
            name: "Cognitive".to_string(),
            agents: vec!["sequential-thinking".to_string(), "memory".to_string()],
            active: true,
        },
    ])
}

pub async fn fetch_plugins() -> Result<Vec<PluginConfig>, String> {
    // Placeholder - would fetch from /api/plugins
    Ok(vec![
        PluginConfig {
            name: "network".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            tunables: simd_json::json!({"max_interfaces": 100}),
        },
        PluginConfig {
            name: "storage".to_string(),
            version: "1.0.0".to_string(),
            enabled: true,
            tunables: simd_json::json!({"cache_size_mb": 512}),
        },
    ])
}
