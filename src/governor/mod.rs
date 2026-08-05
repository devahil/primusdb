//! # Governor — Resource Usage Governance
//!
//! Tracks resource usage per execution and enforces limits via configurable
//! policies scoped by namespace, user, role or workload type.
//!
//! ```text
//! GovernorEngine
//!   +-- PolicyManager   policy resolution + inheritance
//!   +-- Executions     ExecutionContext map (uuid -> context)
//!   +-- Violations     capped violation log (10 000 entries)
//!   |
//!   +-- start_execution() -> ExecutionHandle
//!   |     +-- check_*()     per-limit checks
//!   |     +-- finish()      deregister execution
//!   +-- status() / metrics_snapshot() / policies()
//! ```
//!
//! Enforcement actions: `monitor` (log only), `warn`, `throttle`, or `block`
//! (error returned to the caller). When the governor is disabled all checks
//! return [`EnforcementAction::Monitor`].
//!
//! ## Usage Types
//!
//! Per-execution usage is tracked as [`ExecutionUsage`], which aggregates CPU,
//! memory, query complexity, pipeline, FFI, AI/ML, vector, graph, migration and
//! backup meters. Limits for the same dimensions live in [`ExecutionLimits`].

pub mod engine;
pub mod policy;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

// ── Workload types ─────────────────────────────────────────────

/// Category of work an execution represents, used for policy scoping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadType {
    /// SQL query or transaction.
    Sql,
    /// Vector similarity search.
    VectorSearch,
    /// AI/ML training or inference.
    AIML,
    /// Graph traversal operation.
    GraphTraversal,
    /// Change-data-capture pipeline.
    CdcPipeline,
    /// Backup operation.
    Backup,
    /// Restore operation.
    Restore,
    /// Data migration operation.
    Migration,
    /// Cluster-wide coordination operation.
    ClusterOperation,
    /// Protocol parsing/processing.
    ProtocolProcessing,
    /// User-defined function invocation.
    UserDefinedFunction,
    /// Stored procedure execution.
    StoredProcedure,
    /// Plugin execution.
    Plugin,
    /// FFI (foreign function interface) call.
    Ffi,
}

impl WorkloadType {
    /// Returns the stable string name of this workload type.
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkloadType::Sql => "sql",
            WorkloadType::VectorSearch => "vector_search",
            WorkloadType::AIML => "ai_ml",
            WorkloadType::GraphTraversal => "graph_traversal",
            WorkloadType::CdcPipeline => "cdc_pipeline",
            WorkloadType::Backup => "backup",
            WorkloadType::Restore => "restore",
            WorkloadType::Migration => "migration",
            WorkloadType::ClusterOperation => "cluster_operation",
            WorkloadType::ProtocolProcessing => "protocol_processing",
            WorkloadType::UserDefinedFunction => "udf",
            WorkloadType::StoredProcedure => "stored_procedure",
            WorkloadType::Plugin => "plugin",
            WorkloadType::Ffi => "ffi",
        }
    }
}

// ── Resource usage tracking ────────────────────────────────────

/// CPU usage meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuUsage {
    /// Number of execution steps performed.
    pub execution_steps: u64,
    /// Total CPU time consumed in milliseconds.
    pub cpu_time_ms: u64,
}

/// Memory usage meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Currently allocated bytes.
    pub allocated_bytes: u64,
    /// Peak allocated bytes observed.
    pub peak_memory_bytes: u64,
    /// Bytes allocated for temporary work.
    pub temporary_memory_bytes: u64,
}

/// Query complexity meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryComplexityUsage {
    /// Number of joins executed.
    pub joins: u32,
    /// Number of nested queries executed.
    pub nested_queries: u32,
    /// Maximum graph traversal depth.
    pub graph_depth: u32,
    /// Number of vector candidates scanned.
    pub vector_candidates: u32,
    /// Number of sort operations performed.
    pub sort_operations: u32,
}

/// Pipeline processing meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineUsage {
    /// Number of pipeline stages.
    pub stages: u32,
    /// Number of transform steps applied.
    pub transform_steps: u32,
    /// Number of operators invoked.
    pub operators: u32,
}

