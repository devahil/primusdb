/*
 * PrimusDB Query Planner Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 2.0.0 - Full stage planning, DAG dependencies, multi-join support
 */

use crate::query::parser::{JoinType, ParsedQuery, QueryOperation};
use crate::{PrimusDBConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query planner that creates optimized execution plans
pub struct QueryPlanner {
    #[allow(dead_code)]
    config: PrimusDBConfig,
}

impl QueryPlanner {
    pub fn new(config: &PrimusDBConfig) -> Self {
        QueryPlanner {
            config: config.clone(),
        }
    }

    pub fn create_plan(
        &self,
        parsed: &ParsedQuery,
        storage_engines: &HashMap<String, crate::storage::StorageEngineType>,
    ) -> Result<QueryPlan> {
        let mut plan = QueryPlan {
            operation: parsed.operation.clone(),
            stages: vec![],
            engine_routing: HashMap::new(),
            cross_engine_joins: vec![],
        };

        // Determine which engines to use for each table
        for table in &parsed.source_tables {
            let engine = self.determine_engine(table, storage_engines);
            plan.engine_routing.insert(table.clone(), engine.clone());
        }
        if let Some(target) = &parsed.target_table {
            if !plan.engine_routing.contains_key(target) {
                let engine = self.determine_engine(target, storage_engines);
                plan.engine_routing.insert(target.clone(), engine);
            }
        }

        // Create execution stages
        match parsed.operation {
            QueryOperation::Select => {
                self.plan_select(parsed, &mut plan)?;
            }
            QueryOperation::Insert => {
                self.plan_insert(parsed, &mut plan)?;
            }
            QueryOperation::Update => {
                self.plan_update(parsed, &mut plan)?;
            }
            QueryOperation::Delete => {
                self.plan_delete(parsed, &mut plan)?;
            }
            _ => {
                self.plan_ddl(parsed, &mut plan)?;
            }
        }

        // Handle joins
        if !parsed.joins.is_empty() {
            self.plan_joins(parsed, &mut plan)?;
        }

        Ok(plan)
    }

    fn determine_engine(
        &self,
        table: &str,
        _storage_engines: &HashMap<String, crate::storage::StorageEngineType>,
    ) -> String {
        let table_lower = table.to_lowercase();

        if table_lower.contains("vector")
            || table_lower.contains("embedding")
            || table_lower.contains("similarity")
        {
            "vector".to_string()
        } else if table_lower.contains("document") || table_lower.contains("content") {
            "document".to_string()
        } else if table_lower.contains("keyvalue")
            || table_lower.contains("kv_")
            || table_lower.contains("cache")
            || table_lower.contains("session")
        {
            "keyvalue".to_string()
        } else if table_lower.contains("columnar")
            || table_lower.contains("analytics")
            || table_lower.contains("olap")
        {
            "columnar".to_string()
        } else {
            "relational".to_string()
        }
    }

    fn next_id(&self, plan: &QueryPlan) -> usize {
        plan.stages.len()
    }

    fn plan_select(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        // Scan stage for each table
        for (_idx, table) in parsed.source_tables.iter().enumerate() {
            let engine = plan
                .engine_routing
                .get(table)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            let stage = ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Scan {
                    table: table.clone(),
                    engine: engine.clone(),
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
            };
            plan.stages.push(stage);
        }

        // Filter stage based on WHERE conditions
        if let Some(cond) = &parsed.conditions {
            let deps: Vec<usize> = (0..parsed.source_tables.len()).collect();
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Filter,
                conditions: Some(cond.clone()),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: deps,
            });
        }

        // Project stage if columns are specified (not just *)
        if parsed.columns.len() != 1 || parsed.columns[0] != "*" {
            let deps = vec![self.next_id(plan) - 1];
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Project,
                conditions: None,
                projections: parsed.columns.clone(),
                limit: None,
                offset: None,
                dependencies: if deps[0] == usize::MAX && parsed.conditions.is_none() {
                    (0..parsed.source_tables.len()).collect()
                } else if deps[0] == usize::MAX {
                    vec![self.next_id(plan) - 2]
                } else {
                    deps
                },
            });
        }

        // Aggregation stage
        if !parsed.aggregations.is_empty() || !parsed.group_by.is_empty() {
            let last_id = if plan.stages.is_empty() {
                0
            } else {
                self.next_id(plan) - 1
            };
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Aggregate {
                    group_by: parsed.group_by.clone(),
                    aggregations: parsed.aggregations.clone(),
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![last_id],
            });
        }

        // Having stage (post-aggregation filter)
        if let Some(having) = &parsed.having {
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Filter,
                conditions: Some(having.clone()),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![self.next_id(plan) - 1],
            });
        }

        // Sort stage
        if !parsed.order_by.is_empty() {
            let last_id = if plan.stages.is_empty() {
                0
            } else {
                self.next_id(plan) - 1
            };
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Sort {
                    order_by: parsed.order_by.clone(),
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![last_id],
            });
        }

        // Limit stage (separate from scan)
        if let Some(limit) = parsed.limit {
            let last_id = if plan.stages.is_empty() {
                0
            } else {
                self.next_id(plan) - 1
            };
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Limit { count: limit },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![last_id],
            });
        }

        // Offset stage
        if let Some(offset) = parsed.offset {
            let last_id = if plan.stages.is_empty() {
                0
            } else {
                self.next_id(plan) - 1
            };
            plan.stages.push(ExecutionStage {
                stage_id: self.next_id(plan),
                operation: StageOperation::Offset { count: offset },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![last_id],
            });
        }

        Ok(())
    }

    fn plan_insert(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        if let Some(target) = &parsed.target_table {
            let engine = plan
                .engine_routing
                .get(target)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation: StageOperation::Insert {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: parsed.columns.clone(),
                limit: None,
                offset: None,
                dependencies: vec![],
            });
        }
        Ok(())
    }

    fn plan_update(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        if let Some(target) = &parsed.target_table {
            let engine = plan
                .engine_routing
                .get(target)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            // Scan stage to find records to update
            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation: StageOperation::Scan {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
            });

            // Update stage
            plan.stages.push(ExecutionStage {
                stage_id: 1,
                operation: StageOperation::Update {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: parsed.columns.clone(),
                limit: None,
                offset: None,
                dependencies: vec![0],
            });
        }
        Ok(())
    }

    fn plan_delete(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        if let Some(target) = &parsed.target_table {
            let engine = plan
                .engine_routing
                .get(target)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            // Scan stage
            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation: StageOperation::Scan {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
            });

            // Delete stage
            plan.stages.push(ExecutionStage {
                stage_id: 1,
                operation: StageOperation::Delete {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![0],
            });
        }
        Ok(())
    }

    fn plan_ddl(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        if let Some(target) = &parsed.target_table {
            let engine = plan
                .engine_routing
                .get(target)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            let operation = match parsed.operation {
                QueryOperation::Create => StageOperation::Create {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                QueryOperation::Drop => StageOperation::Drop {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                QueryOperation::Alter => StageOperation::Alter {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                QueryOperation::Truncate => StageOperation::Truncate {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                _ => StageOperation::Noop,
            };

            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation,
                conditions: parsed.conditions.clone(),
                projections: parsed.columns.clone(),
                limit: None,
                offset: None,
                dependencies: vec![],
            });
        }
        Ok(())
    }

    fn plan_joins(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        // Build a mapping from table name to its scan stage ID
        let mut table_to_stage: HashMap<String, usize> = HashMap::new();
        for (idx, table) in parsed.source_tables.iter().enumerate() {
            table_to_stage.insert(table.clone(), idx);
        }

        // Track which source tables each join depends on
        // The first join depends on all source tables; subsequent joins chain from previous
        let mut join_idx = 0;
        for join in &parsed.joins {
            let left_engine = plan
                .engine_routing
                .get(&join.table)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());
            let right_engine = plan
                .engine_routing
                .get(&join.table)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            let cross_engine = {
                // Check if left and right are on different engines
                let left_eng = plan.engine_routing.get(&join.table).map(|s| s.as_str());
                let right_eng = plan.engine_routing.get(&join.table).map(|s| s.as_str());
                left_eng != right_eng
            };

            // Determine left table: use the join target from the join clause,
            // but for left_table, find which source table this join references
            let left_table = if join_idx == 0 && !parsed.source_tables.is_empty() {
                parsed.source_tables[0].clone()
            } else if !parsed.source_tables.is_empty() {
                // For subsequent joins, chain from the previous join
                // by default use the last source table that isn't the join target
                parsed
                    .source_tables
                    .iter()
                    .find(|t| **t != join.table)
                    .cloned()
                    .unwrap_or_else(|| parsed.source_tables[0].clone())
            } else {
                join.table.clone()
            };

            let cross_join = CrossEngineJoin {
                join_id: join_idx,
                left_table: left_table.clone(),
                right_table: join.table.clone(),
                left_engine: left_engine.clone(),
                right_engine: right_engine.clone(),
                condition: join.condition.clone(),
                join_type: join.join_type.clone(),
                is_cross_engine: cross_engine,
            };

            plan.cross_engine_joins.push(cross_join);

            // Dependencies for this join stage
            let mut deps = Vec::new();
            if let Some(&sid) = table_to_stage.get(&left_table) {
                deps.push(sid);
            }
            if let Some(&sid) = table_to_stage.get(&join.table) {
                if !deps.contains(&sid) {
                    deps.push(sid);
                }
            }
            // Also depend on the previous join stage if it exists
            if join_idx > 0 {
                let prev_join_id = parsed.source_tables.len() + join_idx - 1;
                deps.push(prev_join_id);
            }

            plan.stages.push(ExecutionStage {
                stage_id: plan.stages.len(),
                operation: StageOperation::Join {
                    join_type: join.join_type.clone(),
                    left_table: left_table.clone(),
                    right_table: join.table.clone(),
                    condition: join.condition.clone(),
                    cross_engine,
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: deps,
            });

            // Register the joined result as a pseudo-table for chaining
            table_to_stage.insert(format!("__join_{}", join_idx), plan.stages.len() - 1);

            join_idx += 1;
        }

        Ok(())
    }
}

/// Query execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub operation: QueryOperation,
    pub stages: Vec<ExecutionStage>,
    pub engine_routing: HashMap<String, String>,
    pub cross_engine_joins: Vec<CrossEngineJoin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStage {
    pub stage_id: usize,
    pub operation: StageOperation,
    pub conditions: Option<String>,
    pub projections: Vec<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub dependencies: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageOperation {
    Scan {
        table: String,
        engine: String,
    },
    Insert {
        table: String,
        engine: String,
    },
    Update {
        table: String,
        engine: String,
    },
    Delete {
        table: String,
        engine: String,
    },
    Create {
        table: String,
        engine: String,
    },
    Drop {
        table: String,
        engine: String,
    },
    Alter {
        table: String,
        engine: String,
    },
    Truncate {
        table: String,
        engine: String,
    },
    Join {
        join_type: JoinType,
        left_table: String,
        right_table: String,
        condition: String,
        cross_engine: bool,
    },
    Aggregate {
        group_by: Vec<String>,
        aggregations: Vec<crate::query::parser::AggregationClause>,
    },
    Sort {
        order_by: Vec<crate::query::parser::OrderByClause>,
    },
    Filter,
    Project,
    Limit {
        count: usize,
    },
    Offset {
        count: usize,
    },
    Noop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossEngineJoin {
    pub join_id: usize,
    pub left_table: String,
    pub right_table: String,
    pub left_engine: String,
    pub right_engine: String,
    pub condition: String,
    pub join_type: JoinType,
    pub is_cross_engine: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parser::{AggregationClause, AggregationType, JoinClause, OrderByClause};

    fn make_engines() -> HashMap<String, crate::storage::StorageEngineType> {
        let mut m = HashMap::new();
        m.insert(
            "relational".to_string(),
            crate::storage::StorageEngineType::Relational,
        );
        m.insert(
            "vector".to_string(),
            crate::storage::StorageEngineType::Vector,
        );
        m.insert(
            "document".to_string(),
            crate::storage::StorageEngineType::Document,
        );
        m.insert(
            "keyvalue".to_string(),
            crate::storage::StorageEngineType::KeyValue,
        );
        m.insert(
            "columnar".to_string(),
            crate::storage::StorageEngineType::Columnar,
        );
        m
    }

    fn default_config() -> PrimusDBConfig {
        PrimusDBConfig {
            storage: crate::StorageConfig {
                data_dir: "./data".to_string(),
                max_file_size: 1024 * 1024,
                compression: crate::CompressionType::Lz4,
                cache_size: 1024,
            },
            network: crate::NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
            },
            security: crate::SecurityConfig {
                encryption_enabled: false,
                key_rotation_interval: 86400,
                auth_required: false,
            },
            cluster: crate::ClusterConfig {
                enabled: false,
                node_id: "test".to_string(),
                discovery_servers: vec![],
            },
            namespaces: Default::default(),
        }
    }

    #[test]
    fn test_plan_simple_select() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["users".to_string()],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        assert_eq!(plan.stages.len(), 1);
        assert!(matches!(
            plan.stages[0].operation,
            StageOperation::Scan { .. }
        ));
    }

    #[test]
    fn test_plan_select_with_where() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["users".to_string()],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: Some("age > 25".to_string()),
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        // Scan + Filter
        assert_eq!(plan.stages.len(), 2);
        assert!(matches!(plan.stages[1].operation, StageOperation::Filter));
        assert_eq!(plan.stages[1].conditions, Some("age > 25".to_string()));
    }

    #[test]
    fn test_plan_select_with_projection() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["users".to_string()],
            target_table: None,
            columns: vec!["id".to_string(), "name".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        assert_eq!(plan.stages.len(), 2);
        assert!(matches!(plan.stages[1].operation, StageOperation::Project));
        assert_eq!(plan.stages[1].projections, vec!["id", "name"]);
    }

    #[test]
    fn test_plan_select_with_order_limit_offset() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["users".to_string()],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![OrderByClause {
                column: "name".to_string(),
                direction: "ASC".to_string(),
            }],
            group_by: vec![],
            aggregations: vec![],
            limit: Some(10),
            offset: Some(5),
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        // Scan + Sort + Limit + Offset = 4
        assert_eq!(plan.stages.len(), 4);
        assert!(matches!(
            plan.stages[1].operation,
            StageOperation::Sort { .. }
        ));
        assert!(matches!(
            plan.stages[2].operation,
            StageOperation::Limit { .. }
        ));
        assert!(matches!(
            plan.stages[3].operation,
            StageOperation::Offset { .. }
        ));
    }

    #[test]
    fn test_plan_select_with_aggregation() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["users".to_string()],
            target_table: None,
            columns: vec!["COUNT(*)".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![AggregationClause {
                agg_type: AggregationType::Count,
                column: "*".to_string(),
                alias: None,
            }],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        assert!(plan
            .stages
            .iter()
            .any(|s| matches!(s.operation, StageOperation::Aggregate { .. })));
    }

    #[test]
    fn test_plan_engine_routing() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["vectors".to_string(), "users".to_string()],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        assert_eq!(plan.engine_routing.get("vectors").unwrap(), "vector");
        assert_eq!(plan.engine_routing.get("users").unwrap(), "relational");
    }

    #[test]
    fn test_plan_insert() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Insert,
            source_tables: vec![],
            target_table: Some("users".to_string()),
            columns: vec!["id".to_string(), "name".to_string()],
            conditions: Some("1, 'Alice'".to_string()),
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        assert_eq!(plan.stages.len(), 1);
        assert!(matches!(
            plan.stages[0].operation,
            StageOperation::Insert { .. }
        ));
    }

    #[test]
    fn test_plan_update_with_scan() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Update,
            source_tables: vec![],
            target_table: Some("users".to_string()),
            columns: vec![],
            conditions: Some("id = 1".to_string()),
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        // Scan + Update = 2 stages
        assert_eq!(plan.stages.len(), 2);
        assert!(matches!(
            plan.stages[0].operation,
            StageOperation::Scan { .. }
        ));
        assert!(matches!(
            plan.stages[1].operation,
            StageOperation::Update { .. }
        ));
        assert_eq!(plan.stages[1].dependencies, vec![0]);
    }

    #[test]
    fn test_plan_ddl() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Create,
            source_tables: vec![],
            target_table: Some("users".to_string()),
            columns: vec![],
            conditions: None,
            joins: vec![],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        assert_eq!(plan.stages.len(), 1);
        assert!(matches!(
            plan.stages[0].operation,
            StageOperation::Create { .. }
        ));
    }

    #[test]
    fn test_plan_join() {
        let parsed = ParsedQuery {
            operation: QueryOperation::Select,
            source_tables: vec!["users".to_string()],
            target_table: None,
            columns: vec!["*".to_string()],
            conditions: None,
            joins: vec![JoinClause {
                join_type: JoinType::Inner,
                table: "orders".to_string(),
                condition: "users.id = orders.user_id".to_string(),
                engine_hint: None,
            }],
            order_by: vec![],
            group_by: vec![],
            aggregations: vec![],
            limit: None,
            offset: None,
            set_operations: vec![],
            nested_queries: vec![],
            distinct: false,
            having: None,
            ctes: vec![],
            window_functions: vec![],
        };

        let planner = QueryPlanner::new(&default_config());
        let plan = planner.create_plan(&parsed, &make_engines()).unwrap();
        // Scan + Join + cross_engine_joins
        assert!(plan
            .stages
            .iter()
            .any(|s| matches!(s.operation, StageOperation::Join { .. })));
        assert_eq!(plan.cross_engine_joins.len(), 1);
    }
}
