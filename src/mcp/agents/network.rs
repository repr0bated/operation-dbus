use serde::Deserialize;
use std::process::Command;
use uuid::Uuid;
use zbus::{interface, connection::Builder, object_server::SignalEmitter};

// Security configuration
const FORBIDDEN_CHARS: &[char] = &['$', '`', ';', '&', '|', '>', '<', '(', ')', '{', '}', '\n', '\r'];
const MAX_TARGET_LENGTH: usize = 256;
const MAX_COUNT: u32 = 20;

#[derive(Debug, Deserialize)]
struct NetworkTask {
    #[serde(rename = "type")]
    task_type: String,
    operation: String, // ping, interfaces, connections, ports, route
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    count: Option<u32>,
}

struct NetworkAgent {
    agent_id: String,
}

#[interface(name = "org.dbusmcp.Agent.Network")]
impl NetworkAgent {
    /// Execute a network operation task
    async fn execute(&self, task_json: String) -> zbus::fdo::Result<String> {
        println!("[{}] Received task: {}", self.agent_id, task_json);

        let task: NetworkTask = match serde_json::from_str(&task_json) {
            Ok(t) => t,
            Err(e) => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "Failed to parse task: {}",
                    e
                )));
            }
        };

        if task.task_type != "network" {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "Unknown task type: {}",
                task.task_type
            )));
        }

        println!("[{}] Network operation: {}", self.agent_id, task.operation);

        let result = match task.operation.as_str() {
            "ping" => self.ping(task.target.as_deref(), task.count),
            "interfaces" => self.list_interfaces(),
            "connections" => self.list_connections(),
            "ports" => self.list_ports(),
            "route" => self.show_routes(),
            "dns" => self.check_dns(),
            _ => Err(format!("Unknown network operation: {}", task.operation)),
        };

        match result {
            Ok(data) => {
                let response = serde_json::json!({
                    "success": true,
                    "operation": task.operation,
                    "data": data,
                });
                Ok(response.to_string())
            }
            Err(e) => Err(zbus::fdo::Error::Failed(e)),
        }
    }

    /// Get agent status
    async fn get_status(&self) -> zbus::fdo::Result<String> {
        Ok(format!("Network agent {} is running", self.agent_id))
    }

    /// Signal emitted when task completes
    #[zbus(signal)]
    async fn task_completed(signal_emitter: &SignalEmitter<'_>, result: String) -> zbus::Result<()>;
}

impl NetworkAgent {
    fn validate_target(&self, target: &str) -> Result<(), String> {
        if target.len() > MAX_TARGET_LENGTH {
            return Err(format!(
                "Target exceeds maximum length of {} characters",
                MAX_TARGET_LENGTH
            ));
        }

        if target.trim().is_empty() {
            return Err("Target cannot be empty".to_string());
        }

        for forbidden_char in FORBIDDEN_CHARS {
            if target.contains(*forbidden_char) {
                return Err(format!(
                    "Target contains forbidden character: {:?}",
                    forbidden_char
                ));
            }
        }

        Ok(())
    }

    fn ping(&self, target: Option<&str>, count: Option<u32>) -> Result<String, String> {
        let target = target.ok_or("Target is required for ping operation")?;
        self.validate_target(target)?;
        let count = count.unwrap_or(4).min(MAX_COUNT);

        let output = Command::new("ping")
            .arg("-c")
            .arg(count.to_string())
            .arg(target)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if out.status.success() {
                    Ok(stdout.to_string())
                } else {
                    Err(format!("Ping failed: {}", stderr))
                }
            }
            Err(e) => Err(format!("Failed to execute ping: {}", e)),
        }
    }

    fn list_interfaces(&self) -> Result<String, String> {
        let output = Command::new("ip").arg("addr").output();

        match output {
            Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Err(e) => Err(format!("Failed to list interfaces: {}", e)),
        }
    }

    fn list_connections(&self) -> Result<String, String> {
        let output = Command::new("ss").arg("-tuln").output();

        match output {
            Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Err(e) => Err(format!("Failed to list connections: {}", e)),
        }
    }

    fn list_ports(&self) -> Result<String, String> {
        let output = Command::new("ss").arg("-tulnp").output();

        match output {
            Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Err(e) => Err(format!("Failed to list ports: {}", e)),
        }
    }

    fn show_routes(&self) -> Result<String, String> {
        let output = Command::new("ip").arg("route").output();

        match output {
            Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Err(e) => Err(format!("Failed to show routes: {}", e)),
        }
    }

    fn check_dns(&self) -> Result<String, String> {
        let output = Command::new("resolvectl").arg("status").output();

        match output {
            Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
            Err(e) => {
                // Fallback to old systemd-resolve
                let output_fallback = Command::new("systemd-resolve").arg("--status").output();
                match output_fallback {
                    Ok(out) => Ok(String::from_utf8_lossy(&out.stdout).to_string()),
                    Err(_) => Err(format!("Failed to check DNS: {}", e)),
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let agent_id = if args.len() > 1 {
        args[1].clone()
    } else {
        format!(
            "network-{}",
            Uuid::new_v4().to_string()[..8].to_string()
        )
    };

    println!("Starting Network Agent: {}", agent_id);

    let agent = NetworkAgent {
        agent_id: agent_id.clone(),
    };

    let path = format!("/org/dbusmcp/Agent/Network/{}", agent_id.replace('-', "_"));
    let service_name = format!("org.dbusmcp.Agent.Network.{}", agent_id.replace('-', "_"));

    let _conn = Builder::system()?
        .name(service_name.as_str())?
        .serve_at(path.as_str(), agent)?
        .build()
        .await?;

    println!("Network agent {} ready on D-Bus", agent_id);
    println!("Service: {}", service_name);
    println!("Path: {}", path);

    // Keep running
    std::future::pending::<()>().await;

    Ok(())
}
