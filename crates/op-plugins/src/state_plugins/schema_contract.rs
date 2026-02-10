//! Unified schema-as-code contract for state plugins.
//!
//! Every plugin object follows the same envelope:
//! - stub
//! - immutable
//! - tunable
//! - observed
//! plus meta, semantic_index, and privacy_index sections.

use simd_json::{json, OwnedValue as Value};
use std::collections::HashMap;

fn contract_schema(
    plugin: &str,
    object_type: &str,
    tunable_schema: Value,
    observed_schema: Value,
    dependencies: Vec<&str>,
    include_in_recovery: bool,
    recovery_priority: u32,
    sensitivity: &str,
    semantic_include_paths: Vec<&str>,
    semantic_exclude_paths: Vec<&str>,
    pii_paths: Vec<&str>,
    secret_paths: Vec<&str>,
) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": format!("https://op-dbus.local/schemas/plugins/{plugin}.contract.json"),
        "title": format!("{plugin} contract schema"),
        "type": "object",
        "required": [
            "schema_version",
            "plugin",
            "object_type",
            "object_id",
            "stub",
            "immutable",
            "tunable",
            "observed",
            "meta",
            "semantic_index",
            "privacy_index"
        ],
        "properties": {
            "schema_version": {
                "type": "string",
                "const": "1.0.0"
            },
            "plugin": {
                "type": "string",
                "const": plugin
            },
            "object_type": {
                "type": "string",
                "const": object_type
            },
            "object_id": {
                "type": "string",
                "minLength": 1
            },
            "stub": {
                "type": "object",
                "required": ["system_id", "source", "source_ref", "discovered_at"],
                "properties": {
                    "system_id": { "type": "string", "minLength": 1 },
                    "source": { "type": "string", "minLength": 1 },
                    "source_ref": { "type": "string", "minLength": 1 },
                    "discovered_at": { "type": "string", "format": "date-time" }
                },
                "additionalProperties": false
            },
            "immutable": {
                "type": "object",
                "required": ["created_at", "created_by_plugin", "identity_keys", "provider"],
                "properties": {
                    "created_at": { "type": "string", "format": "date-time" },
                    "created_by_plugin": { "type": "string", "const": plugin },
                    "identity_keys": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1
                    },
                    "provider": { "type": "string", "minLength": 1 }
                },
                "additionalProperties": false
            },
            "tunable": tunable_schema,
            "observed": observed_schema,
            "meta": {
                "type": "object",
                "required": [
                    "dependencies",
                    "include_in_recovery",
                    "recovery_priority",
                    "sensitivity",
                    "tags",
                    "enabled"
                ],
                "properties": {
                    "dependencies": {
                        "type": "array",
                        "items": { "type": "string" },
                        "default": dependencies
                    },
                    "include_in_recovery": { "type": "boolean", "default": include_in_recovery },
                    "recovery_priority": { "type": "integer", "minimum": 0, "default": recovery_priority },
                    "sensitivity": {
                        "type": "string",
                        "enum": ["public", "internal", "secret"],
                        "default": sensitivity
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "default": []
                    },
                    "enabled": { "type": "boolean", "default": true }
                },
                "additionalProperties": false
            },
            "semantic_index": {
                "type": "object",
                "required": ["include_paths", "exclude_paths", "chunking", "redaction"],
                "properties": {
                    "include_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "default": semantic_include_paths
                    },
                    "exclude_paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "default": semantic_exclude_paths
                    },
                    "chunking": {
                        "type": "object",
                        "required": ["strategy", "max_tokens"],
                        "properties": {
                            "strategy": { "type": "string", "enum": ["json-path-group"], "default": "json-path-group" },
                            "max_tokens": { "type": "integer", "minimum": 64, "default": 512 }
                        },
                        "additionalProperties": false
                    },
                    "redaction": {
                        "type": "object",
                        "required": ["enabled"],
                        "properties": {
                            "enabled": { "type": "boolean", "default": true }
                        },
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            },
            "privacy_index": {
                "type": "object",
                "required": ["redaction"],
                "properties": {
                    "redaction": {
                        "type": "object",
                        "required": [
                            "rules",
                            "default_action",
                            "secret_paths",
                            "pii_paths",
                            "hash_salt_ref",
                            "reversible"
                        ],
                        "properties": {
                            "rules": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["path", "action"],
                                    "properties": {
                                        "path": { "type": "string" },
                                        "action": { "type": "string", "enum": ["drop", "mask", "hash"] },
                                        "reason": { "type": "string" }
                                    },
                                    "additionalProperties": false
                                },
                                "default": []
                            },
                            "default_action": {
                                "type": "string",
                                "enum": ["drop", "mask", "hash"],
                                "default": "mask"
                            },
                            "secret_paths": {
                                "type": "array",
                                "items": { "type": "string" },
                                "default": secret_paths
                            },
                            "pii_paths": {
                                "type": "array",
                                "items": { "type": "string" },
                                "default": pii_paths
                            },
                            "hash_salt_ref": {
                                "type": "string",
                                "default": "vault://op-dbus/privacy/hash-salt"
                            },
                            "reversible": {
                                "type": "boolean",
                                "default": false
                            }
                        },
                        "additionalProperties": false
                    }
                },
                "additionalProperties": false
            }
        },
        "additionalProperties": false
    })
}

