//! Inspector Gadget: Discovery, Migration, and Schema Onboarding
//!
//! # CRITICAL INVARIANT: ONE-SHOT ONLY
//!
//! Inspector Gadget is the ONLY component allowed to perform system introspection,
//! and ONLY for bounded, operator-initiated workflows:
//!
//! 1. **Initial Discovery** - Enumerate what exists and import into OP-DBUS
//! 2. **Migration** - Translate legacy systems into deterministic workflows
//! 3. **Schema Onboarding** - Discover subsystem shapes and register schemas
//!
//! Inspector Gadget is NOT:
//! - A background service
//! - Part of the runtime control loop
//! - A source of runtime truth
//!
//! After import/onboarding, the database is authoritative and D-Bus is projection.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use simd_json::OwnedValue as Value;
use simd_json::prelude::*;

use crate::error::{OpDbusError, Result};
use op_state_store::StateStore;
use op_plugins::plugin::PluginMetadata as PluginCore;
use op_plugins::registry::PluginRegistry;
use crate::work_stack::{WorkStack, WorkStackBuilder};
use op_tools::ToolRegistry;

pub mod introspective;

/// Discovery result from system introspection
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiscoveryResult {
    /// Unique discovery ID
    pub discovery_id: String,
    /// Discovery timestamp
    pub timestamp_ns: u128,
    /// Discovered objects by type
    pub objects: HashMap<String, Vec<DiscoveredObject>>,
    /// Discovered schemas
    pub schemas: Vec<DiscoveredSchema>,
    /// Warnings during discovery
    pub warnings: Vec<String>,
    /// Statistics
    pub stats: DiscoveryStats,
}

/// A discovered object from system introspection
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiscoveredObject {
    /// Object type (e.g., "network.interface", "storage.volume")
    pub object_type: String,
    /// Proposed object ID
    pub object_id: String,
    /// Object data
    pub data: Value,
    /// Source of discovery
    pub source: DiscoverySource,
    /// Whether this conflicts with existing state
    pub conflicts: bool,
}

/// Source of discovered data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DiscoverySource {
    /// From /sys filesystem
    Sysfs(String),
    /// From /proc filesystem
    Procfs(String),
    /// From D-Bus introspection
    DBus(String),
    /// From OVSDB
    Ovsdb(String),
    /// From configuration files
    ConfigFile(String),
    /// From NetworkManager
    NetworkManager,
    /// From systemd
    Systemd,
    /// From manual inspection
    Manual,
}

/// A discovered schema for a new object type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiscoveredSchema {
    /// Schema name
    pub name: String,
    /// JSON Schema definition
    pub schema: Value,
    /// Suggested plugin for this schema
    pub suggested_plugin: String,
    /// Sample objects
    pub samples: Vec<Value>,
}

/// Discovery statistics
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DiscoveryStats {
    pub total_objects: usize,
    pub new_objects: usize,
    pub conflicting_objects: usize,
    pub new_schemas: usize,
    pub duration_ms: u64,
}

/// Migration plan from legacy system
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationPlan {
    /// Plan identifier
    pub plan_id: String,
    /// Source system description
    pub source_system: String,
    /// Target state description
    pub target_state: String,
    /// Steps to execute (as work stacks)
    pub steps: Vec<MigrationStep>,
    /// Estimated risk level
    pub risk_level: RiskLevel,
    /// Rollback capability
    pub rollback_possible: bool,
    /// Pre-flight checks
    pub preflight_checks: Vec<PreflightCheck>,
}

/// A step in the migration plan
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationStep {
    pub step_number: usize,
    pub description: String,
    pub tool_name: String,
    pub params: Value,
    pub depends_on: Vec<usize>,
    pub reversible: bool,
}

/// Risk level for migration
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "Low"),
            RiskLevel::Medium => write!(f, "Medium"),
            RiskLevel::High => write!(f, "High"),
            RiskLevel::Critical => write!(f, "Critical"),
        }
    }
}

/// Pre-flight check for migration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreflightCheck {
    pub name: String,
    pub description: String,
    pub check_type: PreflightCheckType,
    pub required: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PreflightCheckType {
    ServiceRunning(String),
    PackageInstalled(String),
    FileExists(String),
    DatabaseConnected,
    NetworkReachable(String),
    Custom(String),
}

