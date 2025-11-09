//! Web UI for op-dbus state management
//! Provides a REST API and web interface for managing system state

#[cfg(feature = "web")]
pub mod server;

#[cfg(feature = "web")]
#[allow(unused_imports)]  // Used by binary, not library
pub use server::{start_web_server, WebConfig};

#[cfg(not(feature = "web"))]
pub struct WebConfig;

#[cfg(not(feature = "web"))]
pub async fn start_web_server(
    _state_manager: std::sync::Arc<crate::state::StateManager>,
    _config: WebConfig,
) -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "Web feature not enabled. Rebuild with --features web"
    ))
}
