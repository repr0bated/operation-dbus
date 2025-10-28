use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use zbus::{dbus_interface, ConnectionBuilder, SignalContext};

#[derive(Debug, Deserialize)]
struct FileTask {
    #[serde(rename = "type")]
    task_type: String,
    operation: String, // read, write, delete, exists, list, mkdir
    path: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    recursive: bool,
}

struct FileAgent {
    agent_id: String,
}

#[dbus_interface(name = "org.dbusmcp.Agent.File")]
impl FileAgent {
    /// Execute a file operation task
    async fn execute(&self, task_json: String) -> zbus::fdo::Result<String> {
        println!("[{}] Received task: {}", self.agent_id, task_json);

        let task: FileTask = match serde_json::from_str(&task_json) {
            Ok(t) => t,
            Err(e) => {
                return Err(zbus::fdo::Error::InvalidArgs(
                    format!("Failed to parse task: {}", e)
                ));
            }
        };

        if task.task_type != "file" {
            return Err(zbus::fdo::Error::InvalidArgs(
                format!("Unknown task type: {}", task.task_type)
            ));
        }

        println!("[{}] File operation: {} on path: {}", self.agent_id, task.operation, task.path);

        let result = match task.operation.as_str() {
            "read" => self.read_file(&task.path),
            "write" => self.write_file(&task.path, task.content.as_deref()),
            "delete" => self.delete_file(&task.path, task.recursive),
            "exists" => self.file_exists(&task.path),
            "list" => self.list_directory(&task.path),
            "mkdir" => self.create_directory(&task.path, task.recursive),
            _ => Err(format!("Unknown file operation: {}", task.operation)),
        };

        match result {
            Ok(data) => {
                let response = serde_json::json!({
                    "success": true,
                    "operation": task.operation,
                    "path": task.path,
                    "data": data,
                });
                Ok(response.to_string())
            }
            Err(e) => {
                Err(zbus::fdo::Error::Failed(e))
            }
        }
    }

    /// Get agent status
    async fn get_status(&self) -> zbus::fdo::Result<String> {
        Ok(format!("File agent {} is running", self.agent_id))
    }

    /// Signal emitted when task completes
    #[dbus_interface(signal)]
    async fn task_completed(signal_ctx: &SignalContext<'_>, result: String) -> zbus::Result<()>;
}

impl FileAgent {
    fn read_file(&self, path: &str) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
    }

    fn write_file(&self, path: &str, content: Option<&str>) -> Result<String, String> {
        let content = content.ok_or("Content is required for write operation")?;
        fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(format!("Wrote {} bytes", content.len()))
    }

    fn delete_file(&self, path: &str, recursive: bool) -> Result<String, String> {
        let path_obj = Path::new(path);
        if path_obj.is_dir() {
            if recursive {
                fs::remove_dir_all(path).map_err(|e| format!("Failed to delete directory: {}", e))?;
            } else {
                fs::remove_dir(path).map_err(|e| format!("Failed to delete directory: {}", e))?;
            }
        } else {
            fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))?;
        }
        Ok("Deleted successfully".to_string())
    }

    fn file_exists(&self, path: &str) -> Result<String, String> {
        let exists = Path::new(path).exists();
        Ok(exists.to_string())
    }

    fn list_directory(&self, path: &str) -> Result<String, String> {
        let entries = fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))?;
        let mut files = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }
        Ok(serde_json::to_string(&files).unwrap())
    }

    fn create_directory(&self, path: &str, recursive: bool) -> Result<String, String> {
        if recursive {
            fs::create_dir_all(path).map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            fs::create_dir(path).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
        Ok("Directory created successfully".to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let agent_id = if args.len() > 1 {
        args[1].clone()
    } else {
        format!("file-{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
    };

    println!("Starting File Agent: {}", agent_id);

    let agent = FileAgent {
        agent_id: agent_id.clone(),
    };

    let path = format!("/org/dbusmcp/Agent/File/{}", agent_id.replace('-', "_"));
    let service_name = format!("org.dbusmcp.Agent.File.{}", agent_id.replace('-', "_"));

    let _conn = ConnectionBuilder::session()?
        .name(service_name.as_str())?
        .serve_at(path.as_str(), agent)?
        .build()
        .await?;

    println!("File agent {} ready on D-Bus", agent_id);
    println!("Service: {}", service_name);
    println!("Path: {}", path);

    // Keep running
    std::future::pending::<()>().await;

    Ok(())
}
