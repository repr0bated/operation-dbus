//! gRPC Server - Implements the Operation gRPC services (shared-server topology)

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use chrono::{DateTime, Utc};
use prost_types::{Struct as ProstStruct, Timestamp as ProstTimestamp, Value as ProstValue};
use simd_json::prelude::{ValueAsContainer, ValueAsScalar};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn};

use crate::proto::{
    event_chain_service_server::EventChainService, plugin_service_server::PluginService,
    state_sync_server::StateSync, BatchMutateRequest, BatchMutateResponse, CallMethodRequest,
    CallMethodResponse, CapabilityMissing as ProtoCapabilityMissing, ChainEvent as ProtoChainEvent,
    ChangeType as ProtoChangeType, ConstraintFail as ProtoConstraintFail, CreateSnapshotRequest,
    CreateSnapshotResponse, Decision as ProtoDecision, DenyReason as ProtoDenyReason,
    ErrorCode as ProtoErrorCode, GetEventsRequest, GetEventsResponse, GetProofRequest,
    GetProofResponse, GetPropertyRequest, GetPropertyResponse, GetSchemaRequest, GetSchemaResponse,
    GetSnapshotRequest, GetSnapshotResponse, GetStateRequest, GetStateResponse, ListPluginsRequest,
    ListPluginsResponse, MerkleProofSibling, MutateRequest, MutateResponse,
    MutationError as ProtoMutationError, OperationType as ProtoOperationType, PluginInfo,
    ProveTagImmutabilityRequest, ProveTagImmutabilityResponse,
    ReadOnlyViolation as ProtoReadOnlyViolation, SetPropertyRequest, SetPropertyResponse, Signal,
    StateChange as ProtoStateChange, SubscribeEventsRequest, SubscribeRequest,
    SubscribeSignalsRequest, TagLock as ProtoTagLock, VerifyChainRequest, VerifyChainResponse,
};
use crate::sync_engine::{ChangeType, SyncEngine};
use op_state_store::{Decision, DenyReason, EventChain, MerkleProof, OperationType};
use zbus::zvariant::{Array as ZArray, OwnedValue as ZOwnedValue, Str as ZStr, Value as ZValue};
use zbus::{Connection, Proxy};

/// Plugin schema provider (source of truth)
pub trait PluginSchemaProvider: Send + Sync {
    fn list_plugins(&self) -> Vec<PluginInfo>;
    fn get_schema(&self, plugin_id: &str) -> Option<(String, String, String)>;
}

struct EmptyPluginProvider;

impl PluginSchemaProvider for EmptyPluginProvider {
    fn list_plugins(&self) -> Vec<PluginInfo> {
        Vec::new()
    }

    fn get_schema(&self, _plugin_id: &str) -> Option<(String, String, String)> {
        None
    }
}

/// gRPC server implementation for operation services
#[derive(Clone)]
pub struct OperationGrpcServer {
    sync_engine: Arc<SyncEngine>,
    plugin_provider: Arc<dyn PluginSchemaProvider>,
    /// Broadcast channel for chain events
    chain_events: broadcast::Sender<ProtoChainEvent>,
}

impl OperationGrpcServer {
    pub fn new(sync_engine: Arc<SyncEngine>) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            sync_engine,
            plugin_provider: Arc::new(EmptyPluginProvider),
            chain_events: tx,
        }
    }

    pub fn with_plugin_provider(
        sync_engine: Arc<SyncEngine>,
        plugin_provider: Arc<dyn PluginSchemaProvider>,
    ) -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            sync_engine,
            plugin_provider,
            chain_events: tx,
        }
    }
}

