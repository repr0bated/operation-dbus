use serde::{Deserialize, Serialize};
use std::process::Command;
use zbus::{dbus_interface, ConnectionBuilder, SignalContext};

#[derive(Debug, Deserialize)]
struct ExecuteTask {
    #[serde(rename = "type")]
    task_type: String,
    command: String,
    timeout: Option<u64>,
}

struct ExecutorAgent {
    agent_id: String,
}

#[dbus_interface(name = "org.dbusmcp.Agent.Executor")]
impl ExecutorAgent {
    /// Execute a command
    async fn execute(&self, task_json: String) -> zbus::fdo::Result<String> {
        println!("[{}] Received task: {}", self.agent_id, task_json);

        let task: ExecuteTask = match serde_json::from_str(&task_json) {
            Ok(t) => t,
            Err(e) => {
                return Err(zbus::fdo::Error::InvalidArgs(
                    format!("Failed to parse task: {}", e)
                ));
            }
        };

        if task.task_type != "execute" {
            return Err(zbus::fdo::Error::InvalidArgs(
                format!("Unknown task type: {}", task.task_type)
            ));
        }

        println!("[{}] Executing command: {}", self.agent_id, task.command);

        let output = Command::new("sh")
            .arg("-c")
            .arg(&task.command)
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                let result = serde_json::json!({
                    "success": exit_code == 0,
                    "exit_code": exit_code,
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string(),
                });

                Ok(result.to_string())
            }
            Err(e) => {
                Err(zbus::fdo::Error::Failed(
                    format!("Failed to execute command: {}", e)
                ))
            }
        }
    }

    /// Get agent status
    async fn get_status(&self) -> zbus::fdo::Result<String> {
        Ok(format!("Executor agent {} is running", self.agent_id))
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
        format!("executor-{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
    };

    println!("Starting Executor Agent: {}", agent_id);

    let agent = ExecutorAgent {
        agent_id: agent_id.clone(),
    };

    let path = format!("/org/dbusmcp/Agent/Executor/{}", agent_id.replace('-', "_"));
    let service_name = format!("org.dbusmcp.Agent.Executor.{}", agent_id.replace('-', "_"));

    let _conn = ConnectionBuilder::session()?
        .name(service_name.as_str())?
        .serve_at(path.as_str(), agent)?
        .build()
        .await?;

    println!("Executor agent {} ready on D-Bus", agent_id);
    println!("Service: {}", service_name);
    println!("Path: {}", path);

    // Keep running
    std::future::pending::<()>().await;

    Ok(())
}
