use super::policy::PolicyManager;
use super::{
    EnforcementAction, ExecutionContext, ExecutionLimits, GovernorConfig,
    GovernorMetricsSnapshot, GovernorStatus, Policy, Violation, WorkloadType,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

struct Inner {
    policies: RwLock<PolicyManager>,
    executions: RwLock<HashMap<Uuid, ExecutionContext>>,
    violations: RwLock<Vec<Violation>>,
    config: RwLock<GovernorConfig>,
    start_time: Instant,
    active_executions: AtomicUsize,
    total_blocked: AtomicU64,
    total_throttled: AtomicU64,
    total_violations: AtomicU64,
}

pub struct GovernorEngine {
    inner: Arc<Inner>,
}

impl GovernorEngine {
    pub fn new(config: GovernorConfig) -> Self {
        let policy_mgr = PolicyManager::from_config(&config.policies, &config.default_policy);
        Self {
            inner: Arc::new(Inner {
                policies: RwLock::new(policy_mgr),
                executions: RwLock::new(HashMap::new()),
                violations: RwLock::new(Vec::new()),
                config: RwLock::new(config),
                start_time: Instant::now(),
                active_executions: AtomicUsize::new(0),
                total_blocked: AtomicU64::new(0),
                total_throttled: AtomicU64::new(0),
                total_violations: AtomicU64::new(0),
            }),
        }
    }

    pub fn new_disabled() -> Self {
        Self::new(GovernorConfig {
            enabled: false,
            ..GovernorConfig::default()
        })
    }

    pub async fn is_enabled(&self) -> bool {
        self.inner.config.read().await.enabled
    }

    pub async fn start_execution(
        &self,
        namespace: String,
        workload_type: WorkloadType,
        user: Option<&str>,
        role: Option<&str>,
    ) -> ExecutionHandle {
        let policy_mgr = self.inner.policies.read().await;
        let policy = policy_mgr.resolve_policy(&namespace, workload_type, user, role);
        let limits = policy.limits.clone();
        let action = policy.action;

        let ctx = ExecutionContext::new(namespace, workload_type, limits)
            .with_action(action);

        let exec_id = ctx.execution_id;
        self.inner.active_executions.fetch_add(1, Ordering::Relaxed);
        {
            let mut execs = self.inner.executions.write().await;
            execs.insert(exec_id, ctx);
        }

        ExecutionHandle {
            engine: self.inner.clone(),
            execution_id: exec_id,
            action,
        }
    }

    pub async fn finish_execution(&self, execution_id: Uuid) {
        self.inner.active_executions.fetch_sub(1, Ordering::Relaxed);
        let mut execs = self.inner.executions.write().await;
        execs.remove(&execution_id);
    }

    pub async fn check_limit(
        &self,
        execution_id: Uuid,
        field: &str,
        current: u64,
        limit: Option<u64>,
    ) -> Result<EnforcementAction, String> {
        let enabled = self.is_enabled().await;
        if !enabled {
            return Ok(EnforcementAction::Monitor);
        }

        let Some(limit) = limit else {
            return Ok(EnforcementAction::Monitor);
        };

        if current >= limit {
            let action = {
                let execs = self.inner.executions.read().await;
                execs
                    .get(&execution_id)
                    .map(|e| e.action)
                    .unwrap_or(EnforcementAction::Block)
            };

            if action == EnforcementAction::Block {
                self.inner.total_blocked.fetch_add(1, Ordering::Relaxed);
            } else if action == EnforcementAction::Throttle {
                self.inner.total_throttled.fetch_add(1, Ordering::Relaxed);
            }
            self.inner.total_violations.fetch_add(1, Ordering::Relaxed);

            let namespace = {
                let execs = self.inner.executions.read().await;
                execs
                    .get(&execution_id)
                    .map(|e| e.namespace.clone())
                    .unwrap_or_default()
            };
            let workload_type = {
                let execs = self.inner.executions.read().await;
                execs
                    .get(&execution_id)
                    .map(|e| e.workload_type)
                    .unwrap_or(WorkloadType::Sql)
            };

            let violation = Violation {
                id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                execution_id,
                namespace,
                workload_type,
                policy_name: "resolved".to_string(),
                limit_name: field.to_string(),
                limit_value: format!("{limit}"),
                usage_value: format!("{current}"),
                action,
            };

            {
                let mut violations = self.inner.violations.write().await;
                violations.push(violation.clone());
                if violations.len() > 10_000 {
                    let excess = violations.len() - 5_000;
                    violations.drain(0..excess);
                }
            }

            tracing::warn!(
                target: "GovernorViolation",
                "Execution: {} Policy: {} Limit: {} = {} Usage: {} = {} Action: {}",
                execution_id, field, limit_name(field), limit, field, current, action.as_str()
            );

            if action == EnforcementAction::Block {
                return Err(format!(
                    "Resource limit exceeded: {} (limit: {}, usage: {}). Action: blocked",
                    field, limit, current
                ));
            }

            Ok(action)
        } else {
            Ok(EnforcementAction::Monitor)
        }
    }

    pub async fn check_memory(&self, execution_id: Uuid, current_mb: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.memory.max_memory_mb)
        };
        self.check_limit(execution_id, "max_memory_mb", current_mb, limit)
            .await
    }

    pub async fn check_execution_steps(
        &self,
        execution_id: Uuid,
        steps: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.cpu.max_execution_steps)
        };
        self.check_limit(execution_id, "max_execution_steps", steps, limit)
            .await
    }

    pub async fn check_cpu_time(
        &self,
        execution_id: Uuid,
        cpu_time_ms: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.cpu.max_cpu_time_ms)
        };
        self.check_limit(execution_id, "max_cpu_time_ms", cpu_time_ms, limit)
            .await
    }

    pub async fn check_query_complexity(
        &self,
        execution_id: Uuid,
        complexity: u32,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.query_complexity.max_query_complexity)
                .map(|v| v as u64)
        };
        self.check_limit(execution_id, "max_query_complexity", complexity as u64, limit)
            .await
    }

    pub async fn check_join_count(
        &self,
        execution_id: Uuid,
        joins: u32,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.query_complexity.max_join_count)
                .map(|v| v as u64)
        };
        self.check_limit(execution_id, "max_join_count", joins as u64, limit)
            .await
    }

    pub async fn check_sort_rows(
        &self,
        execution_id: Uuid,
        rows: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.query_complexity.max_sort_rows)
        };
        self.check_limit(execution_id, "max_sort_rows", rows, limit)
            .await
    }

    pub async fn check_pipeline_depth(
        &self,
        execution_id: Uuid,
        depth: u32,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.pipeline.max_pipeline_depth)
                .map(|v| v as u64)
        };
        self.check_limit(execution_id, "max_pipeline_depth", depth as u64, limit)
            .await
    }

    pub async fn check_pipeline_stages(
        &self,
        execution_id: Uuid,
        stages: u32,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.pipeline.max_pipeline_stages)
                .map(|v| v as u64)
        };
        self.check_limit(execution_id, "max_pipeline_stages", stages as u64, limit)
            .await
    }

    pub async fn check_ffi_calls(
        &self,
        execution_id: Uuid,
        calls: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.ffi.max_ffi_calls)
        };
        self.check_limit(execution_id, "max_ffi_calls", calls, limit)
            .await
    }

    pub async fn check_ffi_memory(
        &self,
        execution_id: Uuid,
        memory_mb: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.ffi.max_ffi_memory_mb)
        };
        self.check_limit(execution_id, "max_ffi_memory_mb", memory_mb, limit)
            .await
    }

    pub async fn check_ffi_time(
        &self,
        execution_id: Uuid,
        time_ms: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.ffi.max_ffi_time_ms)
        };
        self.check_limit(execution_id, "max_ffi_time_ms", time_ms, limit)
            .await
    }

    pub async fn check_training_iterations(
        &self,
        execution_id: Uuid,
        iterations: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.aiml.max_training_iterations)
        };
        self.check_limit(execution_id, "max_training_iterations", iterations, limit)
            .await
    }

    pub async fn check_prediction_batch(
        &self,
        execution_id: Uuid,
        batch_size: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.aiml.max_prediction_batch_size)
        };
        self.check_limit(execution_id, "max_prediction_batch_size", batch_size, limit)
            .await
    }

    pub async fn check_embedding_batch(
        &self,
        execution_id: Uuid,
        batch_size: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.aiml.max_embedding_batch_size)
        };
        self.check_limit(execution_id, "max_embedding_batch_size", batch_size, limit)
            .await
    }

    pub async fn check_vector_candidates(
        &self,
        execution_id: Uuid,
        candidates: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.vector.max_vector_candidates)
        };
        self.check_limit(execution_id, "max_vector_candidates", candidates, limit)
            .await
    }

    pub async fn check_vector_expansions(
        &self,
        execution_id: Uuid,
        expansions: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.vector.max_vector_expansions)
        };
        self.check_limit(execution_id, "max_vector_expansions", expansions, limit)
            .await
    }

    pub async fn check_graph_depth(
        &self,
        execution_id: Uuid,
        depth: u32,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.graph.max_graph_depth)
                .map(|v| v as u64)
        };
        self.check_limit(execution_id, "max_graph_depth", depth as u64, limit)
            .await
    }

    pub async fn check_graph_nodes(
        &self,
        execution_id: Uuid,
        nodes: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.graph.max_graph_nodes)
        };
        self.check_limit(execution_id, "max_graph_nodes", nodes, limit)
            .await
    }

    pub async fn check_graph_edges(
        &self,
        execution_id: Uuid,
        edges: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.graph.max_graph_edges)
        };
        self.check_limit(execution_id, "max_graph_edges", edges, limit)
            .await
    }

    pub async fn check_import_rows(
        &self,
        execution_id: Uuid,
        rows: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.migration.max_import_rows)
        };
        self.check_limit(execution_id, "max_import_rows", rows, limit)
            .await
    }

    pub async fn check_import_batches(
        &self,
        execution_id: Uuid,
        batches: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.migration.max_import_batches)
        };
        self.check_limit(execution_id, "max_import_batches", batches, limit)
            .await
    }

    pub async fn check_backup_size(
        &self,
        execution_id: Uuid,
        size_mb: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.backup.max_backup_size)
        };
        self.check_limit(execution_id, "max_backup_size", size_mb, limit)
            .await
    }

    pub async fn check_restore_size(
        &self,
        execution_id: Uuid,
        size_mb: u64,
    ) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.inner.executions.read().await;
            execs
                .get(&execution_id)
                .and_then(|e| e.limits.backup.max_restore_size)
        };
        self.check_limit(execution_id, "max_restore_size", size_mb, limit)
            .await
    }

    pub async fn get_execution(&self, id: Uuid) -> Option<ExecutionContext> {
        let execs = self.inner.executions.read().await;
        execs.get(&id).cloned()
    }

    pub async fn list_executions(&self) -> Vec<ExecutionContext> {
        let execs = self.inner.executions.read().await;
        execs.values().cloned().collect()
    }

    pub async fn list_violations(&self) -> Vec<Violation> {
        let violations = self.inner.violations.read().await;
        violations.clone()
    }

    pub async fn violations_since(&self, since: chrono::DateTime<chrono::Utc>) -> Vec<Violation> {
        let violations = self.inner.violations.read().await;
        violations
            .iter()
            .filter(|v| v.timestamp >= since)
            .cloned()
            .collect()
    }

    pub async fn status(&self) -> GovernorStatus {
        let enabled = self.is_enabled().await;
        let policies_loaded = self.inner.policies.read().await.list_policies().len();
        GovernorStatus {
            enabled,
            active_executions: self.inner.active_executions.load(Ordering::Relaxed),
            total_violations: self.inner.total_violations.load(Ordering::Relaxed),
            blocked_count: self.inner.total_blocked.load(Ordering::Relaxed),
            throttled_count: self.inner.total_throttled.load(Ordering::Relaxed),
            policies_loaded,
            uptime_seconds: self.inner.start_time.elapsed().as_secs(),
        }
    }

    pub async fn metrics_snapshot(&self) -> GovernorMetricsSnapshot {
        let (memory_usage_bytes, cpu_time_ms, ffi_calls_total) = {
            let execs = self.inner.executions.read().await;
            let mem: u64 = execs
                .values()
                .map(|e| e.usage.memory.allocated_bytes)
                .sum();
            let cpu: u64 = execs.values().map(|e| e.usage.cpu.cpu_time_ms).sum();
            let ffi: u64 = execs.values().map(|e| e.usage.ffi.calls).sum();
            (mem, cpu, ffi)
        };
        GovernorMetricsSnapshot {
            active_executions: self.inner.active_executions.load(Ordering::Relaxed),
            blocked_total: self.inner.total_blocked.load(Ordering::Relaxed),
            throttled_total: self.inner.total_throttled.load(Ordering::Relaxed),
            policy_violations_total: self.inner.total_violations.load(Ordering::Relaxed),
            memory_usage_bytes,
            cpu_time_ms,
            ffi_calls_total,
        }
    }

    pub async fn policies(&self) -> Vec<Policy> {
        self.inner.policies.read().await.list_policies()
    }

    pub async fn update_policy(
        &self,
        name: &str,
        limits: ExecutionLimits,
        action: EnforcementAction,
        scope: String,
    ) {
        let mut pm = self.inner.policies.write().await;
        let scope = PolicyManager::parse_scope(&scope).unwrap_or(super::PolicyScope::Global);
        let policy = Policy {
            name: name.to_string(),
            scope,
            limits,
            action,
        };
        pm.add_policy(policy);
    }

    pub fn shared(&self) -> Arc<GovernorEngine> {
        Arc::new(GovernorEngine {
            inner: self.inner.clone(),
        })
    }
}