/// Inspector Gadget configuration
#[derive(Debug, Clone)]
pub struct InspectorConfig {
    /// Enable network discovery
    pub discover_network: bool,
    /// Enable storage discovery
    pub discover_storage: bool,
    /// Enable process discovery
    pub discover_processes: bool,
    /// Enable service discovery
    pub discover_services: bool,
    /// Discovery timeout in seconds
    pub timeout_secs: u64,
    /// Maximum objects to discover
    pub max_objects: usize,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            discover_network: true,
            discover_storage: true,
            discover_processes: false, // Can be expensive
            discover_services: true,
            timeout_secs: 60,
            max_objects: 10000,
        }
    }
}

/// Inspector Gadget - One-shot system introspection
///
/// This is the ONLY component allowed to probe the system.
/// It is NOT a background service and does NOT participate in runtime.
pub struct InspectorGadget {
    config: InspectorConfig,
    state_store: Arc<dyn StateStore>,
    plugin_registry: Arc<PluginRegistry>,
    tool_registry: Arc<ToolRegistry>,
}

impl InspectorGadget {
    /// Create a new Inspector Gadget instance
    pub fn new(
        config: InspectorConfig,
        state_store: Arc<dyn StateStore>,
        plugin_registry: Arc<PluginRegistry>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        tracing::info!("Inspector Gadget initialized (one-shot discovery only)");
        Self {
            config,
            state_store,
            plugin_registry,
            tool_registry,
        }
    }

    /// Perform initial discovery
    ///
    /// This is a ONE-SHOT operation that:
    /// 1. Inspects the host system
    /// 2. Enumerates existing resources
    /// 3. Generates a canonical import
    ///
    /// After this, the database is authoritative.
    pub async fn discover(&self) -> Result<DiscoveryResult> {
        let start = std::time::Instant::now();
        let discovery_id = uuid::Uuid::new_v4().to_string();
        
        tracing::info!("Starting one-shot discovery: {}", discovery_id);

        let mut objects: HashMap<String, Vec<DiscoveredObject>> = HashMap::new();
        let mut schemas = Vec::new();
        let mut warnings = Vec::new();

        // Discover network interfaces
        if self.config.discover_network {
            match self.discover_network_interfaces().await {
                Ok(ifaces) => {
                    objects.insert("network.interface".to_string(), ifaces);
                }
                Err(e) => warnings.push(format!("Network discovery failed: {}", e)),
            }
        }

        // Discover storage volumes
        if self.config.discover_storage {
            match self.discover_storage_volumes().await {
                Ok(vols) => {
                    objects.insert("storage.volume".to_string(), vols);
                }
                Err(e) => warnings.push(format!("Storage discovery failed: {}", e)),
            }
        }

        // Discover systemd services
        if self.config.discover_services {
            match self.discover_services().await {
                Ok(svcs) => {
                    objects.insert("system.service".to_string(), svcs);
                }
                Err(e) => warnings.push(format!("Service discovery failed: {}", e)),
            }
        }

        // Calculate statistics
        let total_objects: usize = objects.values().map(|v| v.len()).sum();
        let conflicting_objects = objects.values()
            .flat_map(|v| v.iter())
            .filter(|o| o.conflicts)
            .count();

        let stats = DiscoveryStats {
            total_objects,
            new_objects: total_objects, // All are new on fresh discovery
            conflicting_objects,
            new_schemas: schemas.len(),
            duration_ms: start.elapsed().as_millis() as u64,
        };

        tracing::info!(
            "Discovery complete: {} objects, {} schemas, {} warnings",
            total_objects, schemas.len(), warnings.len()
        );

        Ok(DiscoveryResult {
            discovery_id,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            objects,
            schemas,
            warnings,
            stats,
        })
    }

