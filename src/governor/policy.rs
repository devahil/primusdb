//! Resource usage policies for the Governor engine.
//!
//! Policies bundle an execution limit set ([`super::ExecutionLimits`]) with a
//! scope and an enforcement action. [`PolicyManager`] resolves the most
//! specific matching policy for a request and merges broader policies with
//! more specific overrides:
//!
//! ```text
//! +-------------------------+
//! | PolicyManager           |
//! |  policies: name ->      |
//! |  Policy (scope, limits, |
//! |         action)         |
//! +------------+------------+
//!              | resolve(namespace, workload, user, role)
//!              v
//! +-------------------------+
//! | matching policies        |
//! |  global < namespace <    |
//! |  user ... (specificity)  |
//! |  merge: specific limits  |
//! |  override general ones   |
//! +------------+------------+
//!              |
//!              v
//! +-------------------------+
//! | resolved Policy          |
//! +-------------------------+
//! ```

use super::{EnforcementAction, ExecutionLimits, Policy, PolicyConfig, PolicyScope, WorkloadType};
use std::collections::HashMap;

pub struct PolicyManager {
    policies: HashMap<String, Policy>,
    default_policy: String,
}

impl PolicyManager {
    pub fn new() -> Self {
        let default_limits = ExecutionLimits::default();
        let mut policies = HashMap::new();
        policies.insert(
            "default".to_string(),
            Policy {
                name: "default".to_string(),
                scope: PolicyScope::Global,
                limits: default_limits,
                action: EnforcementAction::Monitor,
            },
        );
        Self {
            policies,
            default_policy: "default".to_string(),
        }
    }

    pub fn from_config(config: &HashMap<String, PolicyConfig>, default_name: &str) -> Self {
        let mut policies = HashMap::new();
        for (name, cfg) in config {
            if let Some(policy) = Self::config_to_policy(name, cfg) {
                policies.insert(name.clone(), policy);
            }
        }
        let default = if policies.contains_key(default_name) {
            default_name.to_string()
        } else {
            "default".to_string()
        };
        if !policies.contains_key(&default) {
            policies.insert(
                default.clone(),
                Policy {
                    name: default.clone(),
                    scope: PolicyScope::Global,
                    limits: ExecutionLimits::default(),
                    action: EnforcementAction::Monitor,
                },
            );
        }
        Self {
            policies,
            default_policy: default,
        }
    }

    fn config_to_policy(name: &str, cfg: &PolicyConfig) -> Option<Policy> {
        let scope = Self::parse_scope(&cfg.scope)?;
        let action = match cfg.action.as_deref() {
            Some("block") => EnforcementAction::Block,
            Some("throttle") => EnforcementAction::Throttle,
            Some("warn") => EnforcementAction::Warn,
            _ => EnforcementAction::Monitor,
        };
        let limits = ExecutionLimits {
            cpu: super::CpuLimits {
                max_execution_steps: cfg.max_execution_steps,
                max_cpu_time_ms: cfg.max_cpu_time_ms,
            },
            memory: super::MemoryLimits {
                max_memory_mb: cfg.max_memory_mb,
            },
            query_complexity: super::QueryComplexityLimits {
                max_query_complexity: cfg.max_query_complexity,
                max_join_count: cfg.max_join_count,
                max_sort_rows: cfg.max_sort_rows,
            },
            pipeline: super::PipelineLimits {
                max_pipeline_depth: cfg.max_pipeline_depth,
                max_pipeline_stages: cfg.max_pipeline_stages,
            },
            ffi: super::FfiLimits {
                max_ffi_calls: cfg.max_ffi_calls,
                max_ffi_memory_mb: cfg.max_ffi_memory_mb,
                max_ffi_time_ms: cfg.max_ffi_time_ms,
            },
            aiml: super::AimlLimits {
                max_training_iterations: cfg.max_training_iterations,
                max_prediction_batch_size: cfg.max_prediction_batch_size,
                max_embedding_batch_size: cfg.max_embedding_batch_size,
            },
            vector: super::VectorLimits {
                max_vector_candidates: cfg.max_vector_candidates,
                max_vector_expansions: cfg.max_vector_expansions,
            },
            graph: super::GraphLimits {
                max_graph_depth: cfg.max_graph_depth,
                max_graph_nodes: cfg.max_graph_nodes,
                max_graph_edges: cfg.max_graph_edges,
            },
            migration: super::MigrationLimits {
                max_import_rows: cfg.max_import_rows,
                max_import_batches: cfg.max_import_batches,
            },
            backup: super::BackupLimits {
                max_backup_size: cfg.max_backup_size,
                max_restore_size: cfg.max_restore_size,
            },
        };
        Some(Policy {
            name: name.to_string(),
            scope,
            limits,
            action,
        })
    }

