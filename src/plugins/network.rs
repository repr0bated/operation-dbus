// src/plugins/network.rs - Network plugin with OVS/OVSDB persistence
//
// This plugin manages network configuration including OVS bridges via OVSDB.
// CRITICAL: Uses OVSDB JSON-RPC to ensure bridges persist in database.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

// Import OVSDB client
use crate::native::ovsdb_jsonrpc::OvsdbClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPlugin {
    /// OVS bridges to create
    #[serde(default)]
    pub bridges: Vec<OvsBridge>,

    /// Network interfaces to configure
    #[serde(default)]
    pub interfaces: Vec<NetworkInterface>,

    /// OVSDB persistence configuration
    #[serde(default)]
    pub ovsdb: OvsdbConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvsBridge {
    /// Bridge name (e.g., "vmbr0", "ovsbr0")
    pub name: String,

    /// Datapath type: "system" (default, kernel-based, persistent) or "netdev" (userspace)
    #[serde(default = "default_datapath_type")]
    pub datapath_type: String,

    /// Physical ports to add to bridge
    #[serde(default)]
    pub ports: Vec<String>,

    /// Internal ports (for IP assignment)
    #[serde(default)]
    pub internal_ports: Vec<String>,

    /// IP address for bridge interface (e.g., "10.0.1.1/24")
    pub address: Option<String>,

    /// Enable DHCP on this bridge
    #[serde(default)]
    pub dhcp: bool,

    /// VLAN ID (if this is a VLAN interface)
    pub vlan: Option<u16>,

    /// OpenFlow configuration
    pub openflow: Option<OpenFlowConfig>,
}

fn default_datapath_type() -> String {
    "system".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenFlowConfig {
    /// Automatically apply default rules on bridge creation
    #[serde(default)]
    pub auto_apply_defaults: bool,

    /// Default OpenFlow rules to apply
    #[serde(default)]
    pub default_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0", "ens1")
    pub name: String,

    /// IP address (e.g., "192.168.1.10/24")
    pub address: Option<String>,

    /// Enable DHCP
    #[serde(default)]
    pub dhcp: bool,

    /// Bring interface up
    #[serde(default = "default_true")]
    pub up: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OvsdbConfig {
    /// OVSDB socket path (default: /var/run/openvswitch/db.sock)
    #[serde(default = "default_ovsdb_socket")]
    pub socket_path: String,

    /// Database file path for persistence (default: /etc/openvswitch/conf.db)
    #[serde(default = "default_ovsdb_database")]
    pub database_path: String,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Ensure database persists across reboots
    #[serde(default = "default_true")]
    pub persist: bool,
}

fn default_ovsdb_socket() -> String {
    "/var/run/openvswitch/db.sock".to_string()
}

fn default_ovsdb_database() -> String {
    "/etc/openvswitch/conf.db".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl Default for OvsdbConfig {
    fn default() -> Self {
        Self {
            socket_path: default_ovsdb_socket(),
            database_path: default_ovsdb_database(),
            timeout_seconds: default_timeout(),
            persist: true,
        }
    }
}

impl NetworkPlugin {
    pub async fn apply(&self) -> Result<()> {
        info!("Network plugin: Starting network configuration");

        // Step 1: Verify OVSDB persistence configuration
        if !self.bridges.is_empty() {
            self.verify_ovsdb_persistence().await?;
        }

        // Step 2: Wait for OVSDB to be ready
        if !self.bridges.is_empty() {
            self.wait_for_ovsdb().await?;
        }

        // Step 3: Create OVS bridges (via OVSDB JSON-RPC for persistence)
        for bridge in &self.bridges {
            self.create_ovs_bridge(bridge).await?;
        }

        // Step 4: Configure network interfaces
        for interface in &self.interfaces {
            self.configure_interface(interface).await?;
        }

        info!("✓ Network plugin: Complete");
        Ok(())
    }

    async fn verify_ovsdb_persistence(&self) -> Result<()> {
        info!("Verifying OVSDB persistence configuration");

        // Check if database file directory exists
        let db_path = std::path::Path::new(&self.ovsdb.database_path);
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                warn!(
                    "OVSDB database directory does not exist: {}",
                    parent.display()
                );
                info!("Creating directory: {}", parent.display());
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        // Verify persistence is enabled
        if !self.ovsdb.persist {
            warn!("OVSDB persistence is DISABLED - bridges may not survive reboots!");
            warn!("Set ovsdb.persist=true in state.json to enable persistence");
        } else {
            info!("✓ OVSDB persistence enabled: {}", self.ovsdb.database_path);
        }

        // Check if ovsdb-server is configured to use the database file
        // This is typically done via systemd service configuration
        info!(
            "Ensure ovsdb-server.service is configured with: --db-file={}",
            self.ovsdb.database_path
        );

        Ok(())
    }

    async fn wait_for_ovsdb(&self) -> Result<()> {
        info!(
            "Waiting for OVSDB to be ready (timeout: {}s)",
            self.ovsdb.timeout_seconds
        );

        let client = OvsdbClient::new();
        let timeout = Duration::from_secs(self.ovsdb.timeout_seconds);
        let start = std::time::Instant::now();

        loop {
            match client.list_dbs().await {
                Ok(dbs) => {
                    info!("✓ OVSDB is ready, available databases: {:?}", dbs);
                    return Ok(());
                }
                Err(e) => {
                    if start.elapsed() > timeout {
                        return Err(anyhow::anyhow!(
                            "OVSDB connection timeout after {}s: {}",
                            self.ovsdb.timeout_seconds,
                            e
                        ));
                    }
                    warn!("OVSDB not ready yet, retrying... ({})", e);
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn create_ovs_bridge(&self, bridge: &OvsBridge) -> Result<()> {
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("Creating OVS bridge: {}", bridge.name);
        info!("  Datapath type: {}", bridge.datapath_type);
        info!("  Ports: {:?}", bridge.ports);
        info!("  Internal ports: {:?}", bridge.internal_ports);
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let client = OvsdbClient::new();

        // Check if bridge already exists
        let exists = client.bridge_exists(&bridge.name).await?;

        if exists {
            info!("  Bridge '{}' already exists (idempotent)", bridge.name);
            // Verify ports are correct
            let existing_ports = client.list_bridge_ports(&bridge.name).await?;
            info!("  Existing ports: {:?}", existing_ports);
        } else {
            // Create bridge via OVSDB JSON-RPC
            // CRITICAL: This writes to OVSDB database, ensuring persistence
            info!("  Creating bridge via OVSDB JSON-RPC (persistent)");
            client.create_bridge(&bridge.name).await
                .context(format!("Failed to create bridge '{}'", bridge.name))?;
            info!("  ✓ Bridge '{}' created in OVSDB database", bridge.name);
        }

        // Add physical ports to bridge
        for port in &bridge.ports {
            info!("  Adding physical port: {}", port);
            // Check if port already exists
            let existing_ports = client.list_bridge_ports(&bridge.name).await?;
            if existing_ports.contains(port) {
                info!("    Port '{}' already exists (idempotent)", port);
            } else {
                client.add_port(&bridge.name, port).await
                    .context(format!("Failed to add port '{}' to bridge '{}'", port, bridge.name))?;
                info!("    ✓ Port '{}' added", port);
            }
        }

        // Add internal ports to bridge
        for internal_port in &bridge.internal_ports {
            info!("  Adding internal port: {}", internal_port);
            let existing_ports = client.list_bridge_ports(&bridge.name).await?;
            if existing_ports.contains(internal_port) {
                info!("    Internal port '{}' already exists (idempotent)", internal_port);
            } else {
                // For internal ports, we need to create them with type=internal
                // This is done via OVSDB, similar to: ovs-vsctl add-port br0 port -- set interface port type=internal
                client.add_port(&bridge.name, internal_port).await
                    .context(format!("Failed to add internal port '{}' to bridge '{}'", internal_port, bridge.name))?;
                info!("    ✓ Internal port '{}' added", internal_port);
            }
        }

        // Bring up bridge interface
        info!("  Bringing up bridge interface");
        self.bring_up_interface(&bridge.name).await?;

        // Configure IP address if specified
        if let Some(ref address) = bridge.address {
            info!("  Configuring IP address: {}", address);
            self.configure_ip(&bridge.name, address).await?;
        }

        // Enable DHCP if requested
        if bridge.dhcp {
            info!("  Enabling DHCP");
            self.enable_dhcp(&bridge.name).await?;
        }

        // Apply OpenFlow rules if configured
        if let Some(ref openflow) = bridge.openflow {
            if openflow.auto_apply_defaults && !openflow.default_rules.is_empty() {
                info!("  Applying OpenFlow default rules");
                self.apply_openflow_rules(&bridge.name, &openflow.default_rules).await?;
            }
        }

        info!("✓ Bridge '{}' configured successfully", bridge.name);
        Ok(())
    }

    async fn configure_interface(&self, interface: &NetworkInterface) -> Result<()> {
        info!("Configuring interface: {}", interface.name);

        // Bring interface up/down
        if interface.up {
            self.bring_up_interface(&interface.name).await?;
        } else {
            self.bring_down_interface(&interface.name).await?;
        }

        // Configure IP address
        if let Some(ref address) = interface.address {
            self.configure_ip(&interface.name, address).await?;
        }

        // Enable DHCP
        if interface.dhcp {
            self.enable_dhcp(&interface.name).await?;
        }

        info!("✓ Interface '{}' configured", interface.name);
        Ok(())
    }

    async fn bring_up_interface(&self, name: &str) -> Result<()> {
        let output = tokio::process::Command::new("ip")
            .arg("link")
            .arg("set")
            .arg(name)
            .arg("up")
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to bring up interface {}: {}", name, stderr);
        } else {
            info!("    ✓ Interface '{}' is up", name);
        }

        Ok(())
    }

    async fn bring_down_interface(&self, name: &str) -> Result<()> {
        let output = tokio::process::Command::new("ip")
            .arg("link")
            .arg("set")
            .arg(name)
            .arg("down")
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to bring down interface {}: {}", name, stderr));
        }

        Ok(())
    }

    async fn configure_ip(&self, interface: &str, address: &str) -> Result<()> {
        // Remove existing IP addresses first
        let _ = tokio::process::Command::new("ip")
            .arg("addr")
            .arg("flush")
            .arg("dev")
            .arg(interface)
            .output()
            .await;

        // Add new IP address
        let output = tokio::process::Command::new("ip")
            .arg("addr")
            .arg("add")
            .arg(address)
            .arg("dev")
            .arg(interface)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "RTNETLINK answers: File exists" error (address already configured)
            if !stderr.contains("File exists") {
                return Err(anyhow::anyhow!("Failed to configure IP {}: {}", address, stderr));
            }
        }

        info!("    ✓ IP address {} configured on {}", address, interface);
        Ok(())
    }

    async fn enable_dhcp(&self, interface: &str) -> Result<()> {
        // Use dhclient to request DHCP lease
        let output = tokio::process::Command::new("dhclient")
            .arg("-v")
            .arg(interface)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("DHCP client warning for {}: {}", interface, stderr);
        } else {
            info!("    ✓ DHCP enabled on {}", interface);
        }

        Ok(())
    }

    async fn apply_openflow_rules(&self, bridge: &str, rules: &[String]) -> Result<()> {
        info!("    Applying {} OpenFlow rules to {}", rules.len(), bridge);

        // Create OpenFlow client and connect to switch
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 6633));
        let mut client = crate::native::openflow::OpenFlowClient::connect(addr).await
            .context(format!("Failed to connect to OpenFlow switch for bridge {}", bridge))?;

        // Clear existing flows first
        client.delete_all_flows().await?;

        // Apply each rule (note: currently using placeholder implementation)
        for rule in rules {
            client.add_flow_rule(rule).await?;
            warn!("Applied OpenFlow rule (placeholder): {}", rule);
        }

        info!("    ✓ OpenFlow rules applied to {}", bridge);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_network_config() {
        let json = r#"
        {
            "bridges": [
                {
                    "name": "vmbr0",
                    "datapath_type": "system",
                    "ports": ["eth0"],
                    "internal_ports": ["vmbr0-if"],
                    "address": "10.0.0.1/24"
                }
            ],
            "ovsdb": {
                "database_path": "/etc/openvswitch/conf.db",
                "persist": true
            }
        }
        "#;

        let plugin: NetworkPlugin = serde_json::from_str(json).unwrap();
        assert_eq!(plugin.bridges.len(), 1);
        assert_eq!(plugin.bridges[0].name, "vmbr0");
        assert_eq!(plugin.bridges[0].datapath_type, "system");
        assert_eq!(plugin.ovsdb.database_path, "/etc/openvswitch/conf.db");
        assert!(plugin.ovsdb.persist);
    }

    #[test]
    fn test_default_ovsdb_config() {
        let config = OvsdbConfig::default();
        assert_eq!(config.socket_path, "/var/run/openvswitch/db.sock");
        assert_eq!(config.database_path, "/etc/openvswitch/conf.db");
        assert_eq!(config.timeout_seconds, 30);
        assert!(config.persist);
    }
}
