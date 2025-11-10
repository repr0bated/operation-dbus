//! PackageKit Plugin for op-dbus
//!
//! Manages system packages via PackageKit D-Bus interface
//! Provides declarative package installation/removal
//!
//! Example state:
//! ```json
//! {
//!   "version": 1,
//!   "plugins": {
//!     "packagekit": {
//!       "packages": {
//!         "proxmox-ve": {
//!           "ensure": "installed",
//!           "provider": "apt"
//!         },
//!         "postfix": {
//!           "ensure": "installed",
//!           "provider": "apt"
//!         }
//!       }
//!     }
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;
use tokio::time::{sleep, Duration};
use zbus::{Connection, Message, proxy};

use crate::state::plugin::{
    ApplyResult, Checkpoint, PluginCapabilities, StateAction, StateDiff, StatePlugin, DiffMetadata
};

// PackageKit D-Bus interface
#[proxy(
    interface = "org.freedesktop.PackageKit",
    default_service = "org.freedesktop.PackageKit",
    default_path = "/org/freedesktop/PackageKit"
)]
trait PackageKit {
    /// Get transaction list
    async fn get_transaction_list(&self) -> zbus::Result<Vec<String>>;

    /// Create transaction
    async fn create_transaction(&self) -> zbus::Result<String>;
}

// Transaction interface
#[proxy(
    interface = "org.freedesktop.PackageKit.Transaction",
    default_service = "org.freedesktop.PackageKit"
)]
trait PackageKitTransaction {
    /// Get properties
    async fn get_properties(&self) -> zbus::Result<HashMap<String, Value>>;

    /// Install packages
    async fn install_packages(&self, transaction_flags: u64, package_ids: Vec<String>) -> zbus::Result<()>;

    /// Remove packages
    async fn remove_packages(&self, transaction_flags: u64, package_ids: Vec<String>, allow_deps: bool, autoremove: bool) -> zbus::Result<()>;

    /// Resolve packages
    async fn resolve(&self, filters: u64, packages: Vec<String>) -> zbus::Result<()>;

    /// Get package list
    async fn get_packages(&self, filters: u64) -> zbus::Result<()>;

