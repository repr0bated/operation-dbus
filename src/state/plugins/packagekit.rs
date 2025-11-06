//! PackageKit state plugin - manages packages via org.freedesktop.PackageKit D-Bus
//! Maps package state to declarative configuration

use crate::state::plugin::{
    ApplyResult, Checkpoint, DiffMetadata, PluginCapabilities, StateAction, StateDiff, StatePlugin,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};
use zbus::{Connection, Proxy};

/// PackageKit configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageKitConfig {
    /// Packages that must be installed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed: Option<Vec<String>>,

    /// Packages that must be removed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<Vec<String>>,

    /// Packages with specific version requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_pinned: Option<HashMap<String, String>>,

    /// Whether to auto-update packages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
}

/// Represents a package in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub repo: String,
    pub installed: bool,
    pub package_id: String, // Full PackageKit ID: name;version;arch;repo
}

/// PackageKit state plugin
pub struct PackageKitPlugin;

impl PackageKitPlugin {
    pub fn new() -> Self {
        Self
    }

    /// Connect to PackageKit daemon
    async fn connect_packagekit(&self) -> Result<Proxy<'static>> {
        let conn = Connection::system()
            .await
            .context("Failed to connect to system D-Bus")?;

        Proxy::new(
            &conn,
            "org.freedesktop.PackageKit",
            "/org/freedesktop/PackageKit",
            "org.freedesktop.PackageKit",
        )
        .await
        .context("Failed to create PackageKit D-Bus proxy")
    }

    /// Create a new transaction
    async fn create_transaction(&self) -> Result<String> {
        let proxy = self.connect_packagekit().await?;

        let path: zbus::zvariant::OwnedObjectPath = proxy
            .call("CreateTransaction", &())
            .await
            .context("Failed to create PackageKit transaction")?;

        Ok(path.to_string())
    }

    /// Get transaction proxy
    async fn get_transaction_proxy(&self, transaction_path: &str) -> Result<Proxy<'static>> {
        let conn = Connection::system().await?;

        Proxy::new(
            &conn,
            "org.freedesktop.PackageKit",
            transaction_path,
            "org.freedesktop.PackageKit.Transaction",
        )
        .await
        .context("Failed to create transaction proxy")
    }

    /// Query all installed packages
    async fn query_installed_packages(&self) -> Result<Vec<Package>> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        // Filter: "installed" - only show installed packages
        let filter: u64 = 1 << 2; // FILTER_INSTALLED bit

        // Start GetPackages call (async)
        proxy
            .call::<_, ()>("GetPackages", &(filter,))
            .await
            .context("Failed to call GetPackages")?;

        // In a full implementation, we would:
        // 1. Listen for "Package" signals via zbus::MessageStream
        // 2. Collect all packages from signals
        // 3. Wait for "Finished" signal
        //
        // For now, return empty vec (will be implemented with signal handling)

        log::info!("GetPackages called on transaction {}", transaction_path);
        log::warn!("Package signal handling not yet implemented - returning empty list");

        Ok(Vec::new())
    }

    /// Resolve package names to package IDs
    async fn resolve_packages(&self, package_names: &[String]) -> Result<Vec<String>> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        let filter: u64 = 0; // No filter
        let packages: Vec<&str> = package_names.iter().map(|s| s.as_str()).collect();

        proxy
            .call::<_, ()>("Resolve", &(filter, packages))
            .await
            .context("Failed to resolve package names")?;

        // Would collect package IDs from signals
        log::info!("Resolve called for {} packages", package_names.len());

        Ok(Vec::new())
    }

    /// Install packages by package ID
    async fn install_packages(&self, package_ids: &[String]) -> Result<()> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        let transaction_flags: u64 = 0; // No special flags
        let ids: Vec<&str> = package_ids.iter().map(|s| s.as_str()).collect();

        proxy
            .call::<_, ()>("InstallPackages", &(transaction_flags, ids))
            .await
            .context("Failed to install packages")?;

        log::info!("InstallPackages called for {} packages", package_ids.len());

        Ok(())
    }

    /// Remove packages by package ID
    async fn remove_packages(&self, package_ids: &[String]) -> Result<()> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        let transaction_flags: u64 = 0;
        let allow_deps: bool = true;
        let autoremove: bool = true;
        let ids: Vec<&str> = package_ids.iter().map(|s| s.as_str()).collect();

        proxy
            .call::<_, ()>("RemovePackages", &(transaction_flags, ids, allow_deps, autoremove))
            .await
            .context("Failed to remove packages")?;

        log::info!("RemovePackages called for {} packages", package_ids.len());

        Ok(())
    }

    /// Search for packages by name
    async fn search_packages(&self, search_terms: &[String]) -> Result<Vec<Package>> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        let filter: u64 = 0; // No filter
        let terms: Vec<&str> = search_terms.iter().map(|s| s.as_str()).collect();

        proxy
            .call::<_, ()>("SearchNames", &(filter, terms))
            .await
            .context("Failed to search packages")?;

        log::info!("SearchNames called for: {:?}", search_terms);

        Ok(Vec::new())
    }

    /// Get package details
    async fn get_package_details(&self, package_ids: &[String]) -> Result<Vec<PackageDetails>> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        let ids: Vec<&str> = package_ids.iter().map(|s| s.as_str()).collect();

        proxy
            .call::<_, ()>("GetDetails", &(ids,))
            .await
            .context("Failed to get package details")?;

        log::info!("GetDetails called for {} packages", package_ids.len());

        Ok(Vec::new())
    }

    /// Refresh package cache
    async fn refresh_cache(&self) -> Result<()> {
        let transaction_path = self.create_transaction().await?;
        let proxy = self.get_transaction_proxy(&transaction_path).await?;

        let force: bool = true; // Force refresh

        proxy
            .call::<_, ()>("RefreshCache", &(force,))
            .await
            .context("Failed to refresh package cache")?;

        log::info!("RefreshCache called");

        Ok(())
    }
}

