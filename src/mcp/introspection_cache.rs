//! D-Bus introspection cache with JSON storage
//!
//! Converts D-Bus XML introspection to JSON and caches in SQLite for fast access.
//! All components can query JSON directly without XML parsing overhead.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::Value as JsonValue;
use std::path::Path;
use zbus_xml::Node as XmlNode;

/// Introspection cache storing JSON representations
pub struct IntrospectionCache {
    conn: Connection,
}

impl IntrospectionCache {
    /// Create or open introspection cache database
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create schema if not exists
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS introspection_cache (
                service_name TEXT NOT NULL,
                object_path TEXT NOT NULL,
                interface_name TEXT NOT NULL,
                cached_at INTEGER NOT NULL,
                introspection_json TEXT NOT NULL,
                PRIMARY KEY (service_name, object_path, interface_name)
            );

            CREATE INDEX IF NOT EXISTS idx_service
                ON introspection_cache(service_name);

            CREATE INDEX IF NOT EXISTS idx_cached_at
                ON introspection_cache(cached_at);

            CREATE TABLE IF NOT EXISTS service_methods (
                service_name TEXT NOT NULL,
                interface_name TEXT NOT NULL,
                method_name TEXT NOT NULL,
                signature_json TEXT NOT NULL,
                PRIMARY KEY (service_name, interface_name, method_name)
            );

            CREATE TABLE IF NOT EXISTS service_properties (
                service_name TEXT NOT NULL,
                interface_name TEXT NOT NULL,
                property_name TEXT NOT NULL,
                type_signature TEXT NOT NULL,
                access TEXT NOT NULL,
                PRIMARY KEY (service_name, interface_name, property_name)
            );