/// FFI invocation meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FfiUsage {
    /// Number of FFI calls made.
    pub calls: u64,
    /// Number of FFI memory allocations.
    pub memory_allocations: u64,
    /// Total FFI execution time in milliseconds.
    pub execution_time_ms: u64,
}

/// AI/ML operation meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AimlUsage {
    /// Number of training iterations.
    pub training_iterations: u64,
    /// Number of prediction batches processed.
    pub prediction_batches: u64,
    /// Number of embeddings generated.
    pub embedding_generations: u64,
}

/// Vector search meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorUsage {
    /// Number of candidate set expansions.
    pub candidate_expansions: u64,
    /// Number of HNSW graph traversals.
    pub hnsw_traversals: u64,
    /// Number of IVF probes performed.
    pub ivf_probes: u64,
}

/// Graph traversal meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphUsage {
    /// Maximum traversal depth reached.
    pub depth: u32,
    /// Number of nodes visited.
    pub visited_nodes: u64,
    /// Number of edges visited.
    pub visited_edges: u64,
}

/// Migration meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationUsage {
    /// Number of rows imported.
    pub rows_imported: u64,
    /// Number of documents imported.
    pub documents_imported: u64,
    /// Number of batches processed.
    pub batches_processed: u64,
}

/// Backup/restore meters for an execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupUsage {
    /// Number of objects processed.
    pub objects_processed: u64,
    /// Number of bytes processed.
    pub bytes_processed: u64,
    /// Duration of the operation in milliseconds.
    pub duration_ms: u64,
}

/// Aggregated resource usage for a single execution, one meter per subsystem.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionUsage {
    pub cpu: CpuUsage,
    pub memory: MemoryUsage,
    pub query_complexity: QueryComplexityUsage,
    pub pipeline: PipelineUsage,
    pub ffi: FfiUsage,
    pub aiml: AimlUsage,
    pub vector: VectorUsage,
    pub graph: GraphUsage,
    pub migration: MigrationUsage,
    pub backup: BackupUsage,
}

impl ExecutionUsage {
    /// Resets all meters to their default (zero) values.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ── Execution limits ──────────────────────────────────────────

/// CPU limits enforced against [`CpuUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLimits {
    /// Maximum execution steps before enforcement.
    pub max_execution_steps: Option<u64>,
    /// Maximum CPU time in milliseconds.
    pub max_cpu_time_ms: Option<u64>,
}

impl Default for CpuLimits {
    fn default() -> Self {
        Self {
            max_execution_steps: Some(10_000_000),
            max_cpu_time_ms: Some(300_000),
        }
    }
}

/// Memory limits enforced against [`MemoryUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    /// Maximum resident memory in megabytes.
    pub max_memory_mb: Option<u64>,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(2048),
        }
    }
}

/// Query complexity limits enforced against [`QueryComplexityUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryComplexityLimits {
    /// Maximum weighted query complexity score.
    pub max_query_complexity: Option<u32>,
    /// Maximum number of joins in a single query.
    pub max_join_count: Option<u32>,
    /// Maximum rows allowed in a sort.
    pub max_sort_rows: Option<u64>,
}

impl Default for QueryComplexityLimits {
    fn default() -> Self {
        Self {
            max_query_complexity: Some(100),
            max_join_count: Some(10),
            max_sort_rows: Some(1_000_000),
        }
    }
}

/// Pipeline limits enforced against [`PipelineUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineLimits {
    /// Maximum pipeline nesting depth.
    pub max_pipeline_depth: Option<u32>,
    /// Maximum number of pipeline stages.
    pub max_pipeline_stages: Option<u32>,
}

impl Default for PipelineLimits {
    fn default() -> Self {
        Self {
            max_pipeline_depth: Some(100),
            max_pipeline_stages: Some(50),
        }
    }
}

/// FFI limits enforced against [`FfiUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfiLimits {
    /// Maximum number of FFI calls.
    pub max_ffi_calls: Option<u64>,
    /// Maximum FFI-allocated memory in megabytes.
    pub max_ffi_memory_mb: Option<u64>,
    /// Maximum FFI execution time in milliseconds.
    pub max_ffi_time_ms: Option<u64>,
}

