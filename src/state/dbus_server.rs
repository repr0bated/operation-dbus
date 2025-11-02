use std::sync::Arc;
use once_cell::sync::OnceCell;
use zbus::{dbus_interface, Connection, ConnectionBuilder};

use crate::state::StateManager;

pub struct NetStateDbus {
    sm: Arc<StateManager>,
}

impl NetStateDbus {
    pub fn new(sm: Arc<StateManager>) -> Self {
        Self { sm }
    }
}

#[dbus_interface(name = "org.opdbus.StatePlugin")]
impl NetStateDbus {
    /// Apply desired state from a JSON file path (string)
    async fn ApplyState(&self, path: &str) -> String {
        let p = std::path::Path::new(path);
        // Minimal: load desired and apply only the net plugin
        let res = async {
            let desired = self.sm.load_desired_state(p).await?;
            self.sm.apply_state_single_plugin(desired, "net").await?;
            anyhow::Ok(())
        }
        .await;
        match res {
            Ok(_) => "ok".to_string(),
            Err(e) => format!("error: {}", e),
        }
    }
}

static DBUS_CONN: OnceCell<Connection> = OnceCell::new();

/// Start the org.opdbus service on the system bus, exposing /org/opdbus/state/net
pub async fn start_system_bus(sm: Arc<StateManager>) -> anyhow::Result<()> {
    let iface = NetStateDbus::new(sm);
    let conn = ConnectionBuilder::system()?
        .name("org.opdbus")?
        .serve_at("/org/opdbus/state/net", iface)?
        .build()
        .await?;
    // Hold the connection for the life of the process so the name stays activatable
    let _ = DBUS_CONN.set(conn);
    Ok(())
}
