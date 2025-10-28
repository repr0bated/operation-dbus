#![allow(async_fn_in_trait)]
//! Base trait for D-Bus state plugins
//! Provides common D-Bus operations, hash footprints, and blockchain integration

use crate::blockchain::PluginFootprint;
use crate::state::plugin::StatePlugin;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use zbus::{Connection, Proxy};

/// Base trait for all D-Bus-based state plugins
/// Provides common functionality for interacting with D-Bus services
#[allow(dead_code, async_fn_in_trait)]
pub trait DbusStatePluginBase: StatePlugin {
    /// D-Bus service name (e.g., "org.freedesktop.systemd1")
    fn service_name(&self) -> &str;

    /// Base object path (e.g., "/org/freedesktop/systemd1")
    fn base_path(&self) -> &str;

    /// Optional blockchain footprint sender
    fn blockchain_sender(&self) -> Option<&UnboundedSender<PluginFootprint>> {
        None
    }

    /// Connect to D-Bus service and create proxy
    async fn connect_dbus(&self, path: &str, interface: &str) -> Result<Proxy<'static>> {
        let conn = Connection::system()
            .await
            .context("Failed to connect to system D-Bus")?;

        // Convert to owned strings to satisfy 'static lifetime
        let service_name = self.service_name().to_string();
        let path_owned = path.to_string();
        let interface_owned = interface.to_string();