impl Default for FfiLimits {
    fn default() -> Self {
        Self {
            max_ffi_calls: Some(10_000),
            max_ffi_memory_mb: Some(512),
            max_ffi_time_ms: Some(30_000),
        }
    }
}

/// AI/ML limits enforced against [`AimlUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimlLimits {
    /// Maximum training iterations.
    pub max_training_iterations: Option<u64>,
    /// Maximum prediction batch size.
    pub max_prediction_batch_size: Option<u64>,
    /// Maximum embedding batch size.
    pub max_embedding_batch_size: Option<u64>,
}

impl Default for AimlLimits {
    fn default() -> Self {
        Self {
            max_training_iterations: Some(100_000),
            max_prediction_batch_size: Some(10_000),
            max_embedding_batch_size: Some(10_000),
        }
    }
}

/// Vector search limits enforced against [`VectorUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorLimits {
    /// Maximum number of vector candidates.
    pub max_vector_candidates: Option<u64>,
    /// Maximum number of candidate expansions.
    pub max_vector_expansions: Option<u64>,
}

impl Default for VectorLimits {
    fn default() -> Self {
        Self {
            max_vector_candidates: Some(100_000),
            max_vector_expansions: Some(100),
        }
    }
}

/// Graph traversal limits enforced against [`GraphUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLimits {
    /// Maximum traversal depth.
    pub max_graph_depth: Option<u32>,
    /// Maximum number of nodes visited.
    pub max_graph_nodes: Option<u64>,
    /// Maximum number of edges visited.
    pub max_graph_edges: Option<u64>,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_graph_depth: Some(100),
            max_graph_nodes: Some(1_000_000),
            max_graph_edges: Some(10_000_000),
        }
    }
}

/// Migration limits enforced against [`MigrationUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationLimits {
    /// Maximum rows to import.
    pub max_import_rows: Option<u64>,
    /// Maximum number of import batches.
    pub max_import_batches: Option<u64>,
}

impl Default for MigrationLimits {
    fn default() -> Self {
        Self {
            max_import_rows: Some(10_000_000),
            max_import_batches: Some(10_000),
        }
    }
}

/// Backup/restore limits enforced against [`BackupUsage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupLimits {
    /// Maximum backup size in bytes.
    pub max_backup_size: Option<u64>,
    /// Maximum restore size in bytes.
    pub max_restore_size: Option<u64>,
}

impl Default for BackupLimits {
    fn default() -> Self {
        Self {
            max_backup_size: Some(100 * 1024 * 1024 * 1024),
            max_restore_size: Some(100 * 1024 * 1024 * 1024),
        }
    }
}

/// Aggregated limits across all governed subsystems.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionLimits {
    pub cpu: CpuLimits,
    pub memory: MemoryLimits,
    pub query_complexity: QueryComplexityLimits,
    pub pipeline: PipelineLimits,
    pub ffi: FfiLimits,
    pub aiml: AimlLimits,
    pub vector: VectorLimits,
    pub graph: GraphLimits,
    pub migration: MigrationLimits,
    pub backup: BackupLimits,
}

// ── Enforcement action ────────────────────────────────────────

/// What the governor does when a limit is exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementAction {
    /// Record usage without any enforcement.
    Monitor,
    /// Log a warning but continue execution.
    Warn,
    /// Allow execution but signal throttling to the caller.
    Throttle,
    /// Abort the execution with an error.
    Block,
}

impl EnforcementAction {
    /// Returns the stable string name of this action.
    pub fn as_str(&self) -> &'static str {
        match self {
            EnforcementAction::Monitor => "monitor",
            EnforcementAction::Warn => "warn",
            EnforcementAction::Throttle => "throttle",
            EnforcementAction::Block => "block",
        }
    }
}

// ── Violation ──────────────────────────────────────────────────