fn limit_name(field: &str) -> &'static str {
    match field {
        "max_memory_mb" => "Memory limit",
        "max_execution_steps" => "Execution steps limit",
        "max_cpu_time_ms" => "CPU time limit",
        "max_query_complexity" => "Query complexity limit",
        "max_join_count" => "Join count limit",
        "max_vector_candidates" => "Vector candidates limit",
        "max_graph_depth" => "Graph depth limit",
        "max_graph_nodes" => "Graph nodes limit",
        "max_graph_edges" => "Graph edges limit",
        "max_import_rows" => "Import rows limit",
        "max_import_batches" => "Import batches limit",
        "max_backup_size" => "Backup size limit",
        "max_restore_size" => "Restore size limit",
        "max_ffi_calls" => "FFI calls limit",
        "max_ffi_memory_mb" => "FFI memory limit",
        "max_ffi_time_ms" => "FFI time limit",
        "max_training_iterations" => "Training iterations limit",
        "max_prediction_batch_size" => "Prediction batch limit",
        "max_embedding_batch_size" => "Embedding batch limit",
        "max_vector_expansions" => "Vector expansions limit",
        "max_pipeline_depth" => "Pipeline depth limit",
        "max_pipeline_stages" => "Pipeline stages limit",
        "max_sort_rows" => "Sort rows limit",
        _ => "Custom limit",
    }
}

