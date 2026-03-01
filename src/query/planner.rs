/*
 * PrimusDB Query Planner Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha
 */

//! # Query Planner Module
//!
//! The query planner creates optimized execution plans for UQL queries.
//! It determines which storage engines to use, how to execute joins across
//! engines, and optimizes query execution order.

use crate::query::parser::{JoinClause, JoinType, ParsedQuery, QueryOperation};
use crate::{PrimusDBConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Query planner that creates optimized execution plans
pub struct QueryPlanner {
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

        // Handle cross-engine joins
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
        // Heuristics to determine the best engine for a table
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

    fn plan_select(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        // Create a scan stage for each table
        for (idx, table) in parsed.source_tables.iter().enumerate() {
            let engine = plan
                .engine_routing
                .get(table)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            let mut stage = ExecutionStage {
                stage_id: idx,
                operation: StageOperation::Scan {
                    table: table.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: parsed.columns.clone(),
                limit: parsed.limit,
                offset: parsed.offset,
                dependencies: vec![],
            };

            // First table has no dependencies
            if idx > 0 {
                stage.dependencies.push(idx - 1);
            }

            plan.stages.push(stage);
        }

        // Add aggregation stage if needed
        if !parsed.aggregations.is_empty() || !parsed.group_by.is_empty() {
            plan.stages.push(ExecutionStage {
                stage_id: plan.stages.len(),
                operation: StageOperation::Aggregate {
                    group_by: parsed.group_by.clone(),
                    aggregations: parsed.aggregations.clone(),
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![0],
            });
        }

        // Add sort stage if needed
        if !parsed.order_by.is_empty() {
            plan.stages.push(ExecutionStage {
                stage_id: plan.stages.len(),
                operation: StageOperation::Sort {
                    order_by: parsed.order_by.clone(),
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![plan.stages.len() - 1],
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
                projections: vec![],
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

            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation: StageOperation::Update {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
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

            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation: StageOperation::Delete {
                    table: target.clone(),
                    engine: engine.clone(),
                },
                conditions: parsed.conditions.clone(),
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
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
                },
                _ => StageOperation::Noop,
            };

            plan.stages.push(ExecutionStage {
                stage_id: 0,
                operation,
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
            });
        }
        Ok(())
    }

    fn plan_joins(&self, parsed: &ParsedQuery, plan: &mut QueryPlan) -> Result<()> {
        // Identify cross-engine joins
        for (idx, join) in parsed.joins.iter().enumerate() {
            let left_engine = plan
                .engine_routing
                .get(&parsed.source_tables[0])
                .cloned()
                .unwrap_or_else(|| "relational".to_string());
            let right_engine = plan
                .engine_routing
                .get(&join.table)
                .cloned()
                .unwrap_or_else(|| "relational".to_string());

            let cross_engine = left_engine != right_engine;

            let cross_join = CrossEngineJoin {
                join_id: idx,
                left_table: parsed.source_tables[0].clone(),
                right_table: join.table.clone(),
                left_engine,
                right_engine,
                condition: join.condition.clone(),
                join_type: join.join_type.clone(),
                is_cross_engine: cross_engine,
            };

            plan.cross_engine_joins.push(cross_join);

            // Add join execution stage
            plan.stages.push(ExecutionStage {
                stage_id: plan.stages.len(),
                operation: StageOperation::Join {
                    join_type: join.join_type.clone(),
                    left_table: parsed.source_tables[0].clone(),
                    right_table: join.table.clone(),
                    condition: join.condition.clone(),
                    cross_engine,
                },
                conditions: None,
                projections: vec![],
                limit: None,
                offset: None,
                dependencies: vec![],
            });
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

/// Execution stage in a query plan
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

/// Stage operations
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

/// Cross-engine join information
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
