//! D-Bus server for system bus integration

use crate::manager::StateManager;
use crate::plugin::{StateAction, StateDiff};
use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use simd_json::prelude::*;
use std::collections::HashMap;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use zbus::{connection::Builder, interface, Connection};

/// D-Bus interface for the state manager
pub struct StateManagerDBus {
    state_manager: Arc<StateManager>,
}

#[derive(Clone)]
struct ProjectedObject {
    origin_service: String,
    origin_path: String,
}

#[zbus::interface(name = "org.opdbus.ProjectedObject")]
impl ProjectedObject {
    #[zbus(property)]
    async fn origin_service(&self) -> String {
        self.origin_service.clone()
    }

    #[zbus(property)]
    async fn origin_path(&self) -> String {
        self.origin_path.clone()
    }
}

#[zbus::interface(name = "org.opdbus.StateManager")]
impl StateManagerDBus {
    /// Apply state from JSON string
    async fn apply_openflow_state(&self, state_json: String) -> zbus::fdo::Result<String> {
        let mut state_json_mut = state_json;
        match unsafe { simd_json::from_str::<crate::manager::DesiredState>(&mut state_json_mut) } {
            Ok(desired_state) => match self.state_manager.apply_state(desired_state).await {
                Ok(report) => Ok(format!("Applied successfully: {}", report.success)),
                Err(e) => Err(zbus::fdo::Error::Failed(format!("Apply failed: {}", e))),
            },
            Err(e) => Err(zbus::fdo::Error::InvalidArgs(format!(
                "Invalid JSON: {}",
                e
            ))),
        }
    }

    /// Query current state
    async fn query_state(&self) -> zbus::fdo::Result<String> {
        match self.state_manager.query_current_state().await {
            Ok(state) => match simd_json::to_string(&state) {
                Ok(json) => Ok(json),
                Err(e) => Err(zbus::fdo::Error::Failed(format!(
                    "Serialization failed: {}",
                    e
                ))),
            },
            Err(e) => Err(zbus::fdo::Error::Failed(format!("Query failed: {}", e))),
        }
    }

    /// Apply one contract mutation routed from transport adapters.
    /// This is the canonical write ingress for strict flow mode.
    #[zbus(name = "ApplyContractMutation")]
    async fn apply_contract_mutation(&self, mutation_json: String) -> zbus::fdo::Result<String> {
        let mut mutation_json_mut = mutation_json;
        let mutation = unsafe { simd_json::from_str::<simd_json::OwnedValue>(&mut mutation_json_mut) }
            .map_err(|e| zbus::fdo::Error::InvalidArgs(format!("Invalid mutation JSON: {}", e)))?;

        let plugin_id = mutation
            .get("plugin_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs("Missing plugin_id".to_string()))?;
        let value = mutation
            .get("value")
            .cloned()
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs("Missing value".to_string()))?;

        let desired_state = crate::manager::DesiredState {
            version: 1,
            plugins: HashMap::from([(plugin_id.to_string(), value)]),
        };

        match self
            .state_manager
            .apply_state_single_plugin(desired_state, plugin_id)
            .await
        {
            Ok(report) if report.success => Ok("ok".to_string()),
            Ok(_) => Err(zbus::fdo::Error::Failed(
                "apply_state_single_plugin returned success=false".to_string(),
            )),
            Err(e) => Err(zbus::fdo::Error::Failed(format!(
                "apply_state_single_plugin failed: {}",
                e
            ))),
        }
    }

