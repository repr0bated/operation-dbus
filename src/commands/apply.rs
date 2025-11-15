// src/commands/apply.rs - Apply declarative state
//
// This is the core of operation-dbus: read state.json and orchestrate
// all plugins to achieve the desired state.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn, error};

/// Complete system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    #[serde(default)]
    pub metadata: Metadata,
    #[serde(default)]
    pub bootstrap: Bootstrap,
    pub plugins: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bootstrap {
    pub debian_version: Option<String>,
    pub hostname: Option<String>,
    pub domain: Option<String>,
    pub timezone: Option<String>,
}

/// Apply state from file
pub async fn apply_state(state_file: PathBuf) -> Result<()> {
    info!("Applying state from: {}", state_file.display());

    // Load state
    let state_json = std::fs::read_to_string(&state_file)
        .context("Failed to read state file")?;

    let state: State = serde_json::from_str(&state_json)
        .context("Failed to parse state file")?;

    info!("State: {}", state.metadata.name.as_deref().unwrap_or("unnamed"));
    info!("Plugins to apply: {}", state.plugins.len());

    // Apply plugins in dependency order
    let plugin_order = determine_plugin_order(&state);

    for plugin_name in plugin_order {
        if let Some(plugin_config) = state.plugins.get(&plugin_name) {
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            info!("Applying plugin: {}", plugin_name);
            info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            match apply_plugin(&plugin_name, plugin_config).await {
                Ok(()) => {
                    info!("✓ Plugin '{}' applied successfully", plugin_name);
                }
                Err(e) => {
                    error!("✗ Plugin '{}' failed: {}", plugin_name, e);
                    return Err(e);
                }
            }
        }
    }

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("✅ State applied successfully!");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Determine plugin application order based on dependencies
fn determine_plugin_order(state: &State) -> Vec<String> {
    // Plugin dependency order (hardcoded for now, could be dynamic later)
    let order = vec![
        "storage",      // Must come first (create filesystems)
        "network",      // Early (configure networking)
        "packagekit",   // Mid (install packages)
        "systemd",      // After packages (enable services)
        "users",        // After systemd (create accounts)
        "firewall",     // After network (configure rules)
        "lxc",          // Late (deploy containers)
        "kvm",          // Late (deploy VMs)
    ];

    // Filter to only plugins present in state
    order
        .into_iter()
        .filter(|name| state.plugins.contains_key(*name))
        .map(String::from)
        .collect()
}

/// Apply a single plugin
async fn apply_plugin(name: &str, config: &serde_json::Value) -> Result<()> {
    match name {
        "packagekit" => {
            #[cfg(feature = "packagekit")]
            {
                use crate::plugins::packagekit::PackageKitPlugin;
                let plugin: PackageKitPlugin = serde_json::from_value(config.clone())?;
                plugin.apply().await
            }
            #[cfg(not(feature = "packagekit"))]
            {
                warn!("PackageKit plugin not compiled in (missing feature flag)");
                Ok(())
            }
        }
        "network" => {
            use crate::plugins::network::NetworkPlugin;
            let plugin: NetworkPlugin = serde_json::from_value(config.clone())?;
            plugin.apply().await
        }
        "systemd" => {
            #[cfg(feature = "systemd")]
            {
                use crate::plugins::systemd::SystemdPlugin;
                let plugin: SystemdPlugin = serde_json::from_value(config.clone())?;
                plugin.apply().await
            }
            #[cfg(not(feature = "systemd"))]
            {
                warn!("Systemd plugin not compiled in");
                Ok(())
            }
        }
        "storage" => {
            #[cfg(feature = "storage")]
            {
                use crate::plugins::storage::StoragePlugin;
                let plugin: StoragePlugin = serde_json::from_value(config.clone())?;
                plugin.apply().await
            }
            #[cfg(not(feature = "storage"))]
            {
                warn!("Storage plugin not compiled in");
                Ok(())
            }
        }
        "users" => {
            #[cfg(feature = "users")]
            {
                use crate::plugins::users::UsersPlugin;
                let plugin: UsersPlugin = serde_json::from_value(config.clone())?;
                plugin.apply().await
            }
            #[cfg(not(feature = "users"))]
            {
                warn!("Users plugin not compiled in");
                Ok(())
            }
        }
        "firewall" => {
            #[cfg(feature = "firewall")]
            {
                use crate::plugins::firewall::FirewallPlugin;
                let plugin: FirewallPlugin = serde_json::from_value(config.clone())?;
                plugin.apply().await
            }
            #[cfg(not(feature = "firewall"))]
            {
                warn!("Firewall plugin not compiled in");
                Ok(())
            }
        }
        "lxc" => {
            use crate::plugins::lxc::LxcPlugin;
            let plugin: LxcPlugin = serde_json::from_value(config.clone())?;
            plugin.apply().await
        }
        _ => {
            warn!("Unknown plugin: {}", name);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_state() {
        let json = r#"
        {
            "version": 1,
            "metadata": {
                "name": "test"
            },
            "plugins": {
                "packagekit": {
                    "manifest": "/tmp/manifest.json"
                }
            }
        }
        "#;

        let state: State = serde_json::from_str(json).unwrap();
        assert_eq!(state.version, 1);
        assert_eq!(state.metadata.name, Some("test".to_string()));
        assert!(state.plugins.contains_key("packagekit"));
    }

    #[test]
    fn test_plugin_order() {
        let json = r#"
        {
            "version": 1,
            "plugins": {
                "lxc": {},
                "storage": {},
                "network": {},
                "packagekit": {}
            }
        }
        "#;

        let state: State = serde_json::from_str(json).unwrap();
        let order = determine_plugin_order(&state);

        // storage should come before packagekit, which should come before lxc
        let storage_idx = order.iter().position(|s| s == "storage").unwrap();
        let packagekit_idx = order.iter().position(|s| s == "packagekit").unwrap();
        let lxc_idx = order.iter().position(|s| s == "lxc").unwrap();

        assert!(storage_idx < packagekit_idx);
        assert!(packagekit_idx < lxc_idx);
    }
}