            CREATE TABLE IF NOT EXISTS service_signals (
                service_name TEXT NOT NULL,
                interface_name TEXT NOT NULL,
                signal_name TEXT NOT NULL,
                signature_json TEXT NOT NULL,
                PRIMARY KEY (service_name, interface_name, signal_name)
            );
            "#,
        )?;

        Ok(Self { conn })
    }

    /// Store introspection XML as JSON
    pub fn store_introspection(
        &self,
        service_name: &str,
        object_path: &str,
        xml_data: &str,
    ) -> Result<()> {
        // Parse XML to structured format
        let node = XmlNode::from_reader(xml_data.as_bytes())
            .context("Failed to parse introspection XML")?;

        // Convert to JSON representation
        let json_data = self.xml_to_json(&node)?;
        let json_str = serde_json::to_string(&json_data)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Store in cache with JSON format
        for interface in node.interfaces() {
            let iface_name = interface.name().as_ref().to_string();

            self.conn.execute(
                "INSERT OR REPLACE INTO introspection_cache
                 (service_name, object_path, interface_name, cached_at, introspection_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    service_name,
                    object_path,
                    &iface_name,
                    timestamp,
                    &json_str,
                ],
            )?;

            // Store methods for fast lookup
            for method in interface.methods() {
                let method_name = method.name().as_ref().to_string();
                let method_json = serde_json::json!({
                    "name": method.name().as_ref(),
                    "in_args": method.args().iter()
                        .filter(|a| matches!(a.direction(), Some(zbus_xml::ArgDirection::In)))
                        .map(|a| {
                            serde_json::json!({
                                "name": a.name(),
                                "type": a.ty().to_string()
                            })
                        })
                        .collect::<Vec<_>>(),
                    "out_args": method.args().iter()
                        .filter(|a| matches!(a.direction(), Some(zbus_xml::ArgDirection::Out)))
                        .map(|a| {
                            serde_json::json!({
                                "name": a.name(),
                                "type": a.ty().to_string()
                            })
                        })
                        .collect::<Vec<_>>(),
                });

                self.conn.execute(
                    "INSERT OR REPLACE INTO service_methods
                     (service_name, interface_name, method_name, signature_json)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        service_name,
                        &iface_name,
                        &method_name,
                        serde_json::to_string(&method_json)?,
                    ],
                )?;
            }

            // Store properties
            for property in interface.properties() {
                let prop_name = property.name().as_ref().to_string();
                let access_str = match property.access() {
                    zbus_xml::PropertyAccess::Read => "read",
                    zbus_xml::PropertyAccess::Write => "write",
                    zbus_xml::PropertyAccess::ReadWrite => "readwrite",
                };

                self.conn.execute(
                    "INSERT OR REPLACE INTO service_properties
                     (service_name, interface_name, property_name, type_signature, access)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        service_name,
                        &iface_name,
                        &prop_name,
                        property.ty().to_string(),
                        access_str,
                    ],
                )?;
            }

            // Store signals
            for signal in interface.signals() {
                let signal_name = signal.name().as_ref().to_string();
                let signal_json = serde_json::json!({
                    "name": signal.name().as_ref(),
                    "args": signal.args().iter()
                        .map(|a| {
                            serde_json::json!({
                                "name": a.name(),
                                "type": a.ty().to_string()
                            })
                        })
                        .collect::<Vec<_>>(),
                });

                self.conn.execute(
                    "INSERT OR REPLACE INTO service_signals
                     (service_name, interface_name, signal_name, signature_json)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        service_name,
                        &iface_name,
                        &signal_name,
                        serde_json::to_string(&signal_json)?,
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// Get cached introspection as JSON
    pub fn get_introspection_json(
        &self,
        service_name: &str,
        object_path: &str,
        interface_name: Option<&str>,
    ) -> Result<Option<JsonValue>> {
        let query = if let Some(iface) = interface_name {
            self.conn.query_row(
                "SELECT introspection_json FROM introspection_cache
                 WHERE service_name = ?1 AND object_path = ?2 AND interface_name = ?3",
                params![service_name, object_path, iface],
                |row| row.get::<_, String>(0),
            )
        } else {
            self.conn.query_row(
                "SELECT introspection_json FROM introspection_cache
                 WHERE service_name = ?1 AND object_path = ?2
                 LIMIT 1",
                params![service_name, object_path],
                |row| row.get::<_, String>(0),
            )
        };

        match query {
            Ok(json_str) => Ok(Some(serde_json::from_str(&json_str)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all methods for a service interface
    pub fn get_methods_json(&self, service_name: &str, interface_name: &str) -> Result<JsonValue> {
        let mut stmt = self.conn.prepare(
            "SELECT method_name, signature_json FROM service_methods
             WHERE service_name = ?1 AND interface_name = ?2"
        )?;

        let methods: Vec<JsonValue> = stmt
            .query_map(params![service_name, interface_name], |row| {
                let json_str: String = row.get(1)?;
                Ok(serde_json::from_str(&json_str).unwrap())
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(serde_json::json!({ "methods": methods }))
    }

    /// Search for methods by name pattern
    pub fn search_methods(&self, pattern: &str) -> Result<Vec<JsonValue>> {
        let mut stmt = self.conn.prepare(
            "SELECT service_name, interface_name, signature_json
             FROM service_methods
             WHERE method_name LIKE ?1"
        )?;

        let results: Vec<JsonValue> = stmt
            .query_map(params![format!("%{}%", pattern)], |row| {
                let service: String = row.get(0)?;
                let interface: String = row.get(1)?;
                let json_str: String = row.get(2)?;
                let mut method: JsonValue = serde_json::from_str(&json_str).unwrap();

                if let Some(obj) = method.as_object_mut() {
                    obj.insert("service".to_string(), service.into());
                    obj.insert("interface".to_string(), interface.into());
                }

                Ok(method)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Convert XML Node to JSON
    fn xml_to_json(&self, node: &XmlNode) -> Result<JsonValue> {
        let interfaces: Vec<JsonValue> = node
            .interfaces()
            .iter()
            .map(|iface| {
                serde_json::json!({
                    "name": iface.name().as_ref(),
                    "methods": iface.methods().iter().map(|m| {
                        serde_json::json!({
                            "name": m.name().as_ref(),
                            "args": m.args().iter().map(|a| {
                                let dir = match a.direction() {
                                    Some(zbus_xml::ArgDirection::In) => "in",
                                    Some(zbus_xml::ArgDirection::Out) => "out",
                                    None => "in",
                                };
                                serde_json::json!({
                                    "name": a.name(),
                                    "type": a.ty().to_string(),
                                    "direction": dir
                                })
                            }).collect::<Vec<_>>()
                        })
                    }).collect::<Vec<_>>(),
                    "properties": iface.properties().iter().map(|p| {
                        let access = match p.access() {
                            zbus_xml::PropertyAccess::Read => "read",
                            zbus_xml::PropertyAccess::Write => "write",
                            zbus_xml::PropertyAccess::ReadWrite => "readwrite",
                        };
                        serde_json::json!({
                            "name": p.name().as_ref(),
                            "type": p.ty().to_string(),
                            "access": access
                        })
                    }).collect::<Vec<_>>(),
                    "signals": iface.signals().iter().map(|s| {
                        serde_json::json!({
                            "name": s.name().as_ref(),
                            "args": s.args().iter().map(|a| {
                                serde_json::json!({
                                    "name": a.name(),
                                    "type": a.ty().to_string()
                                })
                            }).collect::<Vec<_>>()
                        })
                    }).collect::<Vec<_>>()
                })
            })
            .collect();

        Ok(serde_json::json!({
            "interfaces": interfaces,
            "nodes": node.nodes().iter()
                .filter_map(|n| n.name().map(|name| name.to_string()))
                .collect::<Vec<_>>()
        }))
    }

    /// Clear old cache entries (older than days)
    pub fn clear_old_cache(&self, days: u64) -> Result<usize> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64
            - (days * 86400) as i64;

        let count = self.conn.execute(
            "DELETE FROM introspection_cache WHERE cached_at < ?1",
            params![cutoff],
        )?;

        Ok(count)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> Result<JsonValue> {
        let total_services: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT service_name) FROM introspection_cache",
            [],
            |row| row.get(0),
        )?;

        let total_interfaces: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM introspection_cache",
            [],
            |row| row.get(0),
        )?;

        let total_methods: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM service_methods",
            [],
            |row| row.get(0),
        )?;

        let db_size = std::fs::metadata(self.conn.path().unwrap())?.len();

        Ok(serde_json::json!({
            "services": total_services,
            "interfaces": total_interfaces,
            "methods": total_methods,
            "database_size_bytes": db_size
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_introspection_cache() -> Result<()> {
        let cache = IntrospectionCache::new(":memory:")?;

        let xml = r#"
        <node>
            <interface name="org.freedesktop.DBus">
                <method name="Hello">
                    <arg direction="out" type="s"/>
                </method>
                <property name="Features" type="as" access="read"/>
                <signal name="NameAcquired">
                    <arg type="s" name="name"/>
                </signal>
            </interface>
        </node>
        "#;

        cache.store_introspection("org.freedesktop.DBus", "/", xml)?;

        let json = cache.get_introspection_json("org.freedesktop.DBus", "/", None)?;
        assert!(json.is_some());

        let methods = cache.get_methods_json("org.freedesktop.DBus", "org.freedesktop.DBus")?;
        assert!(methods["methods"].as_array().unwrap().len() > 0);

        Ok(())
    }
}