pub struct ExecutionHandle {
    engine: Arc<Inner>,
    execution_id: Uuid,
    action: EnforcementAction,
}

impl ExecutionHandle {
    pub fn id(&self) -> Uuid {
        self.execution_id
    }

    pub fn action(&self) -> EnforcementAction {
        self.action
    }

    pub async fn check_memory(&self, current_mb: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs
                .get(&self.execution_id)
                .and_then(|e| e.limits.memory.max_memory_mb)
        };
        let engine = GovernorEngine {
            inner: self.engine.clone(),
        };
        engine
            .check_limit(self.execution_id, "max_memory_mb", current_mb, limit)
            .await
    }

    pub async fn check_steps(&self, steps: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs
                .get(&self.execution_id)
                .and_then(|e| e.limits.cpu.max_execution_steps)
        };
        let engine = GovernorEngine {
            inner: self.engine.clone(),
        };
        engine
            .check_limit(self.execution_id, "max_execution_steps", steps, limit)
            .await
    }

    pub async fn check_cpu_time(&self, cpu_time_ms: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs
                .get(&self.execution_id)
                .and_then(|e| e.limits.cpu.max_cpu_time_ms)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_cpu_time_ms", cpu_time_ms, limit).await
    }

    pub async fn check_query_complexity(&self, complexity: u32) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.query_complexity.max_query_complexity)
                .map(|v| v as u64)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_query_complexity", complexity as u64, limit).await
    }

    pub async fn check_join_count(&self, joins: u32) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.query_complexity.max_join_count)
                .map(|v| v as u64)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_join_count", joins as u64, limit).await
    }

    pub async fn check_sort_rows(&self, rows: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.query_complexity.max_sort_rows)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_sort_rows", rows, limit).await
    }

    pub async fn check_pipeline_depth(&self, depth: u32) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.pipeline.max_pipeline_depth)
                .map(|v| v as u64)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_pipeline_depth", depth as u64, limit).await
    }

    pub async fn check_pipeline_stages(&self, stages: u32) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.pipeline.max_pipeline_stages)
                .map(|v| v as u64)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_pipeline_stages", stages as u64, limit).await
    }

    pub async fn check_ffi_calls(&self, calls: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.ffi.max_ffi_calls)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_ffi_calls", calls, limit).await
    }

    pub async fn check_ffi_memory(&self, memory_mb: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.ffi.max_ffi_memory_mb)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_ffi_memory_mb", memory_mb, limit).await
    }

    pub async fn check_ffi_time(&self, time_ms: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.ffi.max_ffi_time_ms)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_ffi_time_ms", time_ms, limit).await
    }

    pub async fn check_training_iterations(&self, iterations: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.aiml.max_training_iterations)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_training_iterations", iterations, limit).await
    }

    pub async fn check_prediction_batch(&self, batch_size: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.aiml.max_prediction_batch_size)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_prediction_batch_size", batch_size, limit).await
    }

    pub async fn check_embedding_batch(&self, batch_size: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.aiml.max_embedding_batch_size)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_embedding_batch_size", batch_size, limit).await
    }

    pub async fn check_vector_candidates(&self, candidates: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.vector.max_vector_candidates)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_vector_candidates", candidates, limit).await
    }

    pub async fn check_vector_expansions(&self, expansions: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.vector.max_vector_expansions)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_vector_expansions", expansions, limit).await
    }

    pub async fn check_graph_depth(&self, depth: u32) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.graph.max_graph_depth)
                .map(|v| v as u64)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_graph_depth", depth as u64, limit).await
    }

    pub async fn check_graph_nodes(&self, nodes: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.graph.max_graph_nodes)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_graph_nodes", nodes, limit).await
    }

    pub async fn check_graph_edges(&self, edges: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.graph.max_graph_edges)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_graph_edges", edges, limit).await
    }

    pub async fn check_import_rows(&self, rows: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.migration.max_import_rows)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_import_rows", rows, limit).await
    }

    pub async fn check_import_batches(&self, batches: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.migration.max_import_batches)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_import_batches", batches, limit).await
    }

    pub async fn check_backup_size(&self, size_mb: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.backup.max_backup_size)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_backup_size", size_mb, limit).await
    }

    pub async fn check_restore_size(&self, size_mb: u64) -> Result<EnforcementAction, String> {
        let limit = {
            let execs = self.engine.executions.read().await;
            execs.get(&self.execution_id)
                .and_then(|e| e.limits.backup.max_restore_size)
        };
        let engine = GovernorEngine { inner: self.engine.clone() };
        engine.check_limit(self.execution_id, "max_restore_size", size_mb, limit).await
    }

    pub async fn finish(self) {
        self.engine.active_executions.fetch_sub(1, Ordering::Relaxed);
        let mut execs = self.engine.executions.write().await;
        execs.remove(&self.execution_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governor::WorkloadType;

    fn test_engine() -> GovernorEngine {
        GovernorEngine::new(GovernorConfig::default())
    }

    fn test_engine_with_config(config: GovernorConfig) -> GovernorEngine {
        GovernorEngine::new(config)
    }

    #[tokio::test]
    async fn test_start_and_finish_execution() {
        let engine = test_engine();
        let handle = engine
            .start_execution("test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let id = handle.id();
        assert!(engine.get_execution(id).await.is_some());
        handle.finish().await;
        assert!(engine.get_execution(id).await.is_none());
    }

    #[tokio::test]
    async fn test_disabled_engine() {
        let engine = GovernorEngine::new_disabled();
        assert!(!engine.is_enabled().await);
        let handle = engine
            .start_execution("test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_memory(999_999).await.unwrap();
        assert_eq!(result, EnforcementAction::Monitor);
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_memory_limit_blocked() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_memory_mb = Some(100);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_memory(150).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_execution_steps_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_execution_steps = Some(100);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_steps(200).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_no_limit_exceeded() {
        let engine = test_engine();
        let handle = engine
            .start_execution("test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_steps(5).await.unwrap();
        assert_eq!(result, EnforcementAction::Monitor);
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_violations_recorded() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_execution_steps = Some(10);
            policy.action = Some("warn".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("violation-test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let _ = handle.check_steps(20).await;
        let violations = engine.list_violations().await;
        assert!(!violations.is_empty());
        assert_eq!(violations[0].namespace, "violation-test");
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_throttle_action() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_memory_mb = Some(100);
            policy.action = Some("throttle".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("throttle-test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_memory(150).await.unwrap();
        assert_eq!(result, EnforcementAction::Throttle);
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_status() {
        let engine = test_engine();
        let status = engine.status().await;
        assert!(status.enabled);
        assert_eq!(status.active_executions, 0);
    }

    #[tokio::test]
    async fn test_metrics_snapshot() {
        let engine = test_engine();
        let metrics = engine.metrics_snapshot().await;
        assert_eq!(metrics.active_executions, 0);
    }

    #[tokio::test]
    async fn test_list_policies() {
        let engine = test_engine();
        let policies = engine.policies().await;
        assert!(!policies.is_empty());
    }

    #[tokio::test]
    async fn test_cpu_time_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_cpu_time_ms = Some(50);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("cpu-test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_cpu_time(100).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_query_complexity_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_query_complexity = Some(5);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("qc-test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_query_complexity(10).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_join_count_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_join_count = Some(3);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("join-test".to_string(), WorkloadType::Sql, None, None)
            .await;
        let result = handle.check_join_count(5).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_ffi_calls_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_ffi_calls = Some(10);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("ffi-test".to_string(), WorkloadType::Ffi, None, None)
            .await;
        let result = handle.check_ffi_calls(20).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_vector_candidates_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_vector_candidates = Some(100);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("vec-test".to_string(), WorkloadType::VectorSearch, None, None)
            .await;
        let result = handle.check_vector_candidates(200).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_graph_depth_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_graph_depth = Some(5);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("graph-test".to_string(), WorkloadType::GraphTraversal, None, None)
            .await;
        let result = handle.check_graph_depth(10).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_import_rows_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_import_rows = Some(1000);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("import-test".to_string(), WorkloadType::Migration, None, None)
            .await;
        let result = handle.check_import_rows(2000).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_backup_size_limit() {
        let mut config = GovernorConfig::default();
        if let Some(policy) = config.policies.get_mut("default") {
            policy.max_backup_size = Some(500);
            policy.action = Some("block".to_string());
        }
        let engine = test_engine_with_config(config);
        let handle = engine
            .start_execution("backup-test".to_string(), WorkloadType::Backup, None, None)
            .await;
        let result = handle.check_backup_size(1000).await;
        assert!(result.is_err());
        handle.finish().await;
    }

    #[tokio::test]
    async fn test_metrics_aggregates_real_data() {
        let engine = test_engine();
        let handle = engine
            .start_execution("metrics-test".to_string(), WorkloadType::Sql, None, None)
            .await;
        {
            let mut execs = engine.inner.executions.write().await;
            if let Some(ctx) = execs.get_mut(&handle.id()) {
                ctx.usage.memory.allocated_bytes = 42_000_000;
                ctx.usage.cpu.cpu_time_ms = 150;
                ctx.usage.ffi.calls = 7;
            }
        }
        let metrics = engine.metrics_snapshot().await;
        assert_eq!(metrics.active_executions, 1);
        assert_eq!(metrics.memory_usage_bytes, 42_000_000);
        assert_eq!(metrics.cpu_time_ms, 150);
        assert_eq!(metrics.ffi_calls_total, 7);
        handle.finish().await;
    }
}
