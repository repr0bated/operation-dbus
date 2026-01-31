//! D-Bus ↔ gRPC Bidirectional Bridge
//!
//! Provides live synchronization between D-Bus objects and gRPC services:
//! - D-Bus property changes → gRPC streaming updates
//! - gRPC mutations → D-Bus method calls / property sets
//! - D-Bus signals → gRPC server-streaming
//! - All changes flow through the event chain for audit/compliance
//!
//! Architecture:
//! ```text
//!                     ┌─────────────────┐
//!                     │   Event Chain   │ ← Source of truth
//!                     │  (audit + hash) │
//!                     └────────┬────────┘
//!                              │
//!               ┌──────────────┴──────────────┐
//!               ▼                              ▼
//!     ┌─────────────────┐            ┌─────────────────┐
//!     │     D-Bus       │◄──────────►│      gRPC       │
//!     │  (local IPC)    │            │  (remote RPC)   │
//!     └─────────────────┘            └─────────────────┘
//! ```

pub mod dbus_watcher;
pub mod grpc_client;
pub mod grpc_server;
pub mod proto_gen;
pub mod sync_engine;

// Re-export main types
pub use dbus_watcher::{DbusWatcher, WatchConfig};
pub use grpc_client::{GrpcClientPool, RemoteOperationClient, RemoteEndpoint};
pub use grpc_server::{OperationGrpcServer, PluginSchemaProvider, run_grpc_server};
pub use proto_gen::{ProtoGenerator, ProtoGenConfig};
pub use sync_engine::{SyncEngine, StateChange, ChangeSource};

/// Generated protobuf types
pub mod proto {
    tonic::include_proto!("operation.v1");
}
