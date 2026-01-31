//! OP Execution Tracker - Lightweight Execution Monitoring Layer
//!
//! Complements existing state management by providing:
//! - Execution acknowledgment protocol
//! - Real-time execution tracking
//! - Integration with existing workflow/orchestration states
//! - Observability without duplicating state management

pub mod execution_context;

pub mod execution_tracker;

pub mod metrics;

pub mod telemetry;

pub mod record;



pub use execution_context::{ExecutionContext, ExecutionResult};

// ExecutionStatus is now in record.rs or we need to handle conflict.

// If record.rs defines it, we should use that or alias.

pub use execution_tracker::{ExecutionTracker, ExecutionEvent, ExecutionStats};

pub use metrics::ExecutionMetrics;

pub use telemetry::ExecutionTelemetry;

pub use record::{ExecutionRecord, ExecutionTiming, ExecutionStatus, ExecutionRecordBuilder, hash_execution};