    pub(crate) fn parse_scope(s: &str) -> Option<PolicyScope> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        match parts[0] {
            "global" => Some(PolicyScope::Global),
            "cluster" => parts.get(1).map(|n| PolicyScope::Cluster(n.to_string())),
            "node" => parts.get(1).map(|n| PolicyScope::Node(n.to_string())),
            "namespace" => parts.get(1).map(|n| PolicyScope::Namespace(n.to_string())),
            "database" => parts.get(1).map(|n| PolicyScope::Database(n.to_string())),
            "role" => parts.get(1).map(|n| PolicyScope::Role(n.to_string())),
            "user" => parts.get(1).map(|n| PolicyScope::User(n.to_string())),
            "sql" => Some(PolicyScope::WorkloadType(WorkloadType::Sql)),
            "vector_search" => Some(PolicyScope::WorkloadType(WorkloadType::VectorSearch)),
            "ai_ml" => Some(PolicyScope::WorkloadType(WorkloadType::AIML)),
            "graph_traversal" => Some(PolicyScope::WorkloadType(WorkloadType::GraphTraversal)),
            "cdc_pipeline" => Some(PolicyScope::WorkloadType(WorkloadType::CdcPipeline)),
            "backup" => Some(PolicyScope::WorkloadType(WorkloadType::Backup)),
            "restore" => Some(PolicyScope::WorkloadType(WorkloadType::Restore)),
            "migration" => Some(PolicyScope::WorkloadType(WorkloadType::Migration)),
            "ffi" => Some(PolicyScope::WorkloadType(WorkloadType::Ffi)),
            _ => None,
        }
    }

    fn scope_priority(scope: &PolicyScope) -> u8 {
        match scope {
            PolicyScope::Global => 0,
            PolicyScope::Cluster(_) => 1,
            PolicyScope::Node(_) => 2,
            PolicyScope::Namespace(_) => 3,
            PolicyScope::Database(_) => 4,
            PolicyScope::Role(_) => 5,
            PolicyScope::User(_) => 6,
            PolicyScope::WorkloadType(_) => 7,
        }
    }

    pub fn get_policy(&self, name: &str) -> Option<&Policy> {
        self.policies.get(name)
    }

    pub fn resolve_policy(
        &self,
        namespace: &str,
        workload_type: WorkloadType,
        user: Option<&str>,
        role: Option<&str>,
    ) -> Policy {
        let mut candidates: Vec<&Policy> = Vec::new();

        for policy in self.policies.values() {
            let matches = match &policy.scope {
                PolicyScope::Global => true,
                PolicyScope::Namespace(n) => n == namespace,
                PolicyScope::WorkloadType(wt) => *wt == workload_type,
                PolicyScope::User(u) => user.is_some_and(|u2| u == u2),
                PolicyScope::Role(r) => role.is_some_and(|r2| r == r2),
                PolicyScope::Cluster(_) => false,
                PolicyScope::Node(_) => false,
                PolicyScope::Database(_) => false,
            };
            if matches {
                candidates.push(policy);
            }
        }

        if candidates.is_empty() {
            return self
                .policies
                .get(&self.default_policy)
                .cloned()
                .unwrap_or(Policy {
                    name: "default".to_string(),
                    scope: PolicyScope::Global,
                    limits: ExecutionLimits::default(),
                    action: EnforcementAction::Monitor,
                });
        }

        // Sort by scope specificity (least specific first, most specific last)
        // so that merge_policies applies overrides correctly.
        candidates.sort_by_key(|p| Self::scope_priority(&p.scope));

        let mut merged = candidates[0].clone();
        for candidate in &candidates[1..] {
            merged = Self::merge_policies(&merged, candidate);
        }
        merged
    }

    fn merge_policies(base: &Policy, override_p: &Policy) -> Policy {
        use super::*;
        let merge_opt_u64 = |a: Option<u64>, b: Option<u64>| b.or(a);
        let merge_opt_u32 = |a: Option<u32>, b: Option<u32>| b.or(a);
        Policy {
            name: format!("{}+{}", base.name, override_p.name),
            scope: PolicyScope::Global,
            limits: ExecutionLimits {
                cpu: CpuLimits {
                    max_execution_steps: merge_opt_u64(
                        base.limits.cpu.max_execution_steps,
                        override_p.limits.cpu.max_execution_steps,
                    ),
                    max_cpu_time_ms: merge_opt_u64(
                        base.limits.cpu.max_cpu_time_ms,
                        override_p.limits.cpu.max_cpu_time_ms,
                    ),
                },
                memory: MemoryLimits {
                    max_memory_mb: merge_opt_u64(
                        base.limits.memory.max_memory_mb,
                        override_p.limits.memory.max_memory_mb,
                    ),
                },
                query_complexity: QueryComplexityLimits {
                    max_query_complexity: merge_opt_u32(
                        base.limits.query_complexity.max_query_complexity,
                        override_p.limits.query_complexity.max_query_complexity,
                    ),
                    max_join_count: merge_opt_u32(
                        base.limits.query_complexity.max_join_count,
                        override_p.limits.query_complexity.max_join_count,
                    ),
                    max_sort_rows: merge_opt_u64(
                        base.limits.query_complexity.max_sort_rows,
                        override_p.limits.query_complexity.max_sort_rows,
                    ),
                },
                pipeline: PipelineLimits {
                    max_pipeline_depth: merge_opt_u32(
                        base.limits.pipeline.max_pipeline_depth,
                        override_p.limits.pipeline.max_pipeline_depth,
                    ),
                    max_pipeline_stages: merge_opt_u32(
                        base.limits.pipeline.max_pipeline_stages,
                        override_p.limits.pipeline.max_pipeline_stages,
                    ),
                },
                ffi: FfiLimits {
                    max_ffi_calls: merge_opt_u64(
                        base.limits.ffi.max_ffi_calls,
                        override_p.limits.ffi.max_ffi_calls,
                    ),
                    max_ffi_memory_mb: merge_opt_u64(
                        base.limits.ffi.max_ffi_memory_mb,
                        override_p.limits.ffi.max_ffi_memory_mb,
                    ),
                    max_ffi_time_ms: merge_opt_u64(
                        base.limits.ffi.max_ffi_time_ms,
                        override_p.limits.ffi.max_ffi_time_ms,
                    ),
                },
                aiml: AimlLimits {
                    max_training_iterations: merge_opt_u64(
                        base.limits.aiml.max_training_iterations,
                        override_p.limits.aiml.max_training_iterations,
                    ),
                    max_prediction_batch_size: merge_opt_u64(
                        base.limits.aiml.max_prediction_batch_size,
                        override_p.limits.aiml.max_prediction_batch_size,
                    ),
                    max_embedding_batch_size: merge_opt_u64(
                        base.limits.aiml.max_embedding_batch_size,
                        override_p.limits.aiml.max_embedding_batch_size,
                    ),
                },
                vector: VectorLimits {
                    max_vector_candidates: merge_opt_u64(
                        base.limits.vector.max_vector_candidates,
                        override_p.limits.vector.max_vector_candidates,
                    ),
                    max_vector_expansions: merge_opt_u64(
                        base.limits.vector.max_vector_expansions,
                        override_p.limits.vector.max_vector_expansions,
                    ),
                },
                graph: GraphLimits {
                    max_graph_depth: merge_opt_u32(
                        base.limits.graph.max_graph_depth,
                        override_p.limits.graph.max_graph_depth,
                    ),
                    max_graph_nodes: merge_opt_u64(
                        base.limits.graph.max_graph_nodes,
                        override_p.limits.graph.max_graph_nodes,
                    ),
                    max_graph_edges: merge_opt_u64(
                        base.limits.graph.max_graph_edges,
                        override_p.limits.graph.max_graph_edges,
                    ),
                },
                migration: MigrationLimits {
                    max_import_rows: merge_opt_u64(
                        base.limits.migration.max_import_rows,
                        override_p.limits.migration.max_import_rows,
                    ),
                    max_import_batches: merge_opt_u64(
                        base.limits.migration.max_import_batches,
                        override_p.limits.migration.max_import_batches,
                    ),
                },
                backup: BackupLimits {
                    max_backup_size: merge_opt_u64(
                        base.limits.backup.max_backup_size,
                        override_p.limits.backup.max_backup_size,
                    ),
                    max_restore_size: merge_opt_u64(
                        base.limits.backup.max_restore_size,
                        override_p.limits.backup.max_restore_size,
                    ),
                },
            },
            action: override_p.action,
        }
    }

    pub fn policies(&self) -> impl Iterator<Item = &Policy> {
        self.policies.values()
    }

    pub fn list_policies(&self) -> Vec<Policy> {
        self.policies.values().cloned().collect()
    }

    pub fn add_policy(&mut self, policy: Policy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    pub fn remove_policy(&mut self, name: &str) -> bool {
        if name == self.default_policy {
            return false;
        }
        self.policies.remove(name).is_some()
    }
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governor::WorkloadType;

    #[test]
    fn test_default_policy() {
        let mgr = PolicyManager::new();
        let policy = mgr.get_policy("default");
        assert!(policy.is_some());
        assert_eq!(policy.unwrap().name, "default");
    }

    #[test]
    fn test_resolve_global() {
        let mgr = PolicyManager::new();
        let policy = mgr.resolve_policy("analytics", WorkloadType::Sql, None, None);
        assert_eq!(policy.name, "default");
    }

    #[test]
    fn test_policy_inheritance() {
        let mut policies = HashMap::new();
        policies.insert(
            "base".to_string(),
            PolicyConfig {
                scope: "global".to_string(),
                max_memory_mb: Some(1024),
                max_execution_steps: Some(1_000_000),
                max_cpu_time_ms: Some(60_000),
                max_query_complexity: None,
                max_join_count: None,
                max_sort_rows: None,
                max_pipeline_depth: None,
                max_pipeline_stages: None,
                max_ffi_calls: None,
                max_ffi_memory_mb: None,
                max_ffi_time_ms: None,
                max_training_iterations: None,
                max_prediction_batch_size: None,
                max_embedding_batch_size: None,
                max_vector_candidates: None,
                max_vector_expansions: None,
                max_graph_depth: None,
                max_graph_nodes: None,
                max_graph_edges: None,
                max_import_rows: None,
                max_import_batches: None,
                max_backup_size: None,
                max_restore_size: None,
                action: None,
            },
        );
        policies.insert(
            "override".to_string(),
            PolicyConfig {
                scope: "namespace:analytics".to_string(),
                max_memory_mb: Some(2048),
                max_execution_steps: None,
                max_cpu_time_ms: None,
                max_query_complexity: None,
                max_join_count: None,
                max_sort_rows: None,
                max_pipeline_depth: None,
                max_pipeline_stages: None,
                max_ffi_calls: None,
                max_ffi_memory_mb: None,
                max_ffi_time_ms: None,
                max_training_iterations: None,
                max_prediction_batch_size: None,
                max_embedding_batch_size: None,
                max_vector_candidates: None,
                max_vector_expansions: None,
                max_graph_depth: None,
                max_graph_nodes: None,
                max_graph_edges: None,
                max_import_rows: None,
                max_import_batches: None,
                max_backup_size: None,
                max_restore_size: None,
                action: Some("block".to_string()),
            },
        );
        let mgr = PolicyManager::from_config(&policies, "base");
        let policy = mgr.resolve_policy("analytics", WorkloadType::Sql, None, None);
        assert!(policy.name.contains("base"));
        assert!(policy.name.contains("override"));
        assert_eq!(policy.limits.memory.max_memory_mb, Some(2048));
        assert_eq!(policy.limits.cpu.max_execution_steps, Some(1_000_000));
        assert_eq!(policy.action, EnforcementAction::Block);
    }

    #[test]
    fn test_scope_parsing() {
        assert!(matches!(
            PolicyManager::parse_scope("global"),
            Some(PolicyScope::Global)
        ));
        assert!(matches!(
            PolicyManager::parse_scope("namespace:analytics"),
            Some(PolicyScope::Namespace(n)) if n == "analytics"
        ));
        assert!(matches!(
            PolicyManager::parse_scope("sql"),
            Some(PolicyScope::WorkloadType(WorkloadType::Sql))
        ));
        assert!(PolicyManager::parse_scope("invalid").is_none());
    }

    #[test]
    fn test_from_config() {
        let mut config = HashMap::new();
        config.insert(
            "dev".to_string(),
            PolicyConfig {
                scope: "namespace:dev".to_string(),
                max_memory_mb: Some(4096),
                max_execution_steps: None,
                max_cpu_time_ms: None,
                max_query_complexity: None,
                max_join_count: None,
                max_sort_rows: None,
                max_pipeline_depth: None,
                max_pipeline_stages: None,
                max_ffi_calls: None,
                max_ffi_memory_mb: None,
                max_ffi_time_ms: None,
                max_training_iterations: None,
                max_prediction_batch_size: None,
                max_embedding_batch_size: None,
                max_vector_candidates: None,
                max_vector_expansions: None,
                max_graph_depth: None,
                max_graph_nodes: None,
                max_graph_edges: None,
                max_import_rows: None,
                max_import_batches: None,
                max_backup_size: None,
                max_restore_size: None,
                action: Some("warn".to_string()),
            },
        );
        let mgr = PolicyManager::from_config(&config, "dev");
        assert!(mgr.get_policy("dev").is_some());
        let policy = mgr.resolve_policy("other", WorkloadType::Sql, None, None);
        assert_eq!(policy.name, "dev");
    }
}
