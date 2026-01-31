use sqlx::SqlitePool;
use anyhow::{anyhow, Result};
use std::path::Path;
use jsonschema::Draft;
use serde_json::Value;

pub struct PluginDefinition {
    pub name: &'static str,
    pub service_name: &'static str,
    pub schema_json: &'static str,
}

const PLUGIN_DEFINITIONS: &[PluginDefinition] = &[
    PluginDefinition {
        name: "directory",
        service_name: "org.opdbus.directory.v1",
        schema_json: r#"{
                "type": "DirectoryEntry",
                "description": "Enterprise directory and identity management",
                "replaces": ["Active Directory", "OpenLDAP", "FreeIPA"],
                "common_properties": {
                    "id": {"type": "uuid", "required": true},
                    "created_at": {"type": "timestamp", "required": true},
                    "updated_at": {"type": "timestamp", "required": true},
                    "object_type": {"type": "string", "required": true}
                },
                "object_types": {
                    "User": {
                        "description": "Directory user account",
                        "base_path": "/org/opdbus/directory/users",
                        "interface": "org.opdbus.directory.v1.User"
                    },
                    "Group": {
                        "description": "Directory group",
                        "base_path": "/org/opdbus/directory/groups",
                        "interface": "org.opdbus.directory.v1.Group"
                    },
                    "OrganizationalUnit": {
                        "description": "Organizational unit (OU)",
                        "base_path": "/org/opdbus/directory/ou",
                        "interface": "org.opdbus.directory.v1.OrganizationalUnit"
                    },
                    "Computer": {
                        "description": "Computer account",
                        "base_path": "/org/opdbus/directory/computers",
                        "interface": "org.opdbus.directory.v1.Computer"
                    }
                }
            }"#,
    },
    PluginDefinition {
        name: "network",
        service_name: "org.opdbus.network.v1",
        schema_json: r#"{
                "type": "NetworkObject",
                "description": "Modern network management with virtual networking",
                "replaces": ["NetworkManager", "systemd-networkd"],
                "common_properties": {
                    "id": {"type": "uuid", "required": true},
                    "name": {"type": "string", "required": true},
                    "state": {"type": "string", "required": true},
                    "created_at": {"type": "timestamp", "required": true}
                },
                "object_types": {
                    "Interface": {
                        "description": "Network interface",
                        "base_path": "/org/opdbus/network/interfaces",
                        "interface": "org.opdbus.network.v1.Interface"
                    },
                    "Bridge": {
                        "description": "Network bridge",
                        "base_path": "/org/opdbus/network/bridges",
                        "interface": "org.opdbus.network.v1.Bridge"
                    },
                    "VLAN": {
                        "description": "VLAN interface",
                        "base_path": "/org/opdbus/network/vlans",
                        "interface": "org.opdbus.network.v1.VLAN"
                    }
                }
            }"#,
    },
    PluginDefinition {
        name: "hardware",
        service_name: "org.opdbus.hardware.v1",
        schema_json: r#"{
                "type": "HardwareDevice",
                "description": "Unified hardware discovery and management",
                "replaces": ["lshw", "dmidecode"],
                "common_properties": {
                    "id": {"type": "uuid", "required": true},
                    "device_type": {"type": "string", "required": true},
                    "vendor": {"type": "string"},
                    "model": {"type": "string"}
                },
                "object_types": {
                    "CPU": {"base_path": "/org/opdbus/hardware/cpu", "interface": "org.opdbus.hardware.v1.CPU"},
                    "Memory": {"base_path": "/org/opdbus/hardware/memory", "interface": "org.opdbus.hardware.v1.Memory"},
                    "Disk": {"base_path": "/org/opdbus/hardware/disk", "interface": "org.opdbus.hardware.v1.Disk"}
                }
            }"#,
    },
    PluginDefinition {
        name: "storage",
        service_name: "org.opdbus.storage.v1",
        schema_json: r#"{
                "type": "StorageObject",
                "description": "Unified storage management",
                "replaces": ["LVM", "mdadm"],
                "common_properties": {
                    "id": {"type": "uuid", "required": true},
                    "name": {"type": "string", "required": true},
                    "size": {"type": "uint64"}
                },
                "object_types": {
                    "Volume": {"base_path": "/org/opdbus/storage/volumes", "interface": "org.opdbus.storage.v1.Volume"},
                    "Filesystem": {"base_path": "/org/opdbus/storage/filesystems", "interface": "org.opdbus.storage.v1.Filesystem"}
                }
            }"#,
    },
    PluginDefinition {
        name: "container",
        service_name: "org.opdbus.container.v1",
        schema_json: r#"{
                "type": "ContainerObject",
                "description": "Container lifecycle and orchestration",
                "replaces": ["Docker", "Podman"],
                "common_properties": {
                    "id": {"type": "uuid", "required": true},
                    "name": {"type": "string", "required": true},
                    "state": {"type": "string", "required": true}
                },
                "object_types": {
                    "Container": {"base_path": "/org/opdbus/container/containers", "interface": "org.opdbus.container.v1.Container"},
                    "Image": {"base_path": "/org/opdbus/container/images", "interface": "org.opdbus.container.v1.Image"}
                }
            }"#,
    },
];