/// Package details from PackageKit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDetails {
    pub package_id: String,
    pub license: String,
    pub group: String,
    pub description: String,
    pub url: String,
    pub size: u64,
}

#[async_trait]
impl StatePlugin for PackageKitPlugin {
    fn name(&self) -> &str {
        "packagekit"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn query_current_state(&self) -> Result<Value> {
        log::info!("Querying PackageKit state...");

        // Query installed packages
        let installed = self.query_installed_packages().await?;

        let config = PackageKitConfig {
            installed: Some(installed.iter().map(|p| p.name.clone()).collect()),
            removed: None,
            version_pinned: Some(
                installed
                    .iter()
                    .map(|p| (p.name.clone(), p.version.clone()))
                    .collect(),
            ),
            auto_update: None,
        };

        serde_json::to_value(&config).context("Failed to serialize PackageKit state")
    }

    async fn calculate_diff(&self, current: &Value, desired: &Value) -> Result<StateDiff> {
        let current_config: PackageKitConfig =
            serde_json::from_value(current.clone()).unwrap_or(PackageKitConfig {
                installed: None,
                removed: None,
                version_pinned: None,
                auto_update: None,
            });

        let desired_config: PackageKitConfig = serde_json::from_value(desired.clone())
            .context("Invalid PackageKit configuration")?;

        let mut actions = Vec::new();

        // Collect currently installed package names
        let current_installed: Vec<String> = current_config.installed.unwrap_or_default();

        // Check for packages to install
        if let Some(desired_installed) = &desired_config.installed {
            for package in desired_installed {
                if !current_installed.contains(package) {
                    actions.push(StateAction::Create {
                        resource: format!("package:{}", package),
                        config: serde_json::json!({
                            "name": package,
                            "action": "install"
                        }),
                    });
                }
            }
        }

        // Check for packages to remove
        if let Some(desired_removed) = &desired_config.removed {
            for package in desired_removed {
                if current_installed.contains(package) {
                    actions.push(StateAction::Delete {
                        resource: format!("package:{}", package),
                    });
                }
            }
        }

        // Calculate hashes
        use sha2::{Digest, Sha256};
        let current_hash = format!("{:x}", Sha256::digest(current.to_string().as_bytes()));
        let desired_hash = format!("{:x}", Sha256::digest(desired.to_string().as_bytes()));

        Ok(StateDiff {
            plugin: self.name().to_string(),
            actions,
            metadata: DiffMetadata {
                timestamp: chrono::Utc::now().timestamp(),
                current_hash,
                desired_hash,
            },
        })
    }

    async fn apply_state(&self, diff: &StateDiff) -> Result<ApplyResult> {
        let mut changes_applied = Vec::new();
        let mut errors = Vec::new();

        for action in &diff.actions {
            match action {
                StateAction::Create { resource, config } => {
                    // Extract package name
                    if let Some(name) = config.get("name").and_then(|v| v.as_str()) {
                        match self.resolve_packages(&[name.to_string()]).await {
                            Ok(package_ids) => {
                                if !package_ids.is_empty() {
                                    match self.install_packages(&package_ids).await {
                                        Ok(_) => changes_applied.push(format!("Installed {}", name)),
                                        Err(e) => errors.push(format!("Failed to install {}: {}", name, e)),
                                    }
                                } else {
                                    errors.push(format!("Package {} not found", name));
                                }
                            }
                            Err(e) => errors.push(format!("Failed to resolve {}: {}", name, e)),
                        }
                    }
                }
                StateAction::Delete { resource } => {
                    // Extract package name from "package:name"
                    if let Some(name) = resource.strip_prefix("package:") {
                        match self.resolve_packages(&[name.to_string()]).await {
                            Ok(package_ids) => {
                                if !package_ids.is_empty() {
                                    match self.remove_packages(&package_ids).await {
                                        Ok(_) => changes_applied.push(format!("Removed {}", name)),
                                        Err(e) => errors.push(format!("Failed to remove {}: {}", name, e)),
                                    }
                                } else {
                                    errors.push(format!("Package {} not found", name));
                                }
                            }
                            Err(e) => errors.push(format!("Failed to resolve {}: {}", name, e)),
                        }
                    }
                }
                StateAction::Modify { resource, changes } => {
                    log::info!("Modify action for {}: {:?}", resource, changes);
                    // Could implement version updates here
                }
                StateAction::NoOp { resource } => {
                    log::debug!("No-op for {}", resource);
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
        let current = self.query_current_state().await?;
        let diff = self.calculate_diff(&current, desired).await?;

        Ok(diff.actions.is_empty())
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        let state = self.query_current_state().await?;

        Ok(Checkpoint {
            id: uuid::Uuid::new_v4().to_string(),
            plugin: self.name().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            state_snapshot: state,
            backend_checkpoint: None,
        })
    }

    async fn rollback(&self, checkpoint: &Checkpoint) -> Result<()> {
        let current = self.query_current_state().await?;
        let diff = self
            .calculate_diff(&current, &checkpoint.state_snapshot)
            .await?;

        let result = self.apply_state(&diff).await?;

        if result.success {
            Ok(())
        } else {
            Err(anyhow!(
                "Rollback failed: {}",
                result.errors.join(", ")
            ))
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            supports_rollback: true,
            supports_checkpoints: true,
            supports_verification: true,
            atomic_operations: false, // PackageKit operations may fail partially
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_package_config_serialization() {
        let config = PackageKitConfig {
            installed: Some(vec!["nginx".to_string(), "postgresql".to_string()]),
            removed: Some(vec!["apache2".to_string()]),
            version_pinned: None,
            auto_update: Some(false),
        };

        let json = serde_json::to_string_pretty(&config).unwrap();
        println!("{}", json);

        let deserialized: PackageKitConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.installed.as_ref().unwrap().len(), 2);
    }
}
