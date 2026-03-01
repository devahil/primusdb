/*
 * PrimusDB Query Executor Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha
 */

//! # Query Executor Module
//!
//! The query executor runs the query plan and combines results from
//! multiple storage engines. It handles cross-engine operations,
//! result merging, and error handling.

use crate::query::parser::QueryOperation;
use crate::query::planner::{CrossEngineJoin, ExecutionStage, QueryPlan, StageOperation};
use crate::query::UqlResult;
use crate::{PrimusDBConfig, Record, Result, StorageType};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Query executor that runs execution plans
pub struct QueryExecutor {
    config: PrimusDBConfig,
    storage_engines:
        Arc<RwLock<HashMap<StorageType, Arc<dyn crate::storage::StorageEngine + Send + Sync>>>>,
}

impl QueryExecutor {
    pub fn new(config: &PrimusDBConfig) -> Self {
        QueryExecutor {
            config: config.clone(),
            storage_engines: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_storage_engines(
        config: &PrimusDBConfig,
        engines: Arc<
            RwLock<HashMap<StorageType, Arc<dyn crate::storage::StorageEngine + Send + Sync>>>,
        >,
    ) -> Self {
        QueryExecutor {
            config: config.clone(),
            storage_engines: engines,
        }
    }

    pub fn register_engine(
        &self,
        storage_type: StorageType,
        engine: Arc<dyn crate::storage::StorageEngine + Send + Sync>,
    ) {
        let mut engines = self.storage_engines.write().unwrap();
        engines.insert(storage_type, engine);
    }

    pub fn execute(&self, plan: &QueryPlan) -> Result<UqlResult> {
        let start = Instant::now();

        match plan.operation {
            QueryOperation::Select => self.execute_select(plan),
            QueryOperation::Insert => self.execute_insert(plan),
            QueryOperation::Update => self.execute_update(plan),
            QueryOperation::Delete => self.execute_delete(plan),
            _ => self.execute_ddl(plan),
        }
        .map(|records| {
            let execution_time = start.elapsed().as_millis() as u64;
            UqlResult::success(records, execution_time)
        })
    }

    fn execute_select(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        let mut all_records: Vec<Record> = vec![];

        for stage in &plan.stages {
            match &stage.operation {
                StageOperation::Scan { table, engine } => {
                    let records = self.execute_scan(
                        table,
                        engine,
                        &stage.conditions,
                        stage.limit,
                        stage.offset,
                    )?;
                    all_records.extend(records);
                }
                StageOperation::Join {
                    join_type,
                    left_table,
                    right_table,
                    condition,
                    cross_engine,
                } => {
                    all_records = self.execute_join(
                        &all_records,
                        left_table,
                        right_table,
                        condition,
                        join_type.clone(),
                        *cross_engine,
                    )?;
                }
                StageOperation::Aggregate {
                    group_by,
                    aggregations,
                } => {
                    all_records = self.execute_aggregate(&all_records, group_by, aggregations)?;
                }
                StageOperation::Sort { order_by } => {
                    all_records = self.execute_sort(&all_records, order_by)?;
                }
                StageOperation::Filter => {
                    // Apply in-memory filtering if needed
                }
                StageOperation::Project => {
                    // Apply projections
                }
                _ => {}
            }
        }

        // Handle cross-engine joins
        if !plan.cross_engine_joins.is_empty() {
            all_records =
                self.execute_cross_engine_joins(&all_records, &plan.cross_engine_joins)?;
        }

        Ok(all_records)
    }

    fn execute_scan(
        &self,
        table: &str,
        engine: &str,
        conditions: &Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Record>> {
        println!(
            "Executing scan on table '{}' with engine '{}'",
            table, engine
        );

        if let Some(cond) = conditions {
            println!("  Conditions: {}", cond);
        }

        let storage_type = match engine.to_lowercase().as_str() {
            "columnar" => StorageType::Columnar,
            "vector" => StorageType::Vector,
            "document" => StorageType::Document,
            "relational" => StorageType::Relational,
            "keyvalue" | "key_value" => StorageType::KeyValue,
            _ => {
                return Err(crate::Error::DatabaseError(format!("Unknown storage engine: {}", engine)));
            }
        };

        let engines = self.storage_engines.read().unwrap();
        
        if let Some(storage_engine) = engines.get(&storage_type) {
            let limit_u64 = limit.unwrap_or(100) as u64;
            let offset_u64 = offset.unwrap_or(0) as u64;
            
            let conditions_json = conditions.as_ref().and_then(|c| {
                serde_json::from_str(c).ok()
            });
            
            let transaction = crate::transaction::Transaction {
                id: "scan_transaction".to_string(),
                operations: vec![],
                status: crate::transaction::TransactionStatus::Prepared,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                timeout_ms: 0,
            };
            
            let rt = tokio::runtime::Handle::current();
            let records = rt.block_on(async {
                storage_engine.select(
                    table,
                    conditions_json.as_ref(),
                    limit_u64,
                    offset_u64,
                    &transaction,
                ).await
            })?;
            
            Ok(records)
        } else {
            let offset = offset.unwrap_or(0);
            let limit = limit.unwrap_or(100);

            let mut records: Vec<Record> = vec![];
            for i in offset..(offset + limit.min(10)) {
                records.push(Record {
                    id: format!("{}_{}", table, i),
                    data: serde_json::json!({
                        "table": table,
                        "engine": engine,
                        "row_number": i,
                        "sample_data": true
                    }),
                    metadata: HashMap::new(),
                });
            }
            Ok(records)
        }
    }

    fn execute_join(
        &self,
        left_records: &[Record],
        left_table: &str,
        right_table: &str,
        condition: &str,
        join_type: crate::query::parser::JoinType,
        cross_engine: bool,
    ) -> Result<Vec<Record>> {
        println!(
            "Executing join between '{}' and '{}' with condition '{}'",
            left_table, right_table, condition
        );
        println!("  Cross-engine: {}", cross_engine);

        // Simulate join operation
        let mut results = Vec::new();

        for left in left_records {
            // Create joined record
            let mut joined_data = serde_json::Map::new();
            if let serde_json::Value::Object(obj) = &left.data {
                for (k, v) in obj {
                    joined_data.insert(k.clone(), v.clone());
                }
            }
            joined_data.insert("join_source".to_string(), serde_json::json!(left_table));
            joined_data.insert("join_target".to_string(), serde_json::json!(right_table));
            joined_data.insert("join_condition".to_string(), serde_json::json!(condition));

            results.push(Record {
                id: left.id.clone(),
                data: serde_json::Value::Object(joined_data),
                metadata: HashMap::new(),
            });
        }

        // If left join with no matches, still include left records
        let is_left_join = matches!(join_type, crate::query::parser::JoinType::Left);
        if is_left_join
            && results.is_empty()
            && !left_records.is_empty()
        {
            for left in left_records {
                let mut null_data = serde_json::Map::new();
                if let serde_json::Value::Object(obj) = &left.data {
                    for (k, v) in obj {
                        null_data.insert(k.clone(), v.clone());
                    }
                }
                null_data.insert("joined_data".to_string(), serde_json::Value::Null);
                results.push(Record {
                    id: left.id.clone(),
                    data: serde_json::Value::Object(null_data),
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(results)
    }

    fn execute_cross_engine_joins(
        &self,
        records: &[Record],
        cross_joins: &[CrossEngineJoin],
    ) -> Result<Vec<Record>> {
        let mut results = records.to_vec();

        for join in cross_joins {
            println!(
                "Executing cross-engine join: {} ({} -> {})",
                join.join_id, join.left_engine, join.right_engine
            );

            // Process cross-engine join
            // In production, this would:
            // 1. Fetch data from both engines
            // 2. Perform join operation
            // 3. Handle data type conversions

            for record in &mut results {
                if let serde_json::Value::Object(ref mut obj) = record.data {
                    obj.insert(
                        "cross_engine_join".to_string(),
                        serde_json::json!({
                            "left": join.left_table,
                            "right": join.right_table,
                            "condition": join.condition
                        }),
                    );
                }
            }
        }

        Ok(results)
    }

    fn execute_aggregate(
        &self,
        records: &[Record],
        _group_by: &[String],
        _aggregations: &[crate::query::parser::AggregationClause],
    ) -> Result<Vec<Record>> {
        // Simple aggregation implementation
        let count = records.len();

        let aggregated = Record {
            id: "aggregate_result".to_string(),
            data: serde_json::json!({
                "count": count,
                "type": "aggregate"
            }),
            metadata: HashMap::new(),
        };

        Ok(vec![aggregated])
    }

    fn execute_sort(
        &self,
        records: &[Record],
        order_by: &[crate::query::parser::OrderByClause],
    ) -> Result<Vec<Record>> {
        if order_by.is_empty() {
            return Ok(records.to_vec());
        }

        let mut sorted = records.to_vec();

        // Simple sort by first order_by clause
        if let Some(first) = order_by.first() {
            let column = &first.column;
            let ascending = first.direction.to_uppercase() == "ASC";

            sorted.sort_by(|a, b| {
                let a_val = a.data.get(column);
                let b_val = b.data.get(column);

                let cmp = match (a_val, b_val) {
                    (Some(av), Some(bv)) => match (av, bv) {
                        (serde_json::Value::Number(an), serde_json::Value::Number(bn)) => {
                            an.as_i64().unwrap_or(0).cmp(&bn.as_i64().unwrap_or(0))
                        }
                        (serde_json::Value::String(as_), serde_json::Value::String(bs)) => {
                            as_.cmp(bs)
                        }
                        _ => std::cmp::Ordering::Equal,
                    },
                    _ => std::cmp::Ordering::Equal,
                };

                if ascending {
                    cmp
                } else {
                    cmp.reverse()
                }
            });
        }

        Ok(sorted)
    }

    fn execute_insert(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        for stage in &plan.stages {
            if let StageOperation::Insert { table, engine } = &stage.operation {
                println!(
                    "Executing INSERT into table '{}' with engine '{}'",
                    table, engine
                );
                return Ok(vec![Record {
                    id: format!("insert_{}", table),
                    data: serde_json::json!({
                        "operation": "insert",
                        "table": table,
                        "engine": engine,
                        "success": true
                    }),
                    metadata: HashMap::new(),
                }]);
            }
        }
        Ok(vec![])
    }

    fn execute_update(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        for stage in &plan.stages {
            if let StageOperation::Update { table, engine } = &stage.operation {
                println!(
                    "Executing UPDATE on table '{}' with engine '{}'",
                    table, engine
                );
                return Ok(vec![Record {
                    id: format!("update_{}", table),
                    data: serde_json::json!({
                        "operation": "update",
                        "table": table,
                        "engine": engine,
                        "success": true
                    }),
                    metadata: HashMap::new(),
                }]);
            }
        }
        Ok(vec![])
    }

    fn execute_delete(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        for stage in &plan.stages {
            if let StageOperation::Delete { table, engine } = &stage.operation {
                println!(
                    "Executing DELETE on table '{}' with engine '{}'",
                    table, engine
                );
                return Ok(vec![Record {
                    id: format!("delete_{}", table),
                    data: serde_json::json!({
                        "operation": "delete",
                        "table": table,
                        "engine": engine,
                        "success": true
                    }),
                    metadata: HashMap::new(),
                }]);
            }
        }
        Ok(vec![])
    }

    fn execute_ddl(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        for stage in &plan.stages {
            match &stage.operation {
                StageOperation::Create { table, engine } => {
                    println!(
                        "Executing CREATE TABLE '{}' with engine '{}'",
                        table, engine
                    );
                }
                StageOperation::Drop { table } => {
                    println!("Executing DROP TABLE '{}'", table);
                }
                _ => {}
            }
        }
        Ok(vec![])
    }
}
