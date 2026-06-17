pub mod engine;
pub mod policy;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

// ── Workload types ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadType {
    Sql,
    VectorSearch,
    AIML,
    GraphTraversal,
    CdcPipeline,
    Backup,
    Restore,
    Migration,
    ClusterOperation,
    ProtocolProcessing,
    UserDefinedFunction,
    StoredProcedure,
    Plugin,
    Ffi,
}

impl WorkloadType {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuUsage {
    pub execution_steps: u64,
    pub cpu_time_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub allocated_bytes: u64,
    pub peak_memory_bytes: u64,
    pub temporary_memory_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryComplexityUsage {
    pub joins: u32,
    pub nested_queries: u32,
    pub graph_depth: u32,
    pub vector_candidates: u32,
    pub sort_operations: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineUsage {
    pub stages: u32,
    pub transform_steps: u32,
    pub operators: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FfiUsage {
    pub calls: u64,
    pub memory_allocations: u64,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AimlUsage {
    pub training_iterations: u64,
    pub prediction_batches: u64,
    pub embedding_generations: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorUsage {
    pub candidate_expansions: u64,
    pub hnsw_traversals: u64,
    pub ivf_probes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphUsage {
    pub depth: u32,
    pub visited_nodes: u64,
    pub visited_edges: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationUsage {
    pub rows_imported: u64,
    pub documents_imported: u64,
    pub batches_processed: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupUsage {
    pub objects_processed: u64,
    pub bytes_processed: u64,
    pub duration_ms: u64,
}

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
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ── Execution limits ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuLimits {
    pub max_execution_steps: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub max_memory_mb: Option<u64>,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: Some(2048),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryComplexityLimits {
    pub max_query_complexity: Option<u32>,
    pub max_join_count: Option<u32>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineLimits {
    pub max_pipeline_depth: Option<u32>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfiLimits {
    pub max_ffi_calls: Option<u64>,
    pub max_ffi_memory_mb: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimlLimits {
    pub max_training_iterations: Option<u64>,
    pub max_prediction_batch_size: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorLimits {
    pub max_vector_candidates: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphLimits {
    pub max_graph_depth: Option<u32>,
    pub max_graph_nodes: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationLimits {
    pub max_import_rows: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupLimits {
    pub max_backup_size: Option<u64>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementAction {
    Monitor,
    Warn,
    Throttle,
    Block,
}

impl EnforcementAction {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub execution_id: Uuid,
    pub namespace: String,
    pub workload_type: WorkloadType,
    pub policy_name: String,
    pub limit_name: String,
    pub limit_value: String,
    pub usage_value: String,
    pub action: EnforcementAction,
}

// ── Execution context ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: Uuid,
    pub namespace: String,
    pub workload_type: WorkloadType,
    pub limits: ExecutionLimits,
    pub usage: ExecutionUsage,
    pub action: EnforcementAction,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl ExecutionContext {
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

    pub fn with_action(mut self, action: EnforcementAction) -> Self {
        self.action = action;
        self
    }

    pub fn elapsed_ms(&self) -> i64 {
        (chrono::Utc::now() - self.created_at)
            .num_milliseconds()
    }
}

// ── Policy ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub scope: PolicyScope,
    pub limits: ExecutionLimits,
    pub action: EnforcementAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyScope {
    Global,
    Cluster(String),
    Node(String),
    Namespace(String),
    Database(String),
    Role(String),
    User(String),
    WorkloadType(WorkloadType),
}

impl PolicyScope {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorConfig {
    pub enabled: bool,
    pub default_policy: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
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
    pub action: Option<String>,
}

// ── Shared state ───────────────────────────────────────────────

static EXECUTION_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn next_execution_number() -> u64 {
    EXECUTION_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn reset_execution_counter() {
    EXECUTION_COUNTER.store(0, Ordering::Relaxed);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorStatus {
    pub enabled: bool,
    pub active_executions: usize,
    pub total_violations: u64,
    pub blocked_count: u64,
    pub throttled_count: u64,
    pub policies_loaded: usize,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorMetricsSnapshot {
    pub active_executions: usize,
    pub blocked_total: u64,
    pub throttled_total: u64,
    pub policy_violations_total: u64,
    pub memory_usage_bytes: u64,
    pub cpu_time_ms: u64,
    pub ffi_calls_total: u64,
}
