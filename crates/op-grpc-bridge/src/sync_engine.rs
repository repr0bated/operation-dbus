//! Sync Engine - Coordinates bidirectional state synchronization
//!
//! The sync engine is the central coordinator that:
//! - Routes D-Bus changes to gRPC subscribers
//! - Routes gRPC mutations to D-Bus
//! - Ensures all changes go through the event chain
//! - Maintains subscriber state

use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, info, warn};
use anyhow;
use simd_json::prelude::{ValueAsContainer, ValueAsScalar};
use zbus::{Connection, Proxy};
use zbus::zvariant::{Array as ZArray, OwnedValue as ZOwnedValue, Str as ZStr, Value as ZValue};

use op_state_store::{
    ChainEvent, Decision, EventChain, OperationType,
};

/// A state change that can be synced bidirectionally
#[derive(Debug, Clone)]
pub struct StateChange {
    pub change_id: String,
    pub event_id: u64,
    pub plugin_id: String,
    pub object_path: String,
    pub change_type: ChangeType,
    pub member_name: Option<String>,
    pub old_value: Option<simd_json::OwnedValue>,
    pub new_value: simd_json::OwnedValue,
    pub tags_touched: Vec<String>,
    pub event_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub actor_id: String,
    pub source: ChangeSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    PropertySet,
    PropertyDelete,
    MethodCall,
    Signal,
    ObjectAdded,
    ObjectRemoved,
    SchemaMigration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    DBus,
    Grpc,
    Internal,
}

/// Subscription filter for state changes
#[derive(Debug, Clone, Default)]
pub struct SubscriptionFilter {
    pub plugin_ids: Vec<String>,
    pub path_patterns: Vec<String>,
    pub tags: Vec<String>,
}

impl SubscriptionFilter {
    pub fn matches(&self, change: &StateChange) -> bool {
        // Empty filter = match all
        if self.plugin_ids.is_empty() && self.path_patterns.is_empty() && self.tags.is_empty() {
            return true;
        }

        // Check plugin ID
        if !self.plugin_ids.is_empty() && !self.plugin_ids.contains(&change.plugin_id) {
            return false;
        }

        // Check path patterns (simple glob matching)
        if !self.path_patterns.is_empty() {
            let path_matches = self.path_patterns.iter().any(|pattern| {
                if pattern.contains('*') {
                    // Simple glob: * matches any segment
                    let pattern_parts: Vec<&str> = pattern.split('*').collect();
                    if pattern_parts.len() == 2 {
                        change.object_path.starts_with(pattern_parts[0])
                            && change.object_path.ends_with(pattern_parts[1])
                    } else {
                        change.object_path == *pattern
                    }
                } else {
                    change.object_path == *pattern
                }
            });
            if !path_matches {
                return false;
            }
        }

        // Check tags
        if !self.tags.is_empty() {
            let tag_matches = self.tags.iter().any(|tag| change.tags_touched.contains(tag));
            if !tag_matches {
                return false;
            }
        }

        true
    }
}

/// The sync engine coordinates all state synchronization
pub struct SyncEngine {
    /// Event chain for audit trail
    event_chain: Arc<RwLock<EventChain>>,
    /// Broadcast channel for state changes
    change_tx: broadcast::Sender<StateChange>,
    /// Active subscriptions by subscriber ID
    subscriptions: Arc<RwLock<HashMap<String, SubscriptionFilter>>>,
    /// Plugin state cache
    state_cache: Arc<RwLock<HashMap<String, simd_json::OwnedValue>>>,
}