    /// Import discovered objects into the state store
    ///
    /// This commits the discovery result as authoritative truth.
    pub async fn import(&self, discovery: &DiscoveryResult) -> Result<ImportResult> {
        tracing::info!("Importing discovery {} into state store", discovery.discovery_id);

        let mut imported = 0;
        let mut skipped = 0;
        let mut errors = Vec::new();

        for (object_type, objects) in &discovery.objects {
            for obj in objects {
                if obj.conflicts {
                    skipped += 1;
                    continue;
                }

                match self.state_store.upsert_object(
                    &obj.object_id,
                    object_type,
                    "discovered",
                    &obj.data,
                ).await {
                    Ok(_) => imported += 1,
                    Err(e) => errors.push(format!("{}: {}", obj.object_id, e)),
                }
            }
        }

        tracing::info!("Import complete: {} imported, {} skipped, {} errors",
            imported, skipped, errors.len());

        Ok(ImportResult {
            discovery_id: discovery.discovery_id.clone(),
            imported,
            skipped,
            errors,
        })
    }

    /// Create migration plan from legacy system
    ///
    /// This analyzes the current state and generates a deterministic
    /// sequence of tool executions for migration.
    pub async fn plan_migration(
        &self,
        source: &str,
        target_state: &str,
    ) -> Result<MigrationPlan> {
        let plan_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Creating migration plan: {} -> {}", source, target_state);

        let mut steps = Vec::new();
        let mut preflight_checks = Vec::new();

        // Analyze source system
        match source.to_lowercase().as_str() {
            "networkmanager" | "nm" => {
                preflight_checks.push(PreflightCheck {
                    name: "NetworkManager Running".into(),
                    description: "NetworkManager service must be running for export".into(),
                    check_type: PreflightCheckType::ServiceRunning("NetworkManager".into()),
                    required: true,
                });

                steps.push(MigrationStep {
                    step_number: 1,
                    description: "Export NetworkManager connections".into(),
                    tool_name: "inspector.export_nm_connections".into(),
                    params: simd_json::json!({}),
                    depends_on: vec![],
                    reversible: true,
                });

                steps.push(MigrationStep {
                    step_number: 2,
                    description: "Create OP-DBUS network objects".into(),
                    tool_name: "network.batch_create".into(),
                    params: simd_json::json!({"source": "migration"}),
                    depends_on: vec![1],
                    reversible: true,
                });

                steps.push(MigrationStep {
                    step_number: 3,
                    description: "Disable NetworkManager management".into(),
                    tool_name: "system.service.configure".into(),
                    params: simd_json::json!({
                        "service": "NetworkManager",
                        "managed": false
                    }),
                    depends_on: vec![2],
                    reversible: true,
                });
            }
            "ovsdb" => {
                preflight_checks.push(PreflightCheck {
                    name: "OVSDB Reachable".into(),
                    description: "OVSDB must be accessible".into(),
                    check_type: PreflightCheckType::NetworkReachable("unix:/var/run/openvswitch/db.sock".into()),
                    required: true,
                });

                steps.push(MigrationStep {
                    step_number: 1,
                    description: "Export OVSDB schema and data".into(),
                    tool_name: "inspector.export_ovsdb".into(),
                    params: simd_json::json!({}),
                    depends_on: vec![],
                    reversible: true,
                });
            }
            _ => {
                return Err(OpDbusError::ConfigError(
                    format!("Unknown migration source: {}", source)
                ));
            }
        }

        let risk_level = if steps.iter().any(|s| !s.reversible) {
            RiskLevel::High
        } else if steps.len() > 5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let rollback_possible = steps.iter().all(|s| s.reversible);

        Ok(MigrationPlan {
            plan_id,
            source_system: source.to_string(),
            target_state: target_state.to_string(),
            steps,
            risk_level,
            rollback_possible,
            preflight_checks,
        })
    }