    /// Restore OpenFlow flows from state file (used after OVS restart)
    ///
    /// # Arguments
    /// * `state_file_path` - Optional path to state file (default: /etc/op-dbus/state.json)
    /// * `bridge_name` - Optional bridge filter (empty string = all bridges)
    ///
    /// # Returns
    /// Success message with count of restored flows
    async fn restore_flows(
        &self,
        state_file_path: String,
        bridge_name: String,
    ) -> zbus::fdo::Result<String> {
        use std::path::PathBuf;

        // Handle default state file path
        let state_path = if state_file_path.is_empty() {
            PathBuf::from("/etc/op-dbus/state.json")
        } else {
            PathBuf::from(state_file_path)
        };

        // Check if state file exists
        if !state_path.exists() {
            return Err(zbus::fdo::Error::Failed(format!(
                "State file not found: {}",
                state_path.display()
            )));
        }

        // Load desired state
        let desired_state = match self.state_manager.load_desired_state(&state_path).await {
            Ok(state) => state,
            Err(e) => {
                return Err(zbus::fdo::Error::Failed(format!(
                    "Failed to load state file: {}",
                    e
                )))
            }
        };

        // Check if openflow plugin state exists
        let openflow_state = match desired_state.plugins.get("openflow") {
            Some(state) => state,
            None => {
                return Err(zbus::fdo::Error::Failed(
                    "No 'openflow' plugin configuration in state file".to_string(),
                ))
            }
        };

        // Get the openflow plugin
        let openflow_plugin = match self.state_manager.get_plugin("openflow").await {
            Some(plugin) => plugin,
            None => {
                return Err(zbus::fdo::Error::Failed(
                    "OpenFlow plugin not registered".to_string(),
                ))
            }
        };

        // Query current state
        let current_state = match openflow_plugin.query_current_state().await {
            Ok(state) => state,
            Err(e) => {
                return Err(zbus::fdo::Error::Failed(format!(
                    "Failed to query current state: {}",
                    e
                )))
            }
        };

        // Calculate diff
        let diff = match openflow_plugin
            .calculate_diff(&current_state, openflow_state)
            .await
        {
            Ok(diff) => diff,
            Err(e) => {
                return Err(zbus::fdo::Error::Failed(format!(
                    "Failed to calculate diff: {}",
                    e
                )))
            }
        };

        // Filter for flow-only actions
        let flow_actions: Vec<StateAction> = diff
            .actions
            .iter()
            .filter(|action| match action {
                StateAction::Create { resource, .. } => {
                    resource.contains("flow/") || resource.contains("flows")
                }
                _ => false,
            })
            .cloned()
            .collect();

        if flow_actions.is_empty() {
            return Ok("No flows need to be restored".to_string());
        }

        // Filter by bridge if specified
        let filtered_actions: Vec<StateAction> = if !bridge_name.is_empty() {
            flow_actions
                .into_iter()
                .filter(|action| {
                    if let StateAction::Create { resource, .. } = action {
                        resource.contains(&bridge_name)
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            flow_actions
        };

        if filtered_actions.is_empty() {
            return Ok(format!("No flows to restore for bridge: {}", bridge_name));
        }

        // Create filtered diff
        let flow_count = filtered_actions.len();
        let filtered_diff = StateDiff {
            plugin: diff.plugin.clone(),
            actions: filtered_actions.clone(),
            metadata: diff.metadata.clone(),
        };

        // Apply the restoration
        match openflow_plugin.apply_state(&filtered_diff).await {
            Ok(_) => Ok(format!("Successfully restored {} flows", flow_count)),
            Err(e) => Err(zbus::fdo::Error::Failed(format!(
                "Failed to restore flows: {}",
                e
            ))),
        }
    }
}

/// Start the system bus D-Bus service
pub async fn start_system_bus(state_manager: Arc<StateManager>) -> Result<()> {
    let interface = StateManagerDBus { state_manager };

    let connection = Builder::system()?
        .name("org.opdbus")?
        .serve_at("/org/opdbus/state", interface)?
        .build()
        .await?;

    spawn_projection_task(connection.clone());

    // Keep the connection alive
    std::future::pending::<()>().await;

    Ok(())
}

fn spawn_projection_task(connection: Connection) {
    tokio::spawn(async move {
        let published_paths = Arc::new(RwLock::new(HashSet::<String>::new()));
        let refresh_seconds = std::env::var("OP_DBUS_PROJECTION_REFRESH_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120);

        loop {
            if let Err(e) = refresh_projection(&connection, &published_paths).await {
                log::warn!("D-Bus projection refresh failed: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(refresh_seconds)).await;
        }
    });
}

async fn refresh_projection(
    connection: &Connection,
    published_paths: &Arc<RwLock<HashSet<String>>>,
) -> Result<()> {
    let dbus = zbus::fdo::DBusProxy::new(connection).await?;
    let names = dbus.list_names().await?;

    let max_services = std::env::var("OP_DBUS_PROJECTION_MAX_SERVICES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(512);
    let max_total_objects = std::env::var("OP_DBUS_PROJECTION_MAX_OBJECTS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(20_000);
    let max_nodes_per_service = std::env::var("OP_DBUS_PROJECTION_MAX_NODES_PER_SERVICE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(4_096);

    let mut service_count = 0usize;
    let mut projected_count = 0usize;

    for name in names {
        if service_count >= max_services || projected_count >= max_total_objects {
            break;
        }

        let service = name.to_string();
        if service.starts_with(':') || service == "org.opdbus" {
            continue;
        }

        service_count += 1;
        let paths = discover_service_paths(connection, &service, max_nodes_per_service).await;
        let mut published_guard = published_paths.write().await;

        for origin_path in paths {
            if projected_count >= max_total_objects {
                break;
            }
            let projected_path = map_to_projected_path(&service, &origin_path);
            if !published_guard.insert(projected_path.clone()) {
                continue;
            }

            let projected = ProjectedObject {
                origin_service: service.clone(),
                origin_path: origin_path.clone(),
            };

            if let Err(e) = connection.object_server().at(projected_path.as_str(), projected).await {
                // Keep going even when a specific path fails to register.
                log::debug!(
                    "Failed to publish projected object {} from {}{}: {}",
                    projected_path,
                    service,
                    origin_path,
                    e
                );
                continue;
            }

            projected_count += 1;
        }
    }

    log::info!(
        "D-Bus projection refresh complete: services_scanned={}, projected_objects_total={}",
        service_count,
        projected_count
    );

    Ok(())
}

async fn discover_service_paths(
    connection: &Connection,
    service: &str,
    max_nodes: usize,
) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut discovered = Vec::new();
    let mut object_manager_hits = 0usize;

    for candidate in candidate_object_manager_paths(service) {
        if visited.len() >= max_nodes {
            break;
        }
        let proxy = match zbus::Proxy::new(
            connection,
            service,
            candidate.as_str(),
            "org.freedesktop.DBus.ObjectManager",
        )
        .await
        {
            Ok(proxy) => proxy,
            Err(_) => continue,
        };

        type ManagedMap = std::collections::HashMap<
            zbus::zvariant::OwnedObjectPath,
            std::collections::HashMap<
                String,
                std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
            >,
        >;

        if let Ok(objects) = proxy.call::<_, _, ManagedMap>("GetManagedObjects", &()).await {
            for object_path in objects.keys() {
                if visited.len() >= max_nodes {
                    break;
                }
                let p = object_path.as_str().to_string();
                if visited.insert(p.clone()) {
                    discovered.push(p);
                    object_manager_hits += 1;
                }
            }
        }
    }

    queue.push_back(("/".to_string(), 0usize));

    while let Some((path, depth)) = queue.pop_front() {
        if visited.len() >= max_nodes || depth > 24 {
            break;
        }
        if !visited.insert(path.clone()) {
            continue;
        }

        discovered.push(path.clone());

        let proxy = match zbus::fdo::IntrospectableProxy::builder(connection)
            .destination(service)
            .and_then(|b| b.path(path.as_str()))
        {
            Ok(builder) => match builder.build().await {
                Ok(p) => p,
                Err(_) => continue,
            },
            Err(_) => continue,
        };

        let xml = match proxy.introspect().await {
            Ok(xml) => xml,
            Err(_) => continue,
        };

        for child in parse_child_nodes(&xml, &path) {
            if !visited.contains(&child) {
                queue.push_back((child, depth + 1));
            }
        }
    }

    if object_manager_hits > 0 {
        log::debug!(
            "ObjectManager discovered {} object paths for service {}",
            object_manager_hits,
            service
        );
    }

    discovered
}

fn parse_child_nodes(xml: &str, parent_path: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut children = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if e.name().as_ref() == b"node" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"name" {
                            let child = String::from_utf8_lossy(&attr.value).to_string();
                            if child.is_empty() {
                                continue;
                            }
                            let full = if child.starts_with('/') {
                                child
                            } else if parent_path == "/" {
                                format!("/{}", child)
                            } else {
                                format!("{}/{}", parent_path, child)
                            };
                            children.push(full);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        buf.clear();
    }

    children
}

fn map_to_projected_path(service: &str, origin_path: &str) -> String {
    let mut sanitized = String::with_capacity(service.len());
    for ch in service.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if origin_path == "/" {
        format!("/org/opdbus/projected/{}/root", sanitized)
    } else {
        format!("/org/opdbus/projected/{}{}", sanitized, origin_path)
    }
}

fn candidate_object_manager_paths(service: &str) -> Vec<String> {
    let mut paths = vec!["/".to_string()];
    let derived = format!("/{}", service.replace('.', "/"));
    if !paths.contains(&derived) {
        paths.push(derived);
    }
    paths
}