impl SyncEngine {
    /// Create a new sync engine
    pub fn new(event_chain: Arc<RwLock<EventChain>>) -> Self {
        let (change_tx, _) = broadcast::channel(1024);

        Self {
            event_chain,
            change_tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            state_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to state changes with optional filter
    pub async fn subscribe(
        &self,
        subscriber_id: String,
        filter: SubscriptionFilter,
    ) -> broadcast::Receiver<StateChange> {
        let mut subs = self.subscriptions.write().await;
        subs.insert(subscriber_id.clone(), filter);
        debug!("Added subscription: {}", subscriber_id);
        self.change_tx.subscribe()
    }

    /// Unsubscribe from state changes
    pub async fn unsubscribe(&self, subscriber_id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(subscriber_id);
        debug!("Removed subscription: {}", subscriber_id);
    }

    /// Process a change from D-Bus and propagate to gRPC subscribers
    pub async fn process_dbus_change(
        &self,
        plugin_id: String,
        object_path: String,
        change_type: ChangeType,
        member_name: Option<String>,
        old_value: Option<simd_json::OwnedValue>,
        new_value: simd_json::OwnedValue,
        tags: Vec<String>,
        actor_id: String,
    ) -> Result<StateChange, SyncError> {
        // Record in event chain
        let event = {
            let mut chain = self.event_chain.write().await;
            let event = chain.record(
                actor_id.clone(),
                plugin_id.clone(),
                "1.0.0".to_string(), // TODO: get actual schema version
                change_type_to_operation(change_type),
                object_path.clone(),
                tags.clone(),
                Decision::Allow,
                &new_value,
            );
            event.clone()
        };

        // Create state change
        let change = StateChange {
            change_id: uuid::Uuid::new_v4().to_string(),
            event_id: event.event_id,
            plugin_id,
            object_path,
            change_type,
            member_name,
            old_value,
            new_value,
            tags_touched: tags,
            event_hash: event.event_hash.clone(),
            timestamp: event.timestamp,
            actor_id,
            source: ChangeSource::DBus,
        };

        // Broadcast to subscribers
        if let Err(e) = self.change_tx.send(change.clone()) {
            warn!("No active subscribers for change: {}", e);
        }

        info!(
            "Processed D-Bus change: event_id={}, path={}",
            change.event_id, change.object_path
        );

        Ok(change)
    }

    /// Process a mutation request from gRPC
    pub async fn process_grpc_mutation(
        &self,
        plugin_id: String,
        object_path: String,
        change_type: ChangeType,
        member_name: Option<String>,
        value: simd_json::OwnedValue,
        actor_id: String,
        capability_id: Option<String>,
    ) -> Result<MutationResult, SyncError> {
        // TODO: Check capabilities/permissions here
        // TODO: Validate against schema

        // Record in event chain
        let event = {
            let mut chain = self.event_chain.write().await;

            // For now, always allow (real impl would check immutability, capabilities, etc.)
            let mut event = ChainEvent::new(
                chain.next_event_id(),
                chain.last_hash().to_string(),
                actor_id.clone(),
                plugin_id.clone(),
                "1.0.0".to_string(),
                change_type_to_operation(change_type),
                object_path.clone(),
                vec![], // TODO: compute tags from schema
                Decision::Allow,
                &value,
            );

            if let Some(cap) = capability_id {
                event = event.with_capability(cap);
            }

            chain.append(event.clone());
            event
        };

        // Create state change for broadcasting
        let change = StateChange {
            change_id: uuid::Uuid::new_v4().to_string(),
            event_id: event.event_id,
            plugin_id: plugin_id.clone(),
            object_path: object_path.clone(),
            change_type,
            member_name,
            old_value: None, // TODO: get from cache
            new_value: value.clone(),
            tags_touched: vec![],
            event_hash: event.event_hash.clone(),
            timestamp: event.timestamp,
            actor_id,
            source: ChangeSource::Grpc,
        };

        // Broadcast to subscribers (including D-Bus watcher for propagation)
        if let Err(e) = self.change_tx.send(change.clone()) {
            warn!("No active subscribers for change: {}", e);
        }

        info!(
            "Processed gRPC mutation: event_id={}, path={}",
            event.event_id, object_path
        );

        Ok(MutationResult {
            success: true,
            event_id: event.event_id,
            event_hash: event.event_hash,
            result: Some(value),
            error: None,
        })
    }

    /// Get the broadcast sender for new changes
    pub fn change_sender(&self) -> broadcast::Sender<StateChange> {
        self.change_tx.clone()
    }

    /// Get current state for a plugin
    pub async fn get_state(&self, plugin_id: &str) -> Option<simd_json::OwnedValue> {
        let cache = self.state_cache.read().await;
        cache.get(plugin_id).cloned()
    }

    /// Update state cache
    pub async fn update_state_cache(&self, plugin_id: String, state: simd_json::OwnedValue) {
        let mut cache = self.state_cache.write().await;
        cache.insert(plugin_id, state);
    }

    /// Get the event chain (for queries)
    pub fn event_chain(&self) -> Arc<RwLock<EventChain>> {
        self.event_chain.clone()
    }

    /// Call a D-Bus method directly (shared-server path for gRPC).
    pub async fn call_dbus_method(
        &self,
        plugin_id: &str,
        object_path: &str,
        interface_name: &str,
        method_name: &str,
        args: Vec<simd_json::OwnedValue>,
        _actor_id: &str,
        _capability_id: &Option<String>,
    ) -> Result<simd_json::OwnedValue, SyncError> {
        let connection = Connection::system()
            .await
            .map_err(|e| SyncError::DBus(format!("System bus error: {}", e)))?;

        let proxy = Proxy::new(&connection, plugin_id, object_path, interface_name)
            .await
            .map_err(|e| SyncError::DBus(format!("Proxy error: {}", e)))?;

        let zbus_args: Vec<ZOwnedValue> = args
            .iter()
            .map(simd_json_to_zvariant)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| SyncError::Validation(format!("Argument conversion error: {}", e)))?;

        let result: ZOwnedValue = proxy
            .call(method_name, &zbus_args)
            .await
            .map_err(|e| SyncError::DBus(format!("Method call error: {}", e)))?;

        simd_json::serde::to_owned_value(&result)
            .map_err(|e| SyncError::DBus(format!("Result serialization error: {}", e)))
    }
}

fn simd_json_to_zvariant(value: &simd_json::OwnedValue) -> Result<ZOwnedValue, anyhow::Error> {
    if let Some(obj) = value.as_object() {
        if let (Some(sig_val), Some(inner)) = (obj.get("sig"), obj.get("value")) {
            if let Some(sig) = sig_val.as_str() {
                return zvariant_from_sig(sig, inner);
            }
        }
    }

    if let Some(s) = value.as_str() {
        return Ok(ZOwnedValue::from(ZStr::from(s)));
    }
    if let Some(b) = value.as_bool() {
        return Ok(ZOwnedValue::from(b));
    }
    if let Some(i) = value.as_i64() {
        return Ok(ZOwnedValue::from(i));
    }
    if let Some(u) = value.as_u64() {
        return Ok(ZOwnedValue::from(u));
    }
    if let Some(f) = value.as_f64() {
        return Ok(ZOwnedValue::from(f));
    }

    Err(anyhow::anyhow!("Unsupported argument type; use tagged {{sig,value}} or primitives"))
}

fn zvariant_from_sig(sig: &str, value: &simd_json::OwnedValue) -> Result<ZOwnedValue, anyhow::Error> {
    match sig {
        "s" => value
            .as_str()
            .map(|v| ZOwnedValue::from(ZStr::from(v)))
            .ok_or_else(|| anyhow::anyhow!("Expected string for sig 's'")),
        "b" => value
            .as_bool()
            .map(ZOwnedValue::from)
            .ok_or_else(|| anyhow::anyhow!("Expected bool for sig 'b'")),
        "i" => value
            .as_i64()
            .map(|v| ZOwnedValue::from(v as i32))
            .ok_or_else(|| anyhow::anyhow!("Expected i32 for sig 'i'")),
        "u" => value
            .as_u64()
            .map(|v| ZOwnedValue::from(v as u32))
            .ok_or_else(|| anyhow::anyhow!("Expected u32 for sig 'u'")),
        "x" => value
            .as_i64()
            .map(ZOwnedValue::from)
            .ok_or_else(|| anyhow::anyhow!("Expected i64 for sig 'x'")),
        "t" => value
            .as_u64()
            .map(ZOwnedValue::from)
            .ok_or_else(|| anyhow::anyhow!("Expected u64 for sig 't'")),
        "d" => value
            .as_f64()
            .map(ZOwnedValue::from)
            .ok_or_else(|| anyhow::anyhow!("Expected f64 for sig 'd'")),
        "ay" => {
            let arr = value
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Expected array for sig 'ay'"))?;
            let bytes: Result<Vec<u8>, anyhow::Error> = arr
                .iter()
                .map(|v| v.as_u64().map(|n| n as u8).ok_or_else(|| anyhow::anyhow!("Expected u8 in ay array")))
                .collect();
            ZOwnedValue::try_from(ZValue::Array(ZArray::from(bytes?)))
                .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
        }
        _ if sig.starts_with('a') => {
            let inner = &sig[1..];
            let arr = value
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Expected array for sig '{}'", sig))?;
            match inner {
                "s" => {
                    let items: Result<Vec<String>, anyhow::Error> = arr
                        .iter()
                        .map(|v| v.as_str().map(|s| s.to_string()).ok_or_else(|| anyhow::anyhow!("Expected string in array")))
                        .collect();
                    ZOwnedValue::try_from(ZValue::Array(ZArray::from(items?)))
                        .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
                }
                "i" => {
                    let items: Result<Vec<i32>, anyhow::Error> = arr
                        .iter()
                        .map(|v| v.as_i64().map(|n| n as i32).ok_or_else(|| anyhow::anyhow!("Expected i32 in array")))
                        .collect();
                    ZOwnedValue::try_from(ZValue::Array(ZArray::from(items?)))
                        .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
                }
                "u" => {
                    let items: Result<Vec<u32>, anyhow::Error> = arr
                        .iter()
                        .map(|v| v.as_u64().map(|n| n as u32).ok_or_else(|| anyhow::anyhow!("Expected u32 in array")))
                        .collect();
                    ZOwnedValue::try_from(ZValue::Array(ZArray::from(items?)))
                        .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
                }
                "b" => {
                    let items: Result<Vec<bool>, anyhow::Error> = arr
                        .iter()
                        .map(|v| v.as_bool().ok_or_else(|| anyhow::anyhow!("Expected bool in array")))
                        .collect();
                    ZOwnedValue::try_from(ZValue::Array(ZArray::from(items?)))
                        .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
                }
                "d" => {
                    let items: Result<Vec<f64>, anyhow::Error> = arr
                        .iter()
                        .map(|v| v.as_f64().ok_or_else(|| anyhow::anyhow!("Expected f64 in array")))
                        .collect();
                    ZOwnedValue::try_from(ZValue::Array(ZArray::from(items?)))
                        .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
                }
                _ => Err(anyhow::anyhow!("Unsupported array signature '{}'", sig)),
            }
        }
        _ => Err(anyhow::anyhow!("Unsupported signature '{}'", sig)),
    }
}

/// Result of a mutation operation
#[derive(Debug, Clone)]
pub struct MutationResult {
    pub success: bool,
    pub event_id: u64,
    pub event_hash: String,
    pub result: Option<simd_json::OwnedValue>,
    pub error: Option<MutationError>,
}

/// Error during mutation
#[derive(Debug, Clone)]
pub struct MutationError {
    pub code: ErrorCode,
    pub message: String,
    pub deny_reason: Option<op_state_store::DenyReason>,
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorCode {
    NotFound,
    PermissionDenied,
    ValidationFailed,
    ReadOnly,
    TagLocked,
    Internal,
}

/// Errors that can occur in sync engine
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("Event chain error: {0}")]
    EventChain(String),
    #[error("D-Bus error: {0}")]
    DBus(String),
    #[error("gRPC error: {0}")]
    Grpc(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

fn change_type_to_operation(change_type: ChangeType) -> OperationType {
    match change_type {
        ChangeType::PropertySet => OperationType::PropertySet,
        ChangeType::PropertyDelete => OperationType::PropertySet,
        ChangeType::MethodCall => OperationType::MethodCall,
        ChangeType::Signal => OperationType::EmitSignal,
        ChangeType::ObjectAdded => OperationType::ApplyTunablePatch,
        ChangeType::ObjectRemoved => OperationType::ApplyTunablePatch,
        ChangeType::SchemaMigration => OperationType::Migrate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_state_store::ChainConfig;

    #[tokio::test]
    async fn test_subscription_filter() {
        let filter = SubscriptionFilter {
            plugin_ids: vec!["lxc".to_string()],
            path_patterns: vec![],
            tags: vec![],
        };

        let change = StateChange {
            change_id: "1".to_string(),
            event_id: 1,
            plugin_id: "lxc".to_string(),
            object_path: "/org/operation/lxc/100".to_string(),
            change_type: ChangeType::PropertySet,
            member_name: Some("running".to_string()),
            old_value: None,
            new_value: simd_json::json!(true),
            tags_touched: vec!["container".to_string()],
            event_hash: "abc".to_string(),
            timestamp: chrono::Utc::now(),
            actor_id: "user1".to_string(),
            source: ChangeSource::DBus,
        };

        assert!(filter.matches(&change));

        let filter2 = SubscriptionFilter {
            plugin_ids: vec!["net".to_string()],
            path_patterns: vec![],
            tags: vec![],
        };

        assert!(!filter2.matches(&change));
    }

    #[tokio::test]
    async fn test_sync_engine_dbus_change() {
        let chain = Arc::new(RwLock::new(EventChain::new(ChainConfig::default())));
        let engine = SyncEngine::new(chain);

        let change = engine.process_dbus_change(
            "lxc".to_string(),
            "/org/operation/lxc/100".to_string(),
            ChangeType::PropertySet,
            Some("running".to_string()),
            Some(simd_json::json!(false)),
            simd_json::json!(true),
            vec!["container".to_string()],
            "user1".to_string(),
        ).await.unwrap();

        assert_eq!(change.event_id, 1);
        assert_eq!(change.plugin_id, "lxc");
        assert_eq!(change.source, ChangeSource::DBus);
    }

    #[tokio::test]
    async fn test_sync_engine_grpc_mutation() {
        let chain = Arc::new(RwLock::new(EventChain::new(ChainConfig::default())));
        let engine = SyncEngine::new(chain);

        let result = engine.process_grpc_mutation(
            "lxc".to_string(),
            "/org/operation/lxc/100".to_string(),
            ChangeType::PropertySet,
            Some("memory".to_string()),
            simd_json::json!(1024),
            "grpc-client".to_string(),
            Some("admin".to_string()),
        ).await.unwrap();

        assert!(result.success);
        assert_eq!(result.event_id, 1);
    }
}