/// Run gRPC server for StateSync + PluginService + EventChainService
pub async fn run_grpc_server(
    addr: std::net::SocketAddr,
    sync_engine: Arc<SyncEngine>,
    plugin_provider: Option<Arc<dyn PluginSchemaProvider>>,
) -> Result<(), tonic::transport::Error> {
    use crate::proto::event_chain_service_server::EventChainServiceServer;
    use crate::proto::plugin_service_server::PluginServiceServer;
    use crate::proto::state_sync_server::StateSyncServer;

    let server = if let Some(provider) = plugin_provider {
        OperationGrpcServer::with_plugin_provider(sync_engine, provider)
    } else {
        OperationGrpcServer::new(sync_engine)
    };

    tonic::transport::Server::builder()
        .add_service(StateSyncServer::new(server.clone()))
        .add_service(PluginServiceServer::new(server.clone()))
        .add_service(EventChainServiceServer::new(server))
        .serve(addr)
        .await
}

// =============================================================================
// StateSync Service
// =============================================================================

#[tonic::async_trait]
impl StateSync for OperationGrpcServer {
    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<ProtoStateChange, Status>> + Send>>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let req = request.into_inner();
        info!("gRPC Subscribe: plugins={:?}", req.plugin_ids);

        let mut rx = self.sync_engine.change_sender().subscribe();
        let plugin_filters = req.plugin_ids;
        let path_filters = req.path_patterns;
        let tag_filters = req.tags;

