use serde::{Deserialize, Serialize};
use std::process::Command;
use zbus::{dbus_interface, ConnectionBuilder, SignalContext};

#[derive(Debug, Deserialize)]
struct SystemdTask {
    #[serde(rename = "type")]
    task_type: String,
    service: String,
    action: String, // start, stop, restart, status, enable, disable
}

struct SystemdAgent {
    agent_id: String,
}

#[dbus_interface(name = "org.dbusmcp.Agent.Systemd")]
impl SystemdAgent {
    /// Execute a systemd service management task
    async fn execute(&self, task_json: String) -> zbus::fdo::Result<String> {
        println!("[{}] Received task: {}", self.agent_id, task_json);

        let task: SystemdTask = match serde_json::from_str(&task_json) {
            Ok(t) => t,
            Err(e) => {
                return Err(zbus::fdo::Error::InvalidArgs(format!(
                    "Failed to parse task: {}",
                    e
                )));
            }
        };

        if task.task_type != "systemd" {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "Unknown task type: {}",
                task.task_type
            )));
        }

        // Validate action
        let valid_actions = [
            "start",
            "stop",
            "restart",
            "status",
            "enable",
            "disable",
            "is-active",
            "is-enabled",
        ];
        if !valid_actions.contains(&task.action.as_str()) {
            return Err(zbus::fdo::Error::InvalidArgs(format!(
                "Invalid action: {}. Valid actions: {:?}",
                task.action, valid_actions
            )));
        }

        println!(
            "[{}] Managing service: {} action: {}",
            self.agent_id, task.service, task.action
        );

        let output = Command::new("systemctl")
            .arg(&task.action)
            .arg(&task.service)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                let result = serde_json::json!({
                    "success": exit_code == 0,
                    "exit_code": exit_code,
                    "service": task.service,
                    "action": task.action,
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string(),
                });

                Ok(result.to_string())
            }
            Err(e) => Err(zbus::fdo::Error::Failed(format!(
                "Failed to execute systemctl: {}",
                e
            ))),
        }
    }

    /// Get agent status
    async fn get_status(&self) -> zbus::fdo::Result<String> {
        Ok(format!("Systemd agent {} is running", self.agent_id))
    }

    /// Signal emitted when task completes
    #[dbus_interface(signal)]
    async fn task_completed(signal_ctx: &SignalContext<'_>, result: String) -> zbus::Result<()>;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let agent_id = if args.len() > 1 {
        args[1].clone()
    } else {
        format!(
            "systemd-{}",
            uuid::Uuid::new_v4().to_string()[..8].to_string()
        )
    };

    println!("Starting Systemd Agent: {}", agent_id);

    let agent = SystemdAgent {
        agent_id: agent_id.clone(),
    };

    let path = format!("/org/dbusmcp/Agent/Systemd/{}", agent_id.replace('-', "_"));
    let service_name = format!("org.dbusmcp.Agent.Systemd.{}", agent_id.replace('-', "_"));

    let _conn = ConnectionBuilder::session()?
        .name(service_name.as_str())?
        .serve_at(path.as_str(), agent)?
        .build()
        .await?;

    println!("Systemd agent {} ready on D-Bus", agent_id);
    println!("Service: {}", service_name);
    println!("Path: {}", path);

    // Keep running
    std::future::pending::<()>().await;

    Ok(())
}