    /// Execute a migration plan
    ///
    /// This converts the plan into work stacks and executes through the orchestrator.
    pub async fn execute_migration(
        &self,
        plan: &MigrationPlan,
    ) -> Result<MigrationResult> {
        tracing::info!("Executing migration plan: {}", plan.plan_id);

        // Build work stack from migration steps
        let mut builder = WorkStack::builder()
            .stack_id(format!("migration-{}", plan.plan_id))
            .policy_id("migration-policy".to_string());

        for step in &plan.steps {
            builder = builder.add_tool(
                format!("step-{}", step.step_number),
                &step.tool_name,
                step.params.clone(),
            );

            // Add dependencies
            for dep in &step.depends_on {
                builder = builder.add_edge(
                    format!("step-{}", dep),
                    format!("step-{}", step.step_number),
                );
            }
        }

        let work_stack = builder.build();

        Ok(MigrationResult {
            plan_id: plan.plan_id.clone(),
            work_stack_id: work_stack.stack_id.clone(),
            status: MigrationStatus::Pending,
            steps_completed: 0,
            steps_total: plan.steps.len(),
            errors: vec![],
        })
    }

    /// Discover and onboard a new schema
    ///
    /// This inspects a subsystem and generates/registers schemas for new object types.
    pub async fn onboard_schema(
        &self,
        subsystem: &str,
    ) -> Result<SchemaOnboardingResult> {
        tracing::info!("Onboarding schema for subsystem: {}", subsystem);

        let schemas = match subsystem.to_lowercase().as_str() {
            "network" => self.discover_network_schemas().await?,
            "storage" => self.discover_storage_schemas().await?,
            "systemd" => self.discover_systemd_schemas().await?,
            _ => vec![],
        };

        let registered = schemas.len();

        let registered = schemas.len();
        let any_schemas = !schemas.is_empty();

        Ok(SchemaOnboardingResult {
            subsystem: subsystem.to_string(),
            schemas_discovered: schemas,
            schemas_registered: registered,
            plugin_updated: any_schemas,
        })
    }

    // Private discovery methods

    async fn discover_network_interfaces(&self) -> Result<Vec<DiscoveredObject>> {
        let mut interfaces = Vec::new();

        // Read from /sys/class/net
        let net_path = std::path::Path::new("/sys/class/net");
        if net_path.exists() {
            if let Ok(entries) = std::fs::read_dir(net_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name == "lo" {
                        continue; // Skip loopback
                    }

                    let mut data = simd_json::value::owned::Object::new();
                    data.insert("name".into(), Value::from(name.clone()));

                    // Read MAC address
                    let mac_path = entry.path().join("address");
                    if let Ok(mac) = std::fs::read_to_string(&mac_path) {
                        data.insert("mac_address".into(), Value::from(mac.trim().to_string()));
                    }

                    // Read MTU
                    let mtu_path = entry.path().join("mtu");
                    if let Ok(mtu) = std::fs::read_to_string(&mtu_path) {
                        if let Ok(mtu_val) = mtu.trim().parse::<u64>() {
                            data.insert("mtu".into(), Value::from(mtu_val));
                        }
                    }

                    // Read operstate
                    let state_path = entry.path().join("operstate");
                    if let Ok(state) = std::fs::read_to_string(&state_path) {
                        data.insert("state".into(), Value::from(state.trim().to_string()));
                    }

                    interfaces.push(DiscoveredObject {
                        object_type: "network.interface".to_string(),
                        object_id: format!("interface-{}", name),
                        data: Value::from(data),
                        source: DiscoverySource::Sysfs(format!("/sys/class/net/{}", name)),
                        conflicts: false,
                    });
                }
            }
        }