        let stream = stream! {
            loop {
                match rx.recv().await {
                    Ok(update) => {
                        let matches_plugin = plugin_filters.is_empty()
                            || plugin_filters.contains(&update.plugin_id);
                        let matches_path = path_filters.is_empty()
                            || path_filters.iter().any(|p| update.object_path.starts_with(p));
                        let matches_tag = tag_filters.is_empty()
                            || update.tags_touched.iter().any(|t| tag_filters.contains(t));

                        if matches_plugin && matches_path && matches_tag {
                            yield Ok(proto_state_change(&update));
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Subscriber lagged, missed {} updates", n);
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn mutate(
        &self,
        request: Request<MutateRequest>,
    ) -> Result<Response<MutateResponse>, Status> {
        let req = request.into_inner();
        let value = prost_value_to_simd(&req.value.unwrap_or_else(|| ProstValue::from(0)));
        let change_type = match req.operation {
            x if x == ProtoOperationType::SetProperty as i32 => ChangeType::PropertySet,
            x if x == ProtoOperationType::CallMethod as i32 => ChangeType::MethodCall,
            x if x == ProtoOperationType::ApplyPatch as i32 => ChangeType::ObjectAdded,
            _ => ChangeType::PropertySet,
        };

        let result = self
            .sync_engine
            .process_grpc_mutation(
                req.plugin_id.clone(),
                req.object_path.clone(),
                change_type,
                if req.member_name.is_empty() {
                    None
                } else {
                    Some(req.member_name.clone())
                },
                value,
                req.actor_id.clone(),
                if req.capability_id.is_empty() {
                    None
                } else {
                    Some(req.capability_id.clone())
                },
            )
            .await;

        match result {
            Ok(ok) => Ok(Response::new(MutateResponse {
                success: ok.success,
                event_id: ok.event_id,
                event_hash: ok.event_hash,
                result: ok.result.map(|v| simd_to_prost_value(&v)),
                error: None,
                effective_hash: String::new(),
            })),
            Err(e) => Ok(Response::new(MutateResponse {
                success: false,
                event_id: 0,
                event_hash: String::new(),
                result: None,
                error: Some(ProtoMutationError {
                    code: ProtoErrorCode::Internal as i32,
                    message: e.to_string(),
                    deny_reason: None,
                }),
                effective_hash: String::new(),
            })),
        }
    }

    async fn get_state(
        &self,
        request: Request<GetStateRequest>,
    ) -> Result<Response<GetStateResponse>, Status> {
        let req = request.into_inner();
        let state = self.sync_engine.get_state(&req.plugin_id).await;

        let state_struct = state
            .map(|v| simd_to_prost_struct(&v))
            .unwrap_or_else(ProstStruct::default);

        Ok(Response::new(GetStateResponse {
            state: Some(state_struct),
            effective_hash: String::new(),
            at_event_id: 0,
        }))
    }

    async fn batch_mutate(
        &self,
        request: Request<BatchMutateRequest>,
    ) -> Result<Response<BatchMutateResponse>, Status> {
        let req = request.into_inner();
        let mut results = Vec::new();
        let mut failed_index = -1;

        for (idx, m) in req.mutations.into_iter().enumerate() {
            let mut_req = Request::new(m);
            let resp = self.mutate(mut_req).await?.into_inner();
            if !resp.success && failed_index < 0 && req.atomic {
                failed_index = idx as i32;
                break;
            }
            results.push(resp);
        }

        Ok(Response::new(BatchMutateResponse {
            success: failed_index < 0,
            results,
            failed_index,
        }))
    }
}

// =============================================================================
// PluginService
// =============================================================================

#[tonic::async_trait]
impl PluginService for OperationGrpcServer {
    type SubscribeSignalsStream = Pin<Box<dyn Stream<Item = Result<Signal, Status>> + Send>>;

    async fn list_plugins(
        &self,
        _request: Request<ListPluginsRequest>,
    ) -> Result<Response<ListPluginsResponse>, Status> {
        Ok(Response::new(ListPluginsResponse {
            plugins: self.plugin_provider.list_plugins(),
        }))
    }

    async fn get_schema(
        &self,
        request: Request<GetSchemaRequest>,
    ) -> Result<Response<GetSchemaResponse>, Status> {
        let req = request.into_inner();
        if let Some((schema_json, dialect, version)) =
            self.plugin_provider.get_schema(&req.plugin_id)
        {
            Ok(Response::new(GetSchemaResponse {
                schema_json,
                dialect,
                version,
            }))
        } else {
            Ok(Response::new(GetSchemaResponse {
                schema_json: String::new(),
                dialect: String::new(),
                version: String::new(),
            }))
        }
    }

    async fn call_method(
        &self,
        request: Request<CallMethodRequest>,
    ) -> Result<Response<CallMethodResponse>, Status> {
        let req = request.into_inner();
        let args: Vec<simd_json::OwnedValue> = req
            .arguments
            .into_iter()
            .map(|v| prost_value_to_simd(&v))
            .collect();

        let result = self
            .sync_engine
            .call_dbus_method(
                &format!("org.opdbus.{}.v1", req.plugin_id),
                &req.object_path,
                &req.interface_name,
                &req.method_name,
                args,
                &req.actor_id,
                &if req.capability_id.is_empty() {
                    None
                } else {
                    Some(req.capability_id.clone())
                },
            )
            .await;

        match result {
            Ok(val) => Ok(Response::new(CallMethodResponse {
                success: true,
                result: Some(simd_to_prost_value(&val)),
                event_id: 0,
                event_hash: String::new(),
                error: None,
            })),
            Err(e) => Ok(Response::new(CallMethodResponse {
                success: false,
                result: None,
                event_id: 0,
                event_hash: String::new(),
                error: Some(ProtoMutationError {
                    code: ProtoErrorCode::Internal as i32,
                    message: e.to_string(),
                    deny_reason: None,
                }),
            })),
        }
    }

    async fn get_property(
        &self,
        request: Request<GetPropertyRequest>,
    ) -> Result<Response<GetPropertyResponse>, Status> {
        let req = request.into_inner();
        let connection = Connection::system()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let proxy = zbus::fdo::PropertiesProxy::builder(&connection)
            .destination(format!("org.opdbus.{}.v1", req.plugin_id))
            .map_err(|e| Status::internal(e.to_string()))?
            .path(req.object_path.as_str())
            .map_err(|e| Status::internal(e.to_string()))?
            .build()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let iface = zbus::names::InterfaceName::try_from(req.interface_name.as_str())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let val: ZOwnedValue = proxy
            .get(iface, req.property_name.as_str())
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let json =
            simd_json::serde::to_owned_value(&val).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetPropertyResponse {
            value: Some(simd_to_prost_value(&json)),
            read_only: false,
        }))
    }

    async fn set_property(
        &self,
        request: Request<SetPropertyRequest>,
    ) -> Result<Response<SetPropertyResponse>, Status> {
        let req = request.into_inner();
        let connection = Connection::system()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        let proxy = zbus::fdo::PropertiesProxy::builder(&connection)
            .destination(format!("org.opdbus.{}.v1", req.plugin_id))
            .map_err(|e| Status::internal(e.to_string()))?
            .path(req.object_path.as_str())
            .map_err(|e| Status::internal(e.to_string()))?
            .build()
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let iface = zbus::names::InterfaceName::try_from(req.interface_name.as_str())
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let value = prost_value_to_simd(&req.value.unwrap_or_else(|| ProstValue::from(0)));
        let zval =
            simd_json_to_zvariant(&value).map_err(|e| Status::invalid_argument(e.to_string()))?;

        proxy
            .set(iface, req.property_name.as_str(), &zval)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(SetPropertyResponse {
            success: true,
            event_id: 0,
            event_hash: String::new(),
            error: None,
        }))
    }

    async fn subscribe_signals(
        &self,
        _request: Request<SubscribeSignalsRequest>,
    ) -> Result<Response<Self::SubscribeSignalsStream>, Status> {
        let stream = tokio_stream::empty::<Result<Signal, Status>>();
        Ok(Response::new(Box::pin(stream)))
    }
}

// =============================================================================
// EventChainService
// =============================================================================

#[tonic::async_trait]
impl EventChainService for OperationGrpcServer {
    type SubscribeEventsStream =
        Pin<Box<dyn Stream<Item = Result<ProtoChainEvent, Status>> + Send>>;

    async fn get_events(
        &self,
        request: Request<GetEventsRequest>,
    ) -> Result<Response<GetEventsResponse>, Status> {
        let req = request.into_inner();
        let chain = self.sync_engine.event_chain();
        let chain = chain.read().await;

        let mut events: Vec<ProtoChainEvent> = chain
            .events()
            .iter()
            .filter(|e| req.from_event_id == 0 || e.event_id >= req.from_event_id)
            .filter(|e| req.to_event_id == 0 || e.event_id <= req.to_event_id)
            .filter(|e| req.plugin_id.is_empty() || e.plugin_id == req.plugin_id)
            .filter(|e| req.tags.is_empty() || e.tags_touched.iter().any(|t| req.tags.contains(t)))
            .filter(|e| match req.decision_filter {
                x if x == ProtoDecision::Allow as i32 => e.decision == Decision::Allow,
                x if x == ProtoDecision::Deny as i32 => e.decision == Decision::Deny,
                _ => true,
            })
            .take(if req.limit == 0 {
                usize::MAX
            } else {
                req.limit as usize
            })
            .map(proto_chain_event)
            .collect();

        let has_more = req.limit > 0 && (events.len() as u32) == req.limit;
        Ok(Response::new(GetEventsResponse { events, has_more }))
    }

    async fn subscribe_events(
        &self,
        request: Request<SubscribeEventsRequest>,
    ) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        let req = request.into_inner();
        let mut rx = self.chain_events.subscribe();
        let plugin_filter = req.plugin_id;
        let tag_filters = req.tags;

        let stream = stream! {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let matches_plugin = plugin_filter.is_empty() || event.plugin_id == plugin_filter;
                        let matches_tag = tag_filters.is_empty() || event.tags_touched.iter().any(|t| tag_filters.contains(t));
                        if matches_plugin && matches_tag {
                            yield Ok(event);
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        };

        Ok(Response::new(Box::pin(stream)))
    }

    async fn verify_chain(
        &self,
        _request: Request<VerifyChainRequest>,
    ) -> Result<Response<VerifyChainResponse>, Status> {
        let chain = self.sync_engine.event_chain();
        let chain = chain.read().await;
        let result = chain.verify_chain();
        Ok(Response::new(VerifyChainResponse {
            valid: result.valid,
            events_verified: result.events_verified as u64,
            batches_verified: result.batches_verified as u64,
            errors: result.errors,
        }))
    }

    async fn get_proof(
        &self,
        request: Request<GetProofRequest>,
    ) -> Result<Response<GetProofResponse>, Status> {
        let req = request.into_inner();
        let chain = self.sync_engine.event_chain();
        let chain = chain.read().await;
        let proof: Option<MerkleProof> =
            op_state_store::EventBatch::generate_proof(chain.events(), req.event_id);

        if let Some(proof) = proof {
            let siblings = proof
                .siblings
                .into_iter()
                .map(|(hash, is_right)| MerkleProofSibling { hash, is_right })
                .collect();
            Ok(Response::new(GetProofResponse {
                event_hash: proof.event_hash,
                siblings,
                root: proof.root,
                batch_first_event_id: 0,
                batch_last_event_id: 0,
            }))
        } else {
            Err(Status::not_found("proof not found"))
        }
    }

    async fn prove_tag_immutability(
        &self,
        request: Request<ProveTagImmutabilityRequest>,
    ) -> Result<Response<ProveTagImmutabilityResponse>, Status> {
        let req = request.into_inner();
        let chain = self.sync_engine.event_chain();
        let chain = chain.read().await;
        let proof = chain.prove_tag_immutability(&req.tag);
        Ok(Response::new(ProveTagImmutabilityResponse {
            tag: proof.tag,
            is_immutable: proof.is_immutable,
            violation_event_ids: proof.violations,
            total_events_checked: proof.total_events_checked as u64,
        }))
    }

    async fn get_snapshot(
        &self,
        request: Request<GetSnapshotRequest>,
    ) -> Result<Response<GetSnapshotResponse>, Status> {
        let req = request.into_inner();
        let chain = self.sync_engine.event_chain();
        let chain = chain.read().await;
        if let Some(snapshot) = chain.get_snapshot(&req.snapshot_id) {
            Ok(Response::new(GetSnapshotResponse {
                snapshot: Some(proto_snapshot(snapshot)),
            }))
        } else {
            Err(Status::not_found("snapshot not found"))
        }
    }

    async fn create_snapshot(
        &self,
        request: Request<CreateSnapshotRequest>,
    ) -> Result<Response<CreateSnapshotResponse>, Status> {
        let req = request.into_inner();
        let state = self
            .sync_engine
            .get_state(&req.plugin_id)
            .await
            .unwrap_or_else(|| simd_json::json!({}));
        let chain = self.sync_engine.event_chain();
        let mut chain = chain.write().await;
        let snapshot = chain.create_snapshot(req.plugin_id, "1.0.0".to_string(), state);
        Ok(Response::new(CreateSnapshotResponse {
            snapshot: Some(proto_snapshot(snapshot)),
        }))
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn proto_state_change(change: &crate::sync_engine::StateChange) -> ProtoStateChange {
    ProtoStateChange {
        change_id: change.change_id.clone(),
        event_id: change.event_id,
        plugin_id: change.plugin_id.clone(),
        object_path: change.object_path.clone(),
        change_type: proto_change_type(change.change_type) as i32,
        member_name: change.member_name.clone().unwrap_or_default(),
        old_value: change.old_value.as_ref().map(simd_to_prost_value),
        new_value: Some(simd_to_prost_value(&change.new_value)),
        tags_touched: change.tags_touched.clone(),
        event_hash: change.event_hash.clone(),
        timestamp: Some(proto_timestamp(change.timestamp)),
        actor_id: change.actor_id.clone(),
    }
}

fn proto_change_type(change_type: ChangeType) -> ProtoChangeType {
    match change_type {
        ChangeType::PropertySet => ProtoChangeType::PropertySet,
        ChangeType::PropertyDelete => ProtoChangeType::PropertyDelete,
        ChangeType::MethodCall => ProtoChangeType::MethodCall,
        ChangeType::Signal => ProtoChangeType::Signal,
        ChangeType::ObjectAdded => ProtoChangeType::ObjectAdded,
        ChangeType::ObjectRemoved => ProtoChangeType::ObjectRemoved,
        ChangeType::SchemaMigration => ProtoChangeType::SchemaMigration,
    }
}

fn proto_chain_event(event: &op_state_store::ChainEvent) -> ProtoChainEvent {
    ProtoChainEvent {
        event_id: event.event_id,
        prev_hash: event.prev_hash.clone(),
        event_hash: event.event_hash.clone(),
        timestamp: Some(proto_timestamp(event.timestamp)),
        actor_id: event.actor_id.clone(),
        capability_id: event.capability_id.clone().unwrap_or_default(),
        plugin_id: event.plugin_id.clone(),
        schema_version: event.schema_version.clone(),
        operation_type: format!("{:?}", event.op),
        target: event.target.clone(),
        tags_touched: event.tags_touched.clone(),
        decision: match event.decision {
            Decision::Allow => ProtoDecision::Allow as i32,
            Decision::Deny => ProtoDecision::Deny as i32,
        },
        deny_reason: event.deny_reason.as_ref().map(proto_deny_reason),
        input_patch_hash: event.input_patch_hash.clone(),
        result_effective_hash: event.result_effective_hash.clone().unwrap_or_default(),
    }
}

fn proto_deny_reason(reason: &DenyReason) -> ProtoDenyReason {
    match reason {
        DenyReason::TagLock { tag, wrapper_id } => ProtoDenyReason {
            reason: Some(crate::proto::deny_reason::Reason::TagLock(ProtoTagLock {
                tag: tag.clone(),
                wrapper_id: wrapper_id.clone(),
            })),
        },
        DenyReason::ConstraintFail {
            constraint,
            message,
        } => ProtoDenyReason {
            reason: Some(crate::proto::deny_reason::Reason::ConstraintFail(
                ProtoConstraintFail {
                    constraint: constraint.clone(),
                    message: message.clone(),
                },
            )),
        },
        DenyReason::CapabilityMissing { capability } => ProtoDenyReason {
            reason: Some(crate::proto::deny_reason::Reason::CapabilityMissing(
                ProtoCapabilityMissing {
                    capability: capability.clone(),
                },
            )),
        },
        DenyReason::ReadOnlyViolation { field } => ProtoDenyReason {
            reason: Some(crate::proto::deny_reason::Reason::ReadOnlyViolation(
                ProtoReadOnlyViolation {
                    field: field.clone(),
                },
            )),
        },
        DenyReason::SchemaValidation { errors } => ProtoDenyReason {
            reason: Some(crate::proto::deny_reason::Reason::ConstraintFail(
                ProtoConstraintFail {
                    constraint: "schema_validation".to_string(),
                    message: errors.join("; "),
                },
            )),
        },
        DenyReason::Custom { reason } => ProtoDenyReason {
            reason: Some(crate::proto::deny_reason::Reason::ConstraintFail(
                ProtoConstraintFail {
                    constraint: "custom".to_string(),
                    message: reason.clone(),
                },
            )),
        },
    }
}

fn proto_snapshot(snapshot: &op_state_store::StateSnapshot) -> crate::proto::Snapshot {
    crate::proto::Snapshot {
        snapshot_id: snapshot.snapshot_id.clone(),
        at_event_id: snapshot.at_event_id,
        plugin_id: snapshot.plugin_id.clone(),
        schema_version: snapshot.schema_version.clone(),
        stub_hash: snapshot.stub_hash.clone(),
        immutable_wrappers_hash: snapshot.immutable_wrappers_hash.clone(),
        tunable_patch_hash: snapshot.tunable_patch_hash.clone(),
        effective_hash: snapshot.effective_hash.clone(),
        timestamp: Some(proto_timestamp(snapshot.timestamp)),
        state: Some(simd_to_prost_struct(&snapshot.state)),
    }
}

fn proto_timestamp(ts: DateTime<Utc>) -> ProstTimestamp {
    ProstTimestamp {
        seconds: ts.timestamp(),
        nanos: ts.timestamp_subsec_nanos() as i32,
    }
}

fn simd_to_prost_struct(value: &simd_json::OwnedValue) -> ProstStruct {
    match value.as_object() {
        Some(map) => {
            let fields = map
                .iter()
                .map(|(k, v)| (k.to_string(), simd_to_prost_value(v)))
                .collect();
            ProstStruct { fields }
        }
        None => ProstStruct {
            fields: BTreeMap::new(),
        },
    }
}

fn simd_to_prost_value(value: &simd_json::OwnedValue) -> ProstValue {
    use prost_types::value::Kind;
    if value.as_null().is_some() {
        return ProstValue {
            kind: Some(Kind::NullValue(0)),
        };
    }
    if let Some(b) = value.as_bool() {
        return ProstValue {
            kind: Some(Kind::BoolValue(b)),
        };
    }
    if let Some(n) = value.as_f64() {
        return ProstValue {
            kind: Some(Kind::NumberValue(n)),
        };
    }
    if let Some(s) = value.as_str() {
        return ProstValue {
            kind: Some(Kind::StringValue(s.to_string())),
        };
    }
    if let Some(arr) = value.as_array() {
        let vals = arr.iter().map(simd_to_prost_value).collect();
        return ProstValue {
            kind: Some(Kind::ListValue(prost_types::ListValue { values: vals })),
        };
    }
    if let Some(obj) = value.as_object() {
        let fields = obj
            .iter()
            .map(|(k, v)| (k.to_string(), simd_to_prost_value(v)))
            .collect();
        return ProstValue {
            kind: Some(Kind::StructValue(ProstStruct { fields })),
        };
    }
    ProstValue {
        kind: Some(Kind::NullValue(0)),
    }
}

fn prost_value_to_simd(value: &ProstValue) -> simd_json::OwnedValue {
    use prost_types::value::Kind;
    match &value.kind {
        None => simd_json::json!(null),
        Some(Kind::NullValue(_)) => simd_json::json!(null),
        Some(Kind::BoolValue(b)) => simd_json::json!(*b),
        Some(Kind::NumberValue(n)) => simd_json::json!(*n),
        Some(Kind::StringValue(s)) => simd_json::json!(s),
        Some(Kind::StructValue(s)) => {
            let mut map = simd_json::value::owned::Object::new();
            for (k, v) in &s.fields {
                map.insert(k.clone(), prost_value_to_simd(v));
            }
            simd_json::OwnedValue::Object(Box::new(map))
        }
        Some(Kind::ListValue(l)) => {
            let arr = l.values.iter().map(prost_value_to_simd).collect::<Vec<_>>();
            simd_json::OwnedValue::from(arr)
        }
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

    Err(anyhow::anyhow!(
        "Unsupported argument type; use tagged {{sig,value}} or primitives"
    ))
}

fn zvariant_from_sig(
    sig: &str,
    value: &simd_json::OwnedValue,
) -> Result<ZOwnedValue, anyhow::Error> {
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
                .map(|v| {
                    v.as_u64()
                        .map(|n| n as u8)
                        .ok_or_else(|| anyhow::anyhow!("Expected u8 in ay array"))
                })
                .collect();
            ZOwnedValue::try_from(ZValue::Array(ZArray::from(bytes?)))
                .map_err(|e| anyhow::anyhow!("Array conversion error: {}", e))
        }
        _ => Err(anyhow::anyhow!("Unsupported signature '{}'", sig)),
    }
}