pub fn plugin_definitions() -> &'static [PluginDefinition] {
    PLUGIN_DEFINITIONS
}

pub fn get_plugin_schema_json(plugin: &str) -> Option<&'static str> {
    PLUGIN_DEFINITIONS
        .iter()
        .find(|p| p.name == plugin)
        .map(|p| p.schema_json)
}

pub fn validate_plugin_schemas_from_repo() -> Result<()> {
    let official_meta_path = Path::new("/git/json-schema-spec/specs/meta/meta.json");
    validate_plugin_schemas(official_meta_path)
}

pub fn validate_plugin_schemas(official_meta_schema_path: &Path) -> Result<()> {
    let official_meta_schema: Value = if official_meta_schema_path.exists() {
        let official_meta_str = std::fs::read_to_string(official_meta_schema_path)
            .map_err(|e| anyhow!("Failed to read meta-schema {}: {}", official_meta_schema_path.display(), e))?;
        serde_json::from_str(&official_meta_str)
            .map_err(|e| anyhow!("Failed to parse official meta-schema JSON: {}", e))?
    } else {
        let embedded = include_str!("../schemas/jsonschema-meta.json");
        serde_json::from_str(embedded)
            .map_err(|e| anyhow!("Failed to parse embedded meta-schema JSON: {}", e))?
    };

    let plugin_meta_schema: Value = {
        let embedded = include_str!("../schemas/opdbus-plugin-schema.json");
        serde_json::from_str(embedded)
            .map_err(|e| anyhow!("Failed to parse embedded plugin meta-schema JSON: {}", e))?
    };

    // Validate our plugin meta-schema against the official meta-schema.
    let official_compiled = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .build(&official_meta_schema)
        .map_err(|e| anyhow!("Failed to compile official meta-schema: {}", e))?;
    let errors: Vec<String> = official_compiled
        .iter_errors(&plugin_meta_schema)
        .map(|err| format!("meta-schema error: {} at {}", err, err.instance_path))
        .collect();
    if !errors.is_empty() {
        return Err(anyhow!(
            "Plugin meta-schema validation failed:\n{}",
            errors.join("\n")
        ));
    }

    // Validate plugin schemas against the plugin meta-schema.
    let plugin_compiled = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .build(&plugin_meta_schema)
        .map_err(|e| anyhow!("Failed to compile plugin meta-schema: {}", e))?;

    let mut errors = Vec::new();
    for plugin in PLUGIN_DEFINITIONS {
        let schema_value: Value = serde_json::from_str(plugin.schema_json)
            .map_err(|e| anyhow!("Invalid JSON for plugin {}: {}", plugin.name, e))?;
        for err in plugin_compiled.iter_errors(&schema_value) {
            errors.push(format!(
                "plugin {}: {} at {}",
                plugin.name,
                err,
                err.instance_path
            ));
        }
    }

    if !errors.is_empty() {
        return Err(anyhow!(
            "Plugin schema validation failed:\n{}",
            errors.join("\n")
        ));
    }

    Ok(())
}

pub async fn insert_plugins(pool: &SqlitePool) -> Result<()> {
    for plugin in PLUGIN_DEFINITIONS {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO plugins (name, service_name, base_object)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(plugin.name)
        .bind(plugin.service_name)
        .bind(plugin.schema_json)
        .execute(pool)
        .await?;
    }

    Ok(())
}