fn tunable_object(properties: Value, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn observed_object() -> Value {
    json!({
        "type": "object",
        "required": ["last_observed_at"],
        "properties": {
            "last_observed_at": { "type": "string", "format": "date-time" },
            "status": { "type": "string" },
            "drift_detected": { "type": "boolean", "default": false },
            "metrics": { "type": "object" }
        },
        "additionalProperties": true
    })
}

/// Get contract schema for a single plugin.
pub fn schema_for_plugin(plugin: &str) -> Option<Value> {
    Some(match plugin {
        "adc" => contract_schema(
            "adc",
            "adc_state",
            tunable_object(
                json!({"configured": {"type": "boolean"}}),
                vec!["configured"],
            ),
            observed_object(),
            vec![],
            true,
            60,
            "internal",
            vec!["/tunable/configured"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "config" => contract_schema(
            "config",
            "config_store",
            tunable_object(
                json!({
                    "configs": {
                        "type": "object",
                        "additionalProperties": true
                    }
                }),
                vec!["configs"],
            ),
            observed_object(),
            vec![],
            true,
            1,
            "internal",
            vec!["/tunable/configs"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "agent_config" => contract_schema(
            "agent_config",
            "agent_config",
            tunable_object(
                json!({
                    "agents": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "enabled", "tools"],
                            "properties": {
                                "name": {"type": "string"},
                                "enabled": {"type": "boolean"},
                                "model": {"type": "string"},
                                "tools": {"type": "array", "items": {"type": "string"}}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["agents"],
            ),
            observed_object(),
            vec![],
            true,
            30,
            "internal",
            vec!["/tunable/agents"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "endpoint" => contract_schema(
            "endpoint",
            "endpoint",
            tunable_object(
                json!({
                    "endpoints": {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                }),
                vec!["endpoints"],
            ),
            observed_object(),
            vec!["net"],
            true,
            50,
            "internal",
            vec!["/tunable/endpoints"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "gcloud_adc" => contract_schema(
            "gcloud_adc",
            "gcloud_adc_state",
            tunable_object(
                json!({
                    "account": {"type": "string"},
                    "project_id": {"type": "string"},
                    "authenticated": {"type": "boolean"}
                }),
                vec!["authenticated"],
            ),
            observed_object(),
            vec![],
            false,
            95,
            "secret",
            vec!["/tunable/project_id", "/tunable/authenticated"],
            vec!["/tunable/account", "/stub/discovered_at"],
            vec!["/tunable/account"],
            vec!["/tunable/account"],
        ),
        "hardware" => contract_schema(
            "hardware",
            "hardware_state",
            tunable_object(
                json!({
                    "cpu": {
                        "type": "object",
                        "required": ["model", "cores"],
                        "properties": {"model": {"type": "string"}, "cores": {"type": "integer", "minimum": 1}},
                        "additionalProperties": false
                    },
                    "memory": {
                        "type": "object",
                        "required": ["total_kb", "available_kb"],
                        "properties": {"total_kb": {"type": "integer", "minimum": 0}, "available_kb": {"type": "integer", "minimum": 0}},
                        "additionalProperties": false
                    },
                    "disks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "size_bytes"],
                            "properties": {
                                "name": {"type": "string"},
                                "size_bytes": {"type": "integer", "minimum": 0},
                                "mountpoint": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["cpu", "memory", "disks"],
            ),
            observed_object(),
            vec![],
            true,
            70,
            "internal",
            vec!["/tunable/cpu", "/tunable/memory", "/tunable/disks"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "keypair" => contract_schema(
            "keypair",
            "keypair_set",
            tunable_object(
                json!({
                    "keypairs": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "algorithm", "present"],
                            "properties": {
                                "name": {"type": "string"},
                                "algorithm": {"type": "string"},
                                "public_key": {"type": "string"},
                                "present": {"type": "boolean"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["keypairs"],
            ),
            observed_object(),
            vec![],
            true,
            40,
            "internal",
            vec!["/tunable/keypairs"],
            vec!["/stub/discovered_at"],
            vec![],
            vec!["/tunable/keypairs/*/public_key"],
        ),
        "mcp" => contract_schema(
            "mcp",
            "mcp_config",
            tunable_object(
                json!({
                    "servers": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "required": ["command", "enabled", "transport"],
                            "properties": {
                                "command": {"type": "string"},
                                "args": {"type": "array", "items": {"type": "string"}},
                                "env": {"type": "object", "additionalProperties": {"type": "string"}},
                                "enabled": {"type": "boolean"},
                                "transport": {"type": "string", "enum": ["stdio", "sse", "http"]}
                            },
                            "additionalProperties": false
                        }
                    },
                    "tool_groups": {
                        "type": "object",
                        "required": ["enabled", "max_tools"],
                        "properties": {
                            "enabled": {"type": "array", "items": {"type": "string"}},
                            "max_tools": {"type": "integer", "minimum": 1},
                            "access_zone": {"type": "string"},
                            "trusted_networks": {"type": "array", "items": {"type": "string"}}
                        },
                        "additionalProperties": false
                    },
                    "compact_mode": {
                        "type": "object",
                        "required": ["enabled", "meta_tools"],
                        "properties": {
                            "enabled": {"type": "boolean"},
                            "meta_tools": {"type": "array", "items": {"type": "string"}}
                        },
                        "additionalProperties": false
                    }
                }),
                vec![],
            ),
            observed_object(),
            vec!["agent_config"],
            true,
            20,
            "internal",
            vec![
                "/tunable/servers",
                "/tunable/tool_groups",
                "/tunable/compact_mode",
            ],
            vec!["/stub/discovered_at", "/tunable/servers/*/env"],
            vec![],
            vec!["/tunable/servers/*/env"],
        ),
        "net" => contract_schema(
            "net",
            "network_config",
            tunable_object(
                json!({
                    "interfaces": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "type"],
                            "properties": {
                                "name": {"type": "string"},
                                "type": {"type": "string"},
                                "driver": {"type": "string"},
                                "ports": {"type": "array", "items": {"type": "string"}},
                                "l3_driver": {"type": "string"},
                                "ipv4": {"type": "object", "additionalProperties": true},
                                "ipv6": {"type": "object", "additionalProperties": true},
                                "controller": {"type": "string"},
                                "properties": {"type": "object", "additionalProperties": true},
                                "property_schema": {"type": "array", "items": {"type": "string"}}
                            },
                            "additionalProperties": true
                        }
                    }
                }),
                vec!["interfaces"],
            ),
            observed_object(),
            vec![],
            true,
            10,
            "internal",
            vec!["/tunable/interfaces"],
            vec!["/stub/discovered_at"],
            vec!["/tunable/interfaces/*/properties/mac_addresses"],
            vec![],
        ),
        "netmaker" => contract_schema(
            "netmaker",
            "netmaker_mesh_config",
            tunable_object(
                json!({
                    "config": {
                        "type": "object",
                        "required": ["enabled", "default_network"],
                        "properties": {
                            "enabled": {"type": "boolean"},
                            "default_network": {"type": "string"},
                            "enrollment_token": {"type": "string"},
                            "api_endpoint": {"type": "string"}
                        },
                        "additionalProperties": false
                    }
                }),
                vec!["config"],
            ),
            observed_object(),
            vec!["net"],
            true,
            17,
            "secret",
            vec!["/tunable/config/default_network"],
            vec!["/stub/discovered_at", "/tunable/config/enrollment_token"],
            vec!["/tunable/config/api_endpoint"],
            vec!["/tunable/config/enrollment_token"],
        ),
        "ovsdb_bridge" => contract_schema(
            "ovsdb_bridge",
            "ovs_bridge_state",
            tunable_object(
                json!({
                    "bridges": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "ports"],
                            "properties": {
                                "name": {"type": "string"},
                                "ports": {"type": "array", "items": {"type": "string"}}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["bridges"],
            ),
            observed_object(),
            vec!["net"],
            true,
            15,
            "internal",
            vec!["/tunable/bridges"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "proxmox" => contract_schema(
            "proxmox",
            "proxmox_container_set",
            tunable_object(
                json!({
                    "containers": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["vmid", "status"],
                            "properties": {
                                "vmid": {"type": "integer", "minimum": 1},
                                "hostname": {"type": "string"},
                                "status": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["containers"],
            ),
            observed_object(),
            vec!["net"],
            true,
            35,
            "internal",
            vec!["/tunable/containers"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "proxy_server" => contract_schema(
            "proxy_server",
            "proxy_server_state",
            tunable_object(
                json!({
                    "enabled": {"type": "boolean"},
                    "port": {"type": "integer", "minimum": 1, "maximum": 65535}
                }),
                vec!["enabled", "port"],
            ),
            observed_object(),
            vec!["net"],
            true,
            45,
            "internal",
            vec!["/tunable/enabled", "/tunable/port"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "service" => contract_schema(
            "service",
            "service_definition_set",
            tunable_object(
                json!({
                    "services": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "required": ["exec_start", "enabled"],
                            "properties": {
                                "name": {"type": "string"},
                                "exec_start": {
                                    "type": "object",
                                    "required": ["program", "args"],
                                    "properties": {
                                        "program": {"type": "string"},
                                        "args": {"type": "array", "items": {"type": "string"}}
                                    },
                                    "additionalProperties": false
                                },
                                "exec_stop": {
                                    "type": "object",
                                    "required": ["program", "args"],
                                    "properties": {
                                        "program": {"type": "string"},
                                        "args": {"type": "array", "items": {"type": "string"}}
                                    },
                                    "additionalProperties": false
                                },
                                "working_dir": {"type": "string"},
                                "user": {"type": "string"},
                                "depends_on": {"type": "array", "items": {"type": "string"}},
                                "environment": {"type": "object", "additionalProperties": {"type": "string"}},
                                "enabled": {"type": "boolean"},
                                "lifecycle": {"type": "object", "additionalProperties": true}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["services"],
            ),
            observed_object(),
            vec!["net"],
            true,
            5,
            "internal",
            vec!["/tunable/services"],
            vec!["/stub/discovered_at", "/tunable/services/*/environment"],
            vec![],
            vec!["/tunable/services/*/environment"],
        ),
        "sess_decl" => contract_schema(
            "sess_decl",
            "session_set",
            tunable_object(
                json!({
                    "sessions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "user"],
                            "properties": {
                                "id": {"type": "string"},
                                "user": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["sessions"],
            ),
            observed_object(),
            vec!["users"],
            true,
            25,
            "internal",
            vec!["/tunable/sessions"],
            vec!["/stub/discovered_at"],
            vec!["/tunable/sessions/*/user"],
            vec![],
        ),
        "software" => contract_schema(
            "software",
            "software_package_set",
            tunable_object(
                json!({
                    "packages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "version", "manager"],
                            "properties": {
                                "name": {"type": "string"},
                                "version": {"type": "string"},
                                "manager": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["packages"],
            ),
            observed_object(),
            vec![],
            true,
            65,
            "internal",
            vec!["/tunable/packages"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "dinit" => contract_schema(
            "dinit",
            "service_runtime_config",
            tunable_object(
                json!({
                    "services": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "properties": {
                                "state": {"type": "string"},
                                "enabled": {"type": "boolean"},
                                "properties": {"type": "object", "additionalProperties": true}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["services"],
            ),
            observed_object(),
            vec!["service"],
            true,
            8,
            "internal",
            vec!["/tunable/services"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "users" => contract_schema(
            "users",
            "user_set",
            tunable_object(
                json!({
                    "users": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["username", "groups", "present"],
                            "properties": {
                                "username": {"type": "string"},
                                "uid": {"type": "integer", "minimum": 0},
                                "gid": {"type": "integer", "minimum": 0},
                                "groups": {"type": "array", "items": {"type": "string"}},
                                "shell": {"type": "string"},
                                "present": {"type": "boolean"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["users"],
            ),
            observed_object(),
            vec![],
            true,
            12,
            "secret",
            vec!["/tunable/users"],
            vec!["/stub/discovered_at"],
            vec!["/tunable/users/*/username", "/tunable/users/*/shell"],
            vec![],
        ),
        "web_ui" | "web-ui" => contract_schema(
            "web_ui",
            "web_ui_tunables",
            tunable_object(
                json!({
                    "enabled": {"type": "boolean"},
                    "cors_origins": {"type": "array", "items": {"type": "string"}},
                    "compression": {"type": "boolean"},
                    "cache_ttl": {"type": "integer", "minimum": 0},
                    "theme": {"type": "string"},
                    "feature_flags": {"type": "object", "additionalProperties": {"type": "boolean"}}
                }),
                vec!["enabled", "compression", "cache_ttl", "theme"],
            ),
            observed_object(),
            vec!["mcp"],
            true,
            55,
            "internal",
            vec!["/tunable/theme", "/tunable/feature_flags"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "wireguard" => contract_schema(
            "wireguard",
            "wireguard_config",
            tunable_object(
                json!({
                    "interfaces": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "listen_port", "peers"],
                            "properties": {
                                "name": {"type": "string"},
                                "private_key": {"type": "string"},
                                "listen_port": {"type": "integer", "minimum": 1, "maximum": 65535},
                                "peers": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "required": ["public_key", "allowed_ips"],
                                        "properties": {
                                            "public_key": {"type": "string"},
                                            "allowed_ips": {"type": "array", "items": {"type": "string"}},
                                            "endpoint": {"type": "string"}
                                        },
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["interfaces"],
            ),
            observed_object(),
            vec!["net"],
            true,
            18,
            "secret",
            vec!["/tunable/interfaces/*/name", "/tunable/interfaces/*/peers"],
            vec!["/stub/discovered_at", "/tunable/interfaces/*/private_key"],
            vec!["/tunable/interfaces/*/peers/*/endpoint"],
            vec!["/tunable/interfaces/*/private_key"],
        ),
        "dnsresolver" => contract_schema(
            "dnsresolver",
            "dns_resolver_state",
            tunable_object(
                json!({
                    "version": {"type": "integer", "minimum": 1},
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "mode", "servers"],
                            "properties": {
                                "id": {"type": "string"},
                                "mode": {"type": "string", "enum": ["enforce", "observe-only"]},
                                "servers": {"type": "array", "items": {"type": "string"}},
                                "search": {"type": "array", "items": {"type": "string"}},
                                "options": {"type": "array", "items": {"type": "string"}}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["items"],
            ),
            observed_object(),
            vec!["net"],
            true,
            14,
            "internal",
            vec!["/tunable/items"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "full_system" => contract_schema(
            "full_system",
            "full_system_snapshot",
            tunable_object(
                json!({
                    "version": {"type": "integer", "minimum": 1},
                    "captured_at": {"type": "string", "format": "date-time"},
                    "hostname": {"type": "string"},
                    "system": {
                        "type": "object",
                        "properties": {
                            "kernel_version": {"type": "string"},
                            "os_release": {"type": "string"},
                            "timezone": {"type": "string"},
                            "locale": {"type": "string"},
                            "uptime_seconds": {"type": "integer", "minimum": 0}
                        },
                        "additionalProperties": false
                    },
                    "network": {
                        "type": "object",
                        "properties": {
                            "interfaces": {"type": "array", "items": {"type": "object", "additionalProperties": true}},
                            "routes": {"type": "array", "items": {"type": "object", "additionalProperties": true}},
                            "dns_servers": {"type": "array", "items": {"type": "string"}},
                            "bridges": {"type": "array", "items": {"type": "object", "additionalProperties": true}}
                        },
                        "additionalProperties": false
                    },
                    "services": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "enabled", "running", "unit_type"],
                            "properties": {
                                "name": {"type": "string"},
                                "enabled": {"type": "boolean"},
                                "running": {"type": "boolean"},
                                "unit_type": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    },
                    "packages": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "version", "arch"],
                            "properties": {
                                "name": {"type": "string"},
                                "version": {"type": "string"},
                                "arch": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    },
                    "users": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "uid", "gid", "home", "shell", "groups"],
                            "properties": {
                                "name": {"type": "string"},
                                "uid": {"type": "integer", "minimum": 0},
                                "gid": {"type": "integer", "minimum": 0},
                                "home": {"type": "string"},
                                "shell": {"type": "string"},
                                "groups": {"type": "array", "items": {"type": "string"}}
                            },
                            "additionalProperties": false
                        }
                    },
                    "storage": {"type": "object", "additionalProperties": true},
                    "containers": {"type": "object", "additionalProperties": true},
                    "plugins": {"type": "object", "additionalProperties": true}
                }),
                vec!["version", "captured_at", "hostname"],
            ),
            observed_object(),
            vec!["net", "service", "software", "users", "lxc", "dinit"],
            true,
            2,
            "internal",
            vec![
                "/tunable/hostname",
                "/tunable/system",
                "/tunable/network",
                "/tunable/services",
                "/tunable/packages",
                "/tunable/users",
                "/tunable/storage",
                "/tunable/containers",
                "/tunable/plugins",
            ],
            vec!["/stub/discovered_at"],
            vec!["/tunable/users"],
            vec![],
        ),
        "keyring" => contract_schema(
            "keyring",
            "secret_service_state",
            tunable_object(
                json!({
                    "collections": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["path", "label", "locked"],
                            "properties": {
                                "path": {"type": "string"},
                                "label": {"type": "string"},
                                "locked": {"type": "boolean"},
                                "created": {"type": "integer", "minimum": 0},
                                "modified": {"type": "integer", "minimum": 0}
                            },
                            "additionalProperties": false
                        }
                    },
                    "default_collection": {"type": "string"}
                }),
                vec!["collections"],
            ),
            observed_object(),
            vec![],
            false,
            92,
            "secret",
            vec![
                "/tunable/collections/*/label",
                "/tunable/default_collection",
            ],
            vec!["/stub/discovered_at"],
            vec!["/tunable/collections/*/label"],
            vec!["/tunable/default_collection"],
        ),
        "login1" => contract_schema(
            "login1",
            "session_runtime_state",
            tunable_object(
                json!({
                    "sessions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "uid", "user", "seat", "path"],
                            "properties": {
                                "id": {"type": "string"},
                                "uid": {"type": "integer", "minimum": 0},
                                "user": {"type": "string"},
                                "seat": {"type": "string"},
                                "path": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["sessions"],
            ),
            observed_object(),
            vec!["users"],
            false,
            78,
            "internal",
            vec!["/tunable/sessions"],
            vec!["/stub/discovered_at"],
            vec!["/tunable/sessions/*/user"],
            vec![],
        ),
        "lxc" => contract_schema(
            "lxc",
            "container_network_state",
            tunable_object(
                json!({
                    "containers": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "veth", "bridge"],
                            "properties": {
                                "id": {"type": "string"},
                                "veth": {"type": "string"},
                                "bridge": {"type": "string"},
                                "running": {"type": "boolean"},
                                "properties": {"type": "object", "additionalProperties": true}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["containers"],
            ),
            observed_object(),
            vec!["net", "openflow"],
            true,
            16,
            "internal",
            vec!["/tunable/containers"],
            vec!["/stub/discovered_at"],
            vec!["/tunable/containers/*/properties/hostname"],
            vec![],
        ),
        "openflow" => contract_schema(
            "openflow",
            "openflow_policy_state",
            tunable_object(
                json!({
                    "bridges": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "flows"],
                            "properties": {
                                "name": {"type": "string"},
                                "flows": {"type": "array", "items": {"type": "object", "additionalProperties": true}},
                                "socket_ports": {"type": "array", "items": {"type": "object", "additionalProperties": true}}
                            },
                            "additionalProperties": false
                        }
                    },
                    "controller_endpoint": {"type": "string"},
                    "flow_policies": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["name", "selector", "template"],
                            "properties": {
                                "name": {"type": "string"},
                                "selector": {"type": "string"},
                                "template": {"type": "object", "additionalProperties": true}
                            },
                            "additionalProperties": false
                        }
                    },
                    "auto_discover_containers": {"type": "boolean"},
                    "enable_security_flows": {"type": "boolean"},
                    "obfuscation_level": {"type": "integer", "minimum": 0, "maximum": 3}
                }),
                vec!["bridges"],
            ),
            observed_object(),
            vec!["net", "ovsdb_bridge", "lxc"],
            true,
            11,
            "internal",
            vec!["/tunable/bridges", "/tunable/flow_policies"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "openflow_obfuscation" => contract_schema(
            "openflow_obfuscation",
            "openflow_obfuscation_config",
            tunable_object(
                json!({
                    "config": {
                        "type": "object",
                        "required": ["bridge_name", "obfuscation_level", "enable_security_flows", "privacy_ports", "custom_flows"],
                        "properties": {
                            "bridge_name": {"type": "string"},
                            "obfuscation_level": {"type": "integer", "minimum": 0, "maximum": 3},
                            "enable_security_flows": {"type": "boolean"},
                            "privacy_ports": {"type": "array", "items": {"type": "string"}},
                            "custom_flows": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "required": ["table", "priority", "match_spec", "actions", "description"],
                                    "properties": {
                                        "table": {"type": "integer", "minimum": 0, "maximum": 254},
                                        "priority": {"type": "integer", "minimum": 0, "maximum": 65535},
                                        "match_spec": {"type": "string"},
                                        "actions": {"type": "string"},
                                        "description": {"type": "string"}
                                    },
                                    "additionalProperties": false
                                }
                            }
                        },
                        "additionalProperties": false
                    }
                }),
                vec!["config"],
            ),
            observed_object(),
            vec!["openflow", "net"],
            true,
            13,
            "internal",
            vec!["/tunable/config/bridge_name", "/tunable/config/obfuscation_level"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "packagekit" => contract_schema(
            "packagekit",
            "package_declaration_state",
            tunable_object(
                json!({
                    "version": {"type": "integer", "minimum": 1},
                    "packages": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "required": ["ensure"],
                            "properties": {
                                "ensure": {"type": "string", "enum": ["installed", "removed", "latest"]},
                                "provider": {"type": "string"},
                                "version": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["packages"],
            ),
            observed_object(),
            vec!["software"],
            true,
            62,
            "internal",
            vec!["/tunable/packages"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "privacy" => contract_schema(
            "privacy",
            "privacy_coordination_config",
            tunable_object(
                json!({
                    "config": {
                        "type": "object",
                        "required": [
                            "wireguard_gateway_enabled",
                            "wireguard_interface",
                            "warp_tunnel_enabled",
                            "warp_interface",
                            "xray_client_enabled",
                            "xray_client_container_id",
                            "xray_socks_port",
                            "proxmox_bridge"
                        ],
                        "properties": {
                            "wireguard_gateway_enabled": {"type": "boolean"},
                            "wireguard_interface": {"type": "string"},
                            "warp_tunnel_enabled": {"type": "boolean"},
                            "warp_interface": {"type": "string"},
                            "xray_client_enabled": {"type": "boolean"},
                            "xray_client_container_id": {"type": "integer", "minimum": 1},
                            "xray_socks_port": {"type": "integer", "minimum": 1, "maximum": 65535},
                            "vps_xray_server": {"type": "string"},
                            "proxmox_bridge": {"type": "string"}
                        },
                        "additionalProperties": false
                    }
                }),
                vec!["config"],
            ),
            observed_object(),
            vec!["wireguard", "proxmox", "privacy_router"],
            true,
            26,
            "secret",
            vec![
                "/tunable/config/wireguard_gateway_enabled",
                "/tunable/config/warp_tunnel_enabled",
                "/tunable/config/xray_client_enabled",
            ],
            vec!["/stub/discovered_at", "/tunable/config/vps_xray_server"],
            vec!["/tunable/config/vps_xray_server"],
            vec![],
        ),
        "privacy_router" => contract_schema(
            "privacy_router",
            "privacy_router_tunnel_config",
            tunable_object(
                json!({
                    "config": {
                        "type": "object",
                        "required": ["bridge_name", "wireguard", "warp", "xray", "vps", "socket_networking", "openflow", "netmaker", "containers"],
                        "properties": {
                            "bridge_name": {"type": "string"},
                            "wireguard": {"type": "object", "additionalProperties": true},
                            "warp": {"type": "object", "additionalProperties": true},
                            "xray": {"type": "object", "additionalProperties": true},
                            "vps": {"type": "object", "additionalProperties": true},
                            "socket_networking": {"type": "object", "additionalProperties": true},
                            "openflow": {"type": "object", "additionalProperties": true},
                            "netmaker": {"type": "object", "additionalProperties": true},
                            "containers": {"type": "array", "items": {"type": "object", "additionalProperties": true}}
                        },
                        "additionalProperties": false
                    }
                }),
                vec!["config"],
            ),
            observed_object(),
            vec!["net", "lxc", "openflow", "netmaker", "openflow_obfuscation"],
            true,
            24,
            "secret",
            vec![
                "/tunable/config/bridge_name",
                "/tunable/config/wireguard",
                "/tunable/config/warp",
                "/tunable/config/xray",
                "/tunable/config/openflow",
                "/tunable/config/netmaker",
            ],
            vec![
                "/stub/discovered_at",
                "/tunable/config/xray/vps_address",
                "/tunable/config/warp/warp_license",
            ],
            vec!["/tunable/config/xray/vps_address"],
            vec!["/tunable/config/warp/warp_license"],
        ),
        "pcidecl" => contract_schema(
            "pcidecl",
            "pci_declaration_state",
            tunable_object(
                json!({
                    "version": {"type": "integer", "minimum": 1},
                    "items": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["id", "mode", "address"],
                            "properties": {
                                "id": {"type": "string"},
                                "mode": {"type": "string", "enum": ["enforce", "observe-only"]},
                                "address": {"type": "string"},
                                "expect_vendor": {"type": "string"},
                                "expect_device": {"type": "string"},
                                "driver_override": {"type": "string"}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["items"],
            ),
            observed_object(),
            vec!["hardware"],
            true,
            52,
            "internal",
            vec!["/tunable/items"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        "systemd" => contract_schema(
            "systemd",
            "systemd_unit_state",
            tunable_object(
                json!({
                    "units": {
                        "type": "object",
                        "additionalProperties": {
                            "type": "object",
                            "properties": {
                                "active_state": {"type": "string"},
                                "enabled": {"type": "boolean"},
                                "masked": {"type": "boolean"},
                                "properties": {"type": "object", "additionalProperties": true}
                            },
                            "additionalProperties": false
                        }
                    }
                }),
                vec!["units"],
            ),
            observed_object(),
            vec!["service"],
            true,
            9,
            "internal",
            vec!["/tunable/units"],
            vec!["/stub/discovered_at"],
            vec![],
            vec![],
        ),
        _ => return None,
    })
}

/// Get all contract schemas keyed by plugin name.
pub fn all_contract_schemas() -> HashMap<String, Value> {
    const PLUGINS: &[&str] = &[
        "adc",
        "agent_config",
        "config",
        "dinit",
        "dnsresolver",
        "endpoint",
        "full_system",
        "gcloud_adc",
        "hardware",
        "keypair",
        "keyring",
        "login1",
        "lxc",
        "mcp",
        "net",
        "netmaker",
        "openflow",
        "openflow_obfuscation",
        "ovsdb_bridge",
        "packagekit",
        "pcidecl",
        "privacy",
        "privacy_router",
        "proxmox",
        "proxy_server",
        "service",
        "sess_decl",
        "software",
        "systemd",
        "users",
        "web_ui",
        "wireguard",
    ];

    PLUGINS
        .iter()
        .filter_map(|name| schema_for_plugin(name).map(|s| ((*name).to_string(), s)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json::prelude::*;
    use std::collections::HashSet;

    #[test]
    fn test_all_plugins_have_contract_schema() {
        let schemas = all_contract_schemas();
        assert_eq!(schemas.len(), 32);
    }

    #[test]
    fn test_contract_shape_has_required_sections() {
        let schema = schema_for_plugin("net").expect("net schema");
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .expect("required array");

        let required_strings: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();

        for field in [
            "stub",
            "immutable",
            "tunable",
            "observed",
            "meta",
            "semantic_index",
            "privacy_index",
        ] {
            assert!(required_strings.contains(&field));
        }
    }

    #[test]
    fn test_dependency_targets_are_known_plugins() {
        let known: HashSet<String> = all_contract_schemas().keys().cloned().collect();

        for (plugin, schema) in all_contract_schemas() {
            let empty: Vec<Value> = Vec::new();
            let deps = schema
                .get("properties")
                .and_then(|v| v.get("meta"))
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("dependencies"))
                .and_then(|v| v.get("default"))
                .and_then(|v| v.as_array())
                .unwrap_or(&empty);

            for dep in deps.iter().filter_map(|v| v.as_str()) {
                assert!(
                    known.contains(dep),
                    "plugin '{}' has unknown dependency '{}'",
                    plugin,
                    dep
                );
            }
        }
    }

    #[test]
    fn test_uniform_index_paths_use_absolute_json_paths() {
        fn validate_path_array(paths: Option<&Vec<Value>>, context: &str) {
            if let Some(arr) = paths {
                for path in arr.iter().filter_map(|v| v.as_str()) {
                    assert!(
                        path.starts_with('/'),
                        "{} contains non-absolute path '{}'",
                        context,
                        path
                    );
                }
            }
        }

        for (plugin, schema) in all_contract_schemas() {
            let semantic = schema
                .get("properties")
                .and_then(|v| v.get("semantic_index"))
                .and_then(|v| v.get("properties"));

            validate_path_array(
                semantic
                    .and_then(|v| v.get("include_paths"))
                    .and_then(|v| v.get("default"))
                    .and_then(|v| v.as_array()),
                &format!("{}.semantic_index.include_paths", plugin),
            );
            validate_path_array(
                semantic
                    .and_then(|v| v.get("exclude_paths"))
                    .and_then(|v| v.get("default"))
                    .and_then(|v| v.as_array()),
                &format!("{}.semantic_index.exclude_paths", plugin),
            );

            let redaction = schema
                .get("properties")
                .and_then(|v| v.get("privacy_index"))
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("redaction"))
                .and_then(|v| v.get("properties"));

            validate_path_array(
                redaction
                    .and_then(|v| v.get("secret_paths"))
                    .and_then(|v| v.get("default"))
                    .and_then(|v| v.as_array()),
                &format!("{}.privacy_index.redaction.secret_paths", plugin),
            );
            validate_path_array(
                redaction
                    .and_then(|v| v.get("pii_paths"))
                    .and_then(|v| v.get("default"))
                    .and_then(|v| v.as_array()),
                &format!("{}.privacy_index.redaction.pii_paths", plugin),
            );
        }
    }

    #[test]
    fn test_recovery_priority_is_bounded() {
        for (plugin, schema) in all_contract_schemas() {
            let priority = schema
                .get("properties")
                .and_then(|v| v.get("meta"))
                .and_then(|v| v.get("properties"))
                .and_then(|v| v.get("recovery_priority"))
                .and_then(|v| v.get("default"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            assert!(
                priority <= 100,
                "plugin '{}' has out-of-range recovery priority {}",
                plugin,
                priority
            );
        }
    }
}