    /// Search packages
    async fn search_names(&self, filters: u64, values: Vec<String>) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageState {
    pub ensure: String, // "installed", "removed", "latest"
    pub provider: Option<String>, // "apt", "dnf", "pacman", etc.
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageKitState {
    pub version: u32,
    pub packages: HashMap<String, PackageState>,
}

#[derive(Debug, Clone)]
pub struct PackageKitPlugin {
    connection: Option<Connection>,
}

impl PackageKitPlugin {
    pub fn new() -> Self {
        Self {
            connection: None,
        }
    }

    async fn get_connection(&mut self) -> Result<&Connection> {
        if self.connection.is_none() {
            self.connection = Some(Connection::system().await?);
        }
        Ok(self.connection.as_ref().unwrap())
    }

    /// Check if PackageKit is available
    async fn packagekit_available(&mut self) -> bool {
        let conn = match self.get_connection().await {
            Ok(c) => c,
            Err(_) => return false,
        };

        // Try to get PackageKit service
        match conn.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &"org.freedesktop.PackageKit",
        ).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Resolve package name to PackageKit package ID
    async fn resolve_package(&mut self, package_name: &str) -> Result<Option<String>> {
        if !self.packagekit_available().await {
            // Fallback to direct package manager
            return self.resolve_via_direct(package_name).await;
        }

        let conn = self.get_connection().await?;
        let pk = PackageKitProxy::new(conn).await?;

        // Create transaction
        let transaction_path = pk.create_transaction().await?;

        // Get transaction proxy
        let transaction = PackageKitTransactionProxy::builder(conn)
            .path(transaction_path)?
            .build()
            .await?;

        // Resolve package
        transaction.resolve(0, vec![package_name.to_string()]).await?;

        // Wait for resolution (simplified - in real implementation would listen to signals)
        sleep(Duration::from_millis(500)).await;

        // For now, return a mock package ID
        // In real implementation, would parse Package signal results
        Ok(Some(format!("{};;installed;local", package_name)))
    }

    /// Fallback resolution via direct package manager
    async fn resolve_via_direct(&self, package_name: &str) -> Result<Option<String>> {
        // Try apt (Debian/Ubuntu)
        if Command::new("apt").arg("show").arg(package_name).output().is_ok() {
            return Ok(Some(format!("{};;installed;apt", package_name)));
        }

        // Try dnf (Fedora/RHEL)
        if Command::new("dnf").arg("info").arg(package_name).output().is_ok() {
            return Ok(Some(format!("{};;installed;dnf", package_name)));
        }

        // Try pacman (Arch)
        if Command::new("pacman").arg("-Qi").arg(package_name).output().is_ok() {
            return Ok(Some(format!("{};;installed;pacman", package_name)));
        }

        Ok(None)
    }

    /// Install package via PackageKit
    async fn install_package(&mut self, package_id: &str) -> Result<()> {
        if !self.packagekit_available().await {
            return self.install_via_direct(package_id).await;
        }

        let conn = self.get_connection().await?;
        let pk = PackageKitProxy::new(conn).await?;

        // Create transaction
        let transaction_path = pk.create_transaction().await?;

        let transaction = PackageKitTransactionProxy::builder(conn)
            .path(transaction_path)?
            .build()
            .await?;

        // Install package
        transaction.install_packages(0, vec![package_id.to_string()]).await?;

        // Wait for completion (simplified)
        sleep(Duration::from_secs(10)).await;

        Ok(())
    }

    /// Install via direct package manager
    async fn install_via_direct(&self, package_id: &str) -> Result<()> {
        let package_name = package_id.split(";;").next().unwrap_or(package_id);

        // Try apt
        if Command::new("apt-get")
            .args(["install", "-y", package_name])
            .status()?
            .success()
        {
            return Ok(());
        }

        // Try dnf
        if Command::new("dnf")
            .args(["install", "-y", package_name])
            .status()?
            .success()
        {
            return Ok(());
        }

        // Try pacman
        if Command::new("pacman")
            .args(["-S", "--noconfirm", package_name])
            .status()?
            .success()
        {
            return Ok(());
        }

        Err(anyhow::anyhow!("No suitable package manager found"))
    }

    /// Remove package via PackageKit
    async fn remove_package(&mut self, package_id: &str) -> Result<()> {
        if !self.packagekit_available().await {
            return self.remove_via_direct(package_id).await;
        }

        let conn = self.get_connection().await?;
        let pk = PackageKitProxy::new(conn).await?;

        let transaction_path = pk.create_transaction().await?;
        let transaction = PackageKitTransactionProxy::builder(conn)
            .path(transaction_path)?
            .build()
            .await?;

        transaction.remove_packages(0, vec![package_id.to_string()], true, true).await?;
        sleep(Duration::from_secs(10)).await;

        Ok(())
    }

    /// Remove via direct package manager
    async fn remove_via_direct(&self, package_id: &str) -> Result<()> {
        let package_name = package_id.split(";;").next().unwrap_or(package_id);

        // Try apt
        if Command::new("apt-get")
            .args(["remove", "-y", package_name])
            .status()?
            .success()
        {
            return Ok(());
        }

        // Try dnf
        if Command::new("dnf")
            .args(["remove", "-y", package_name])
            .status()?
            .success()
        {
            return Ok(());
        }

        // Try pacman
        if Command::new("pacman")
            .args(["-R", "--noconfirm", package_name])
            .status()?
            .success()
        {
            return Ok(());
        }

        Err(anyhow::anyhow!("No suitable package manager found"))
    }

    /// Check if package is installed
    async fn package_installed(&self, package_name: &str) -> Result<bool> {
        // Try dpkg (Debian/Ubuntu)
        if Command::new("dpkg")
            .args(["-l", package_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(true);
        }

        // Try rpm (Fedora/RHEL)
        if Command::new("rpm")
            .args(["-q", package_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(true);
        }

        // Try pacman (Arch)
        if Command::new("pacman")
            .args(["-Q", package_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(true);
        }

        Ok(false)
    }
}

#[async_trait]
impl StatePlugin for PackageKitPlugin {
    fn name(&self) -> &str {
        "packagekit"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    async fn query_current_state(&self) -> Result<Value> {
        // For now, return empty state
        // In real implementation, would query all installed packages
        Ok(serde_json::json!({
            "version": 1,
            "packages": {}
        }))
    }

    async fn calculate_diff(&self, _current: &Value, desired: &Value) -> Result<StateDiff> {
        let desired_state: PackageKitState = serde_json::from_value(desired.clone())?;
        let mut actions = Vec::new();

        for (package_name, package_config) in &desired_state.packages {
            let is_installed = self.package_installed(package_name).await?;

            match package_config.ensure.as_str() {
                "installed" if !is_installed => {
                    actions.push(StateAction::Create {
                        resource: package_name.clone(),
                        config: serde_json::json!({
                            "ensure": "installed",
                            "provider": package_config.provider,
                            "version": package_config.version
                        }),
                    });
                }
                "removed" if is_installed => {
                    actions.push(StateAction::Delete {
                        resource: package_name.clone(),
                    });
                }
                _ => {
                    actions.push(StateAction::NoOp {
                        resource: package_name.clone(),
                    });
                }
            }
        }

        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash: "placeholder".to_string(),
                desired_hash: "placeholder".to_string(),
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    match self.install_package(resource).await {
                        Ok(()) => {
                            changes_applied.push(format!("✅ Installed package: {}", resource));
                        }
                        Err(e) => {
                            errors.push(format!("❌ Failed to install {}: {}", resource, e));
                        }
                    }
                }
                StateAction::Delete { resource } => {
                    match self.remove_package(resource).await {
                        Ok(()) => {
                            changes_applied.push(format!("✅ Removed package: {}", resource));
                        }
                        Err(e) => {
                            errors.push(format!("❌ Failed to remove {}: {}", resource, e));
                        }
                    }
                }
                StateAction::Modify { resource, changes } => {
                    // Handle version updates, etc.
                    changes_applied.push(format!("⚠️  Package {} modification not implemented", resource));
                }
                StateAction::NoOp { resource } => {
                    changes_applied.push(format!("📦 Package {}: no action required", resource));
                }
            }
        }

        Ok(ApplyResult {
            success: errors.is_empty(),
            changes_applied,
            errors,
            checkpoint: None,
        })
    }

    async fn verify_state(&self, desired: &Value) -> Result<bool> {
        let desired_state: PackageKitState = serde_json::from_value(desired.clone())?;

        for (package_name, package_config) in &desired_state.packages {
            let is_installed = self.package_installed(package_name).await?;

            match package_config.ensure.as_str() {
                "installed" => {
                    if !is_installed {
                        return Ok(false);
                    }
                }
                "removed" => {
                    if is_installed {
                        return Ok(false);
                    }
                }
                _ => {}
            }
        }

        Ok(true)
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        Ok(Checkpoint {
            id: format!("{}-{}", self.name(), chrono::Utc::now().timestamp()),
            plugin: self.name().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: serde_json::json!({}),
            backend_checkpoint: None,
        })
    }

    async fn rollback(&self, _checkpoint: &Checkpoint) -> Result<()> {
        // Package rollback is complex, not implemented yet
        Ok(())
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: false,
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: false,
        }
    }
}