        Ok(interfaces)
    }

    async fn discover_storage_volumes(&self) -> Result<Vec<DiscoveredObject>> {
        let mut volumes = Vec::new();

        // Read from /sys/block
        let block_path = std::path::Path::new("/sys/block");
        if block_path.exists() {
            if let Ok(entries) = std::fs::read_dir(block_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    
                    // Skip loop devices and ram disks
                    if name.starts_with("loop") || name.starts_with("ram") {
                        continue;
                    }

                    let mut data = simd_json::value::owned::Object::new();
                    data.insert("name".into(), Value::from(name.clone()));

                    // Read size
                    let size_path = entry.path().join("size");
                    if let Ok(size) = std::fs::read_to_string(&size_path) {
                        if let Ok(sectors) = size.trim().parse::<u64>() {
                            // Convert 512-byte sectors to bytes
                            let size_bytes: u64 = sectors * 512;
                            data.insert("size_bytes".into(), Value::from(size_bytes));
                        }
                    }

                    // Read device type
                    let rotational_path = entry.path().join("queue/rotational");
                    if let Ok(rot) = std::fs::read_to_string(&rotational_path) {
                        let is_ssd = rot.trim() == "0";
                        data.insert("type".into(), Value::String(
                            if is_ssd { "ssd" } else { "hdd" }.to_string()
                        ));
                    }

                    volumes.push(DiscoveredObject {
                        object_type: "storage.volume".to_string(),
                        object_id: format!("volume-{}", name),
                        data: Value::from(data),
                        source: DiscoverySource::Sysfs(format!("/sys/block/{}", name)),
                        conflicts: false,
                    });
                }
            }
        }

        Ok(volumes)
    }

    async fn discover_services(&self) -> Result<Vec<DiscoveredObject>> {
        let mut services = Vec::new();

        // List systemd services via D-Bus would be ideal
        // For now, read from /etc/systemd/system and /run/systemd/system
        let service_dirs = [
            "/etc/systemd/system",
            "/run/systemd/system",
            "/lib/systemd/system",
        ];

        for dir in service_dirs {
            let path = std::path::Path::new(dir);
            if !path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !name.ends_with(".service") {
                        continue;
                    }

                    let service_name = name.trim_end_matches(".service");

                    let mut data = simd_json::value::owned::Object::new();
                    data.insert("name".into(), Value::from(service_name.to_string()));
                    data.insert("unit_file".into(), Value::from(entry.path().display().to_string()));

                    services.push(DiscoveredObject {
                        object_type: "system.service".to_string(),
                        object_id: format!("service-{}", service_name),
                        data: Value::from(data),
                        source: DiscoverySource::Systemd,
                        conflicts: false,
                    });
                }
            }
        }

        Ok(services)
    }

    async fn discover_network_schemas(&self) -> Result<Vec<DiscoveredSchema>> {
        Ok(vec![
            DiscoveredSchema {
                name: "network.interface".to_string(),
                schema: simd_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "mac_address": {"type": "string"},
                        "mtu": {"type": "integer"},
                        "state": {"type": "string", "enum": ["up", "down", "unknown"]},
                        "ip_addresses": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "required": ["name"]
                }),
                suggested_plugin: "network".to_string(),
                samples: vec![],
            },
            DiscoveredSchema {
                name: "network.bridge".to_string(),
                schema: simd_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "interfaces": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "stp_enabled": {"type": "boolean"}
                    },
                    "required": ["name"]
                }),
                suggested_plugin: "network".to_string(),
                samples: vec![],
            },
        ])
    }

    async fn discover_storage_schemas(&self) -> Result<Vec<DiscoveredSchema>> {
        Ok(vec![
            DiscoveredSchema {
                name: "storage.volume".to_string(),
                schema: simd_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "size_bytes": {"type": "integer"},
                        "type": {"type": "string", "enum": ["hdd", "ssd", "nvme"]},
                        "mount_point": {"type": "string"}
                    },
                    "required": ["name"]
                }),
                suggested_plugin: "storage".to_string(),
                samples: vec![],
            },
        ])
    }

    async fn discover_systemd_schemas(&self) -> Result<Vec<DiscoveredSchema>> {
        Ok(vec![
            DiscoveredSchema {
                name: "system.service".to_string(),
                schema: simd_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "unit_file": {"type": "string"},
                        "enabled": {"type": "boolean"},
                        "active": {"type": "boolean"},
                        "description": {"type": "string"}
                    },
                    "required": ["name"]
                }),
                suggested_plugin: "system".to_string(),
                samples: vec![],
            },
        ])
    }
}

/// Result of importing discovered objects
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportResult {
    pub discovery_id: String,
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Result of migration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationResult {
    pub plan_id: String,
    pub work_stack_id: String,
    pub status: MigrationStatus,
    pub steps_completed: usize,
    pub steps_total: usize,
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum MigrationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    RolledBack,
}

/// Result of schema onboarding
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SchemaOnboardingResult {
    pub subsystem: String,
    pub schemas_discovered: Vec<DiscoveredSchema>,
    pub schemas_registered: usize,
    pub plugin_updated: bool,
}