/// A recorded enforcement event for a limit that was exceeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Randomly generated violation id.
    pub id: Uuid,
    /// Time the violation was recorded.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Id of the offending execution.
    pub execution_id: Uuid,
    /// Namespace the execution ran in.
    pub namespace: String,
    /// Workload type of the execution.
    pub workload_type: WorkloadType,
    /// Name of the resolved policy.
    pub policy_name: String,
    /// Name of the limit that was exceeded.
    pub limit_name: String,
    /// Configured limit value.
    pub limit_value: String,
    /// Observed usage value.
    pub usage_value: String,
    /// Enforcement action taken.
    pub action: EnforcementAction,
}

// ── Execution context ──────────────────────────────────────────

/// Per-execution state held by the governor while a workload is running.
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Unique execution id.
    pub execution_id: Uuid,
    /// Namespace the execution belongs to.
    pub namespace: String,
    /// Workload type of the execution.
    pub workload_type: WorkloadType,
    /// Resolved limits for this execution.
    pub limits: ExecutionLimits,
    /// Accumulated resource usage.
    pub usage: ExecutionUsage,
    /// Enforcement action resolved from the applicable policy.
    pub action: EnforcementAction,
    /// When the execution started.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last time the context was updated.
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl ExecutionContext {
    /// Creates a fresh context with default usage and `Monitor` action.
    pub fn new(namespace: String, workload_type: WorkloadType, limits: ExecutionLimits) -> Self {
        let now = chrono::Utc::now();
        Self {
            execution_id: Uuid::new_v4(),
            namespace,
            workload_type,
            limits,
            usage: ExecutionUsage::default(),
            action: EnforcementAction::Monitor,
            created_at: now,
            last_updated: now,
        }
    }

    /// Returns the context with an explicit enforcement action.
    pub fn with_action(mut self, action: EnforcementAction) -> Self {
        self.action = action;
        self
    }

    /// Milliseconds elapsed since the context was created.
    pub fn elapsed_ms(&self) -> i64 {
        (chrono::Utc::now() - self.created_at).num_milliseconds()
    }
}

// ── Policy ─────────────────────────────────────────────────────

/// A named governance policy with a scope, limits and enforcement action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy name.
    pub name: String,
    /// Scope the policy applies to.
    pub scope: PolicyScope,
    /// Limits enforced by the policy.
    pub limits: ExecutionLimits,
    /// Enforcement action taken when a limit is exceeded.
    pub action: EnforcementAction,
}

/// Defines which workloads a policy applies to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyScope {
    /// Applies to every workload.
    Global,
    /// Applies to a named cluster.
    Cluster(String),
    /// Applies to a named node.
    Node(String),
    /// Applies to a namespace.
    Namespace(String),
    /// Applies to a database.
    Database(String),
    /// Applies to a role.
    Role(String),
    /// Applies to a user.
    User(String),
    /// Applies to a workload type.
    WorkloadType(WorkloadType),
}

impl PolicyScope {
    /// Returns the category label of this scope.
    pub fn as_str(&self) -> &str {
        match self {
            PolicyScope::Global => "global",
            PolicyScope::Cluster(_) => "cluster",
            PolicyScope::Node(_) => "node",
            PolicyScope::Namespace(_) => "namespace",
            PolicyScope::Database(_) => "database",
            PolicyScope::Role(_) => "role",
            PolicyScope::User(_) => "user",
            PolicyScope::WorkloadType(_) => "workload_type",
        }
    }

    /// Returns the target name embedded in this scope (`global` for
    /// [`PolicyScope::Global`]).
    pub fn name(&self) -> &str {
        match self {
            PolicyScope::Global => "global",
            PolicyScope::Cluster(n) => n,
            PolicyScope::Node(n) => n,
            PolicyScope::Namespace(n) => n,
            PolicyScope::Database(n) => n,
            PolicyScope::Role(n) => n,
            PolicyScope::User(n) => n,
            PolicyScope::WorkloadType(w) => w.as_str(),
        }
    }
}

// ── Policy config (TOML-friendly) ─────────────────────────────