        Proxy::new(&conn, service_name, path_owned, interface_owned)
            .await
            .context(format!(
                "Failed to create D-Bus proxy for {}/{}",
                self.service_name(),
                interface
            ))
    }

    /// Get a D-Bus property value
    async fn get_property(&self, proxy: &Proxy<'_>, property: &str) -> Result<Value> {
        let value: zbus::zvariant::OwnedValue = proxy
            .get_property(property)
            .await
            .context(format!("Failed to get property {}", property))?;

        // Convert zbus::zvariant::Value to serde_json::Value
        let json_str = format!("{:?}", value); // Simplified - would need proper conversion
        Ok(serde_json::from_str(&json_str).unwrap_or(Value::Null))
    }

    /// Set a D-Bus property value
    async fn set_property(&self, proxy: &Proxy<'_>, property: &str, value: &Value) -> Result<()> {
        // Convert serde_json::Value to zbus::zvariant::Value (simplified)
        let zbus_value: zbus::zvariant::Value = match value {
            Value::String(s) => s.as_str().into(),
            Value::Bool(b) => (*b).into(),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i.into()
                } else if let Some(f) = n.as_f64() {
                    f.into()
                } else {
                    return Err(anyhow::anyhow!("Unsupported number type"));
                }
            }
            _ => return Err(anyhow::anyhow!("Unsupported value type for D-Bus property")),
        };

        proxy
            .set_property(property, zbus_value)
            .await
            .context(format!("Failed to set property {}", property))?;

        Ok(())
    }

    /// Get all properties from a D-Bus interface
    async fn get_all_properties(&self, proxy: &Proxy<'_>) -> Result<HashMap<String, Value>> {
        // Use org.freedesktop.DBus.Properties.GetAll
        let props_proxy = Proxy::new(
            proxy.connection(),
            proxy.destination(),
            proxy.path(),
            "org.freedesktop.DBus.Properties",
        )
        .await?;

        let all_props: HashMap<String, zbus::zvariant::OwnedValue> = props_proxy
            .call("GetAll", &(proxy.interface(),))
            .await
            .context("Failed to get all properties")?;

        // Convert to serde_json::Value HashMap
        let mut result = HashMap::new();
        for (key, _value) in all_props {
            // Simplified conversion - would need proper zvariant to serde_json conversion
            result.insert(key, Value::Null);
        }

        Ok(result)
    }

    /// Call a D-Bus method (no-arg version - for methods with args, use proxy.call directly)
    async fn call_method_no_args(
        &self,
        proxy: &Proxy<'_>,
        method: &str,
    ) -> Result<zbus::zvariant::OwnedValue> {
        proxy
            .call(method, &())
            .await
            .context(format!("Failed to call method {}", method))
    }

    /// Introspect D-Bus object to get schema
    async fn introspect(&self, path: &str) -> Result<String> {
        let conn = Connection::system().await?;
        let proxy = Proxy::new(
            &conn,
            self.service_name(),
            path,
            "org.freedesktop.DBus.Introspectable",
        )
        .await?;

        let xml: String = proxy
            .call("Introspect", &())
            .await
            .context("Failed to introspect D-Bus object")?;

        Ok(xml)
    }

    /// Calculate cryptographic hash of state (SHA-256)
    fn hash_state(&self, state: &Value) -> String {
        use sha2::{Digest, Sha256};
        let json_str = serde_json::to_string(state).unwrap_or_default();
        format!("{:x}", Sha256::digest(json_str.as_bytes()))
    }

    /// Calculate diff between two states and return hash footprint
    fn calculate_footprint(
        &self,
        old_state: &Value,
        new_state: &Value,
        action: &str,
    ) -> PluginFootprint {
        let diff_data = serde_json::json!({
            "old": old_state,
            "new": new_state,
            "old_hash": self.hash_state(old_state),
            "new_hash": self.hash_state(new_state),
        });

        PluginFootprint::new(self.name().to_string(), action.to_string(), diff_data)
    }

    /// Record state change to blockchain
    async fn record_footprint(&self, action: &str, data: Value) -> Result<()> {
        if let Some(sender) = self.blockchain_sender() {
            let footprint = PluginFootprint::new(self.name().to_string(), action.to_string(), data);

            sender
                .send(footprint)
                .map_err(|e| anyhow::anyhow!("Failed to send footprint to blockchain: {}", e))?;

            log::debug!("Recorded footprint for {} action: {}", self.name(), action);
        } else {
            log::trace!("No blockchain sender configured, skipping footprint");
        }

        Ok(())
    }

    /// Record a state transition (before/after)
    async fn record_state_transition(
        &self,
        old_state: &Value,
        new_state: &Value,
        action: &str,
    ) -> Result<()> {
        let footprint_data = serde_json::json!({
            "old_state": old_state,
            "new_state": new_state,
            "old_hash": self.hash_state(old_state),
            "new_hash": self.hash_state(new_state),
            "action": action,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        self.record_footprint(action, footprint_data).await
    }

    /// Verify state hash matches expected
    fn verify_state_hash(&self, state: &Value, expected_hash: &str) -> bool {
        self.hash_state(state) == expected_hash
    }

    /// Get D-Bus connection
    async fn get_connection(&self) -> Result<Connection> {
        Connection::system()
            .await
            .context("Failed to connect to system D-Bus")
    }

    /// List all object paths under a base path (for enumeration)
    async fn list_objects(&self, base_path: &str) -> Result<Vec<String>> {
        // This would use D-Bus introspection to walk the object tree
        // Simplified implementation
        let xml = self.introspect(base_path).await?;

        // Parse XML and extract child nodes
        // For now, return empty - full implementation would parse introspection XML
        log::debug!("Introspection XML for {}: {}", base_path, xml);

        Ok(Vec::new())
    }
}

/// Helper functions for D-Bus value conversion
pub mod conversion {
    use serde_json::Value;
    use zbus::zvariant;

    /// Convert serde_json::Value to zbus::zvariant::Value
    #[allow(dead_code)]
    pub fn json_to_zvariant(value: &Value) -> Result<zvariant::Value<'_>, anyhow::Error> {
        match value {
            Value::Null => Ok(zvariant::Value::from("")),
            Value::Bool(b) => Ok(zvariant::Value::from(*b)),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(zvariant::Value::from(i))
                } else if let Some(u) = n.as_u64() {
                    Ok(zvariant::Value::from(u))
                } else if let Some(f) = n.as_f64() {
                    Ok(zvariant::Value::from(f))
                } else {
                    Err(anyhow::anyhow!("Unsupported number format"))
                }
            }
            Value::String(s) => Ok(zvariant::Value::from(s.as_str())),
            Value::Array(_) => Err(anyhow::anyhow!("Array conversion not yet implemented")),
            Value::Object(_) => Err(anyhow::anyhow!("Object conversion not yet implemented")),
        }
    }

    /// Convert zbus::zvariant::Value to serde_json::Value
    #[allow(dead_code)]
    pub fn zvariant_to_json(value: &zvariant::Value) -> Result<Value, anyhow::Error> {
        // Simplified - full implementation would handle all zvariant types
        match value.value_signature().as_str() {
            "s" => {
                if let Ok(s) = <&str>::try_from(value) {
                    Ok(Value::String(s.to_string()))
                } else {
                    Ok(Value::Null)
                }
            }
            "b" => {
                if let Ok(b) = bool::try_from(value) {
                    Ok(Value::Bool(b))
                } else {
                    Ok(Value::Null)
                }
            }
            "i" | "u" | "x" | "t" => {
                // Try various integer types
                if let Ok(i) = i64::try_from(value) {
                    Ok(Value::Number(i.into()))
                } else if let Ok(u) = u64::try_from(value) {
                    Ok(Value::Number(u.into()))
                } else {
                    Ok(Value::Null)
                }
            }
            "d" => {
                if let Ok(f) = f64::try_from(value) {
                    Ok(serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => {
                // For complex types, use debug representation
                Ok(Value::String(format!("{:?}", value)))
            }
        }
    }
}
