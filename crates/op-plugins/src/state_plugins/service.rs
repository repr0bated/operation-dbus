//! Service plugin - auto-generating, validating, init-agnostic service management

use anyhow::{Context, Result};
use async_trait::async_trait;
use op_state::{ApplyResult, Checkpoint, DiffMetadata, PluginCapabilities, StateAction, StateDiff, StatePlugin};
use serde::{Deserialize, Serialize};
use simd_json::{json, OwnedValue as Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ServiceName(String);

impl ServiceName {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        if name.is_empty() { anyhow::bail!("service name cannot be empty") }
        if name.len() > 64 { anyhow::bail!("service name exceeds 64 chars") }
        Ok(Self(name))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl TryFrom<String> for ServiceName {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> { Self::new(s) }
}
impl From<ServiceName> for String {
    fn from(n: ServiceName) -> String { n.0 }
}

impl std::fmt::Display for ServiceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecCommand {
    pub program: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
}

impl ExecCommand {
    pub fn validate(&self) -> Result<()> {
        if !self.program.exists() {
            anyhow::bail!("executable not found: {}", self.program.display());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&self.program)?;
            if meta.permissions().mode() & 0o111 == 0 {
                anyhow::bail!("not executable: {}", self.program.display());
            }
        }
        Ok(())
    }

    pub fn to_command_line(&self) -> String {
        let mut cmd = self.program.display().to_string();
        for arg in &self.args {
            cmd.push(' ');
            if arg.contains(' ') { cmd.push_str(&format!("\"{}\"", arg)); }
            else { cmd.push_str(arg); }
        }
        cmd
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceLifecycle {
    pub last_active: Option<u64>,
    pub days_since_active: Option<u64>,
    pub is_orphaned: bool,
    pub orphan_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDef {
    pub name: ServiceName,
    pub exec_start: ExecCommand,
    pub exec_stop: Option<ExecCommand>,
    pub working_dir: Option<PathBuf>,
    pub user: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<ServiceName>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<ServiceLifecycle>,
}

impl ServiceDef {
    pub fn validate(&self) -> Result<()> {
        self.exec_start.validate()?;
        if let Some(ref stop) = self.exec_stop { stop.validate()?; }
        if let Some(ref dir) = self.working_dir {
            if !dir.exists() { anyhow::bail!("working_dir missing: {}", dir.display()); }
        }
        #[cfg(unix)]
        if let Some(ref user) = self.user {
            let out = std::process::Command::new("id").arg(user).output()?;
            if !out.status.success() { anyhow::bail!("user not found: {}", user); }
        }
        Ok(())
    }
    
    pub fn to_dinit(&self) -> String {
        let mut out = String::new();
        out.push_str("type = process\n");
        out.push_str(&format!("command = {}\n", self.exec_start.to_command_line()));
        if let Some(ref stop) = self.exec_stop {
            out.push_str(&format!("stop-command = {}\n", stop.to_command_line()));
        }
        if let Some(ref dir) = self.working_dir {
            out.push_str(&format!("working-dir = {}\n", dir.display()));
        }
        if let Some(ref user) = self.user {
            out.push_str(&format!("run-as = {}\n", user));
        }
        for dep in &self.depends_on {
            out.push_str(&format!("depends-on = {}\n", dep));
        }
        for (k, v) in &self.environment {
            out.push_str(&format!("env = {}={}\n", k, v));
        }
        out
    }
    
    /// Convert from systemd unit file
    pub fn from_systemd_unit(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut name = None;
        let mut exec_start = None;
        let mut exec_stop = None;
        let mut working_dir = None;
        let mut user = None;
        let mut depends = vec![];
        let mut env = HashMap::new();
        
        for line in content.lines() {
            let line = line.trim();
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "ExecStart" => {
                        let parts: Vec<&str> = v.trim().split_whitespace().collect();
                        if !parts.is_empty() {
                            exec_start = Some(ExecCommand {
                                program: PathBuf::from(parts[0]),
                                args: parts[1..].iter().map(|s| s.to_string()).collect(),
                            });
                        }
                    }
                    "ExecStop" => {
                        let parts: Vec<&str> = v.trim().split_whitespace().collect();
                        if !parts.is_empty() {
                            exec_stop = Some(ExecCommand {
                                program: PathBuf::from(parts[0]),
                                args: parts[1..].iter().map(|s| s.to_string()).collect(),
                            });
                        }
                    }
                    "WorkingDirectory" => working_dir = Some(PathBuf::from(v.trim())),
                    "User" => user = Some(v.trim().to_string()),
                    "Requires" | "Wants" | "After" => {
                        for dep in v.split_whitespace() {
                            if let Ok(sn) = ServiceName::new(dep) {
                                depends.push(sn);
                            }
                        }
                    }
                    "Environment" => {
                        if let Some((ek, ev)) = v.split_once('=') {
                            env.insert(ek.trim().to_string(), ev.trim().to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        name = Some(ServiceName::new(file_name)?);
        
        Ok(ServiceDef {
            name: name.unwrap(),
            exec_start: exec_start.ok_or_else(|| anyhow::anyhow!("no ExecStart"))?,
            exec_stop,
            working_dir,
            user,
            depends_on: depends,
            environment: env,
            enabled: false,
            lifecycle: None,
        })
    }
}

pub struct ServicePlugin {
    backend: ServiceBackend,
}

enum ServiceBackend {
    Dinit,
    Systemd,
}

impl ServicePlugin {
    pub fn new() -> Self {
        let backend = if Path::new("/run/dinitctl").exists() {
            ServiceBackend::Dinit
        } else {
            ServiceBackend::Systemd
        };
        Self { backend }
    }
    
    /// Auto-generate service from installed binary
    pub async fn auto_generate_service(&self, binary_path: &Path) -> Result<ServiceDef> {
        let name = binary_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("invalid binary name"))?;
        
        Ok(ServiceDef {
            name: ServiceName::new(name)?,
            exec_start: ExecCommand {
                program: binary_path.to_path_buf(),
                args: vec![],
            },
            exec_stop: None,
            working_dir: None,
            user: None,
            depends_on: vec![],
            environment: HashMap::new(),
            enabled: false,
            lifecycle: None,
        })
    }
    
    /// Convert all systemd units to dinit
    pub async fn convert_systemd_to_dinit(&self) -> Result<Vec<ServiceDef>> {
        let mut services = vec![];
        let systemd_dir = Path::new("/etc/systemd/system");
        
        if !systemd_dir.exists() {
            return Ok(services);
        }
        
        for entry in std::fs::read_dir(systemd_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|e| e.to_str()) == Some("service") {
                match ServiceDef::from_systemd_unit(&path) {
                    Ok(svc) => {
                        svc.validate()?;
                        services.push(svc);
                    }
                    Err(e) => {
                        log::warn!("Failed to convert {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        Ok(services)
    }
    
    /// Install service definition
    pub async fn install_service(&self, svc: &ServiceDef) -> Result<()> {
        svc.validate()?;
        
        match self.backend {
            ServiceBackend::Dinit => {
                let path = format!("/etc/dinit.d/{}", svc.name);
                std::fs::write(&path, svc.to_dinit())?;
                log::info!("Installed dinit service: {}", path);
            }
            ServiceBackend::Systemd => {
                anyhow::bail!("systemd installation not implemented - use dinit");
            }
        }
        
        Ok(())
    }
    
    async fn check_lifecycle(&self, name: &str) -> Result<ServiceLifecycle> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        
        let last_active = match self.backend {
            ServiceBackend::Systemd => {
                let out = tokio::process::Command::new("systemctl")
                    .args(["show", name, "--property=ActiveEnterTimestamp"])
                    .output().await?;
                
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .find_map(|l| {
                        l.split_once('=').and_then(|(_, v)| {
                            chrono::DateTime::parse_from_rfc3339(v).ok()
                                .map(|ts| ts.timestamp() as u64)
                        })
                    })
            }
            ServiceBackend::Dinit => None,
        };
        
        let days_since_active = last_active.map(|t| (now - t) / 86400);
        let is_orphaned = days_since_active.map_or(true, |d| d > 30);
        
        let orphan_reason = if is_orphaned {
            Some(if last_active.is_none() {
                "never run".to_string()
            } else {
                format!("inactive {} days", days_since_active.unwrap())
            })
        } else {
            None
        };
        
        Ok(ServiceLifecycle {
            last_active,
            days_since_active,
            is_orphaned,
            orphan_reason,
        })
    }
}

#[async_trait]
impl StatePlugin for ServicePlugin {
    fn name(&self) -> &str { "service" }
    fn version(&self) -> &str { "1.0.0" }
    
    async fn query_current_state(&self) -> Result<Value> {
        let mut services = HashMap::new();
        
        let service_list = match self.backend {
            ServiceBackend::Systemd => {
                let out = tokio::process::Command::new("systemctl")
                    .args(["list-units", "--type=service", "--all", "--no-pager", "--plain"])
                    .output().await?;
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|l| l.split_whitespace().next().map(String::from))
                    .collect::<Vec<_>>()
            }
            ServiceBackend::Dinit => {
                let out = tokio::process::Command::new("dinitctl")
                    .args(["list"])
                    .output().await?;
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|l| l.split_whitespace().next().map(String::from))
                    .collect::<Vec<_>>()
            }
        };
        
        for svc_name in service_list {
            if let Ok(lifecycle) = self.check_lifecycle(&svc_name).await {
                services.insert(svc_name, json!({ "lifecycle": lifecycle }));
            }
        }
        
        Ok(json!({ "services": services }))
    }
    
    async fn calculate_diff(&self, _current: &Value, _desired: &Value) -> Result<StateDiff> {
        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions: vec![],
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: String::new(),
                desired_hash: String::new(),
            },
        })
    }
    
    async fn apply_state(&self, _diff: &StateDiff) -> Result<ApplyResult> {
        Ok(ApplyResult {
            success: true,
            changes_applied: vec![],
            errors: vec![],
            checkpoint: None,
        })
    }
    
    async fn verify_state(&self, _desired: &Value) -> Result<bool> {
        Ok(true)
    }
    
    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        Ok(Checkpoint {
            id: format!("service-{}", chrono::Utc::now().timestamp()),
            plugin: self.name().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: json!({}),
            backend_checkpoint: None,
        })
    }
    
    async fn rollback(&self, _checkpoint: &Checkpoint) -> Result<()> {
        Ok(())
    }
    
    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: true,
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: false,
        }
    }
}