/// TOML-friendly governor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorConfig {
    /// Whether the governor is active.
    pub enabled: bool,
    /// Name of the policy used when no policy matches.
    pub default_policy: String,
    /// Named policy definitions keyed by policy name.
    pub policies: HashMap<String, PolicyConfig>,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        let mut policies = HashMap::new();
        policies.insert(
            "default".to_string(),
            PolicyConfig {
                scope: "global".to_string(),
                max_memory_mb: Some(2048),
                max_execution_steps: Some(10_000_000),
                max_cpu_time_ms: Some(300_000),
                max_query_complexity: Some(100),
                max_join_count: Some(10),
                max_sort_rows: Some(1_000_000),
                max_pipeline_depth: Some(100),
                max_pipeline_stages: Some(50),
                max_ffi_calls: Some(10_000),
                max_ffi_memory_mb: Some(512),
                max_ffi_time_ms: Some(30_000),
                max_training_iterations: Some(100_000),
                max_prediction_batch_size: Some(10_000),
                max_embedding_batch_size: Some(10_000),
                max_vector_candidates: Some(100_000),
                max_vector_expansions: Some(100),
                max_graph_depth: Some(100),
                max_graph_nodes: Some(1_000_000),
                max_graph_edges: Some(10_000_000),
                max_import_rows: Some(10_000_000),
                max_import_batches: Some(10_000),
                max_backup_size: Some(100 * 1024 * 1024 * 1024),
                max_restore_size: Some(100 * 1024 * 1024 * 1024),
                action: Some("monitor".to_string()),
            },
        );
        Self {
            enabled: true,
            default_policy: "default".to_string(),
            policies,
        }
    }
}

/// Flat, serializable representation of a policy's limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Scope string, e.g. `global`, `namespace:analytics`, `sql`.
    pub scope: String,
    pub max_memory_mb: Option<u64>,
    pub max_execution_steps: Option<u64>,
    pub max_cpu_time_ms: Option<u64>,
    pub max_query_complexity: Option<u32>,
    pub max_join_count: Option<u32>,
    pub max_sort_rows: Option<u64>,
    pub max_pipeline_depth: Option<u32>,
    pub max_pipeline_stages: Option<u32>,
    pub max_ffi_calls: Option<u64>,
    pub max_ffi_memory_mb: Option<u64>,
    pub max_ffi_time_ms: Option<u64>,
    pub max_training_iterations: Option<u64>,
    pub max_prediction_batch_size: Option<u64>,
    pub max_embedding_batch_size: Option<u64>,
    pub max_vector_candidates: Option<u64>,
    pub max_vector_expansions: Option<u64>,
    pub max_graph_depth: Option<u32>,
    pub max_graph_nodes: Option<u64>,
    pub max_graph_edges: Option<u64>,
    pub max_import_rows: Option<u64>,
    pub max_import_batches: Option<u64>,
    pub max_backup_size: Option<u64>,
    pub max_restore_size: Option<u64>,
    /// Enforcement action: `monitor`, `warn`, `throttle` or `block`.
    pub action: Option<String>,
}

// ── Shared state ───────────────────────────────────────────────

static EXECUTION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Returns the next monotonically increasing execution sequence number.
pub fn next_execution_number() -> u64 {
    EXECUTION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Resets the shared execution sequence counter to zero (tests only).
pub fn reset_execution_counter() {
    EXECUTION_COUNTER.store(0, Ordering::Relaxed);
}

/// High-level snapshot of the governor's state for observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorStatus {
    /// Whether the governor is enabled.
    pub enabled: bool,
    /// Number of currently running executions.
    pub active_executions: usize,
    /// Total violations recorded.
    pub total_violations: u64,
    /// Total executions blocked.
    pub blocked_count: u64,
    /// Total executions throttled.
    pub throttled_count: u64,
    /// Number of loaded policies.
    pub policies_loaded: usize,
    /// Uptime of the engine in seconds.
    pub uptime_seconds: u64,
}

/// Aggregated metric counters sampled from the governor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorMetricsSnapshot {
    /// Number of currently running executions.
    pub active_executions: usize,
    /// Total executions blocked.
    pub blocked_total: u64,
    /// Total executions throttled.
    pub throttled_total: u64,
    /// Total policy violations recorded.
    pub policy_violations_total: u64,
    /// Sum of allocated bytes across active executions.
    pub memory_usage_bytes: u64,
    /// Sum of CPU time across active executions.
    pub cpu_time_ms: u64,
    /// Sum of FFI calls across active executions.
    pub ffi_calls_total: u64,
}
