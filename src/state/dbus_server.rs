//! D-Bus server for system bus integration

use crate::state::StateManager;
use anyhow::Result;
use std::sync::Arc;
use zbus::{interface, ConnectionBuilder};

/// D-Bus interface for the state manager
pub struct StateManagerDBus {
    state_manager: Arc<StateManager>,
}

#[interface(name = "org.opdbus.StateManager")]
impl StateManagerDBus {
    /// Apply state from JSON string
    async fn apply_state(&self, state_json: String) -> zbus::fdo::Result<String> {
        match serde_json::from_str(&state_json) {
            Ok(desired_state) => match self.state_manager.apply_state(desired_state).await {
                Ok(report) => Ok(format!("Applied successfully: {}", report.success)),
                Err(e) => Err(zbus::fdo::Error::Failed(format!("Apply failed: {}", e))),
            },
            Err(e) => Err(zbus::fdo::Error::InvalidArgs(format!(
                "Invalid JSON: {}",
                e
            ))),
        }
    }

    /// Query current state
    async fn query_state(&self) -> zbus::fdo::Result<String> {
        match self.state_manager.query_current_state().await {
            Ok(state) => match serde_json::to_string(&state) {
                Ok(json) => Ok(json),
                Err(e) => Err(zbus::fdo::Error::Failed(format!(
                    "Serialization failed: {}",
                    e
                ))),
            },
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Query failed: {}", e))),
        }
    }
}

/// Start the system bus D-Bus service
pub async fn start_system_bus(state_manager: Arc<StateManager>) -> Result<()> {
    let dbus_interface = StateManagerDBus { state_manager };

    let _connection = ConnectionBuilder::system()?
        .name("org.opdbus")?
        .serve_at("/org/opdbus/state", dbus_interface)?
        .build()
        .await?;

    // Keep the connection alive
    std::future::pending::<()>().await;

    Ok(())
}
