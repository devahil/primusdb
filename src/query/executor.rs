/*
 * PrimusDB Query Executor Module
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 2.0.0 - Full stage execution, real engine calls, filter/project/limit/offset
 */

use crate::query::parser::{
    AggregationClause, AggregationType, JoinType, OrderByClause, QueryOperation,
};
use crate::query::planner::{CrossEngineJoin, ExecutionStage, QueryPlan, StageOperation};
use crate::query::UqlResult;
use crate::{PrimusDBConfig, Record, Result, StorageType};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

fn block_on_sync<F: std::future::Future>(f: F) -> F::Output {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(f)
    })
}

fn sql_str_to_json_condition(sql: &str) -> Option<serde_json::Value> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(sql) {
        return Some(v);
    }
    let s = sql.trim();

    let or_parts: Vec<&str> = {
        let mut parts = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            match chars[i] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ if depth == 0 && i + 3 < chars.len() => {
                    let slice: String = chars[i..i + 4].iter().collect();
                    if slice.to_lowercase() == " or " {
                        parts.push(&s[start..i]);
                        start = i + 4;
                    }
                }
                _ => {}
            }
        }
        parts.push(&s[start..]);
        parts
    };

    if or_parts.len() > 1 {
        let mut conditions = Vec::new();
        for part in &or_parts {
            if let Some(json) = sql_str_to_json_condition(part.trim()) {
                conditions.push(json);
            }
        }
        if conditions.len() == 1 {
            return Some(conditions.swap_remove(0));
        }
        if !conditions.is_empty() {
            let mut combined = conditions.swap_remove(0);
            for c in conditions {
                combined = serde_json::json!({"op": "or", "left": combined, "right": c});
            }
            return Some(combined);
        }
        return None;
    }

    let and_str = s.trim();
    let and_parts: Vec<&str> = {
        let mut parts = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        let chars: Vec<char> = and_str.chars().collect();
        for i in 0..chars.len() {
            match chars[i] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ if depth == 0 && i + 4 < chars.len() => {
                    let slice: String = chars[i..i + 5].iter().collect();
                    if slice.to_lowercase() == " and " {
                        parts.push(&and_str[start..i]);
                        start = i + 5;
                    }
                }
                _ => {}
            }
        }
        parts.push(&and_str[start..]);
        parts
    };

    if and_parts.len() > 1 {
        let mut conditions = Vec::new();
        for part in &and_parts {
            if let Some(json) = sql_str_to_json_condition(part.trim()) {
                conditions.push(json);
            }
        }
        if conditions.len() == 1 {
            return Some(conditions.swap_remove(0));
        }
        if !conditions.is_empty() {
            let mut combined = conditions.swap_remove(0);
            for c in conditions {
                combined = serde_json::json!({"op": "and", "left": combined, "right": c});
            }
            return Some(combined);
        }
        return None;
    }

    let part = and_str.trim();

    if let Some(pos) = part.to_uppercase().find(" IS NULL") {
        let field = part[..pos].trim();
        return Some(serde_json::json!({"op": "eq", "field": field, "value": null}));
    }

    if let Some(pos) = part.to_uppercase().find(" IS NOT NULL") {
        let field = part[..pos].trim();
        return Some(serde_json::json!({"op": "ne", "field": field, "value": null}));
    }

    if let Some(pos) = part.to_uppercase().find(" IN (") {
        let field = part[..pos].trim();
        let list_start = pos + 5;
        let list_end = part.rfind(')').unwrap_or(part.len());
        let list_str = &part[list_start..list_end];
        let values: Vec<serde_json::Value> = list_str
            .split(',')
            .map(|s| {
                let v = s.trim().trim_matches('\'');
                if let Ok(n) = v.parse::<i64>() {
                    serde_json::json!(n)
                } else if let Ok(f) = v.parse::<f64>() {
                    serde_json::json!(f)
                } else {
                    serde_json::json!(v)
                }
            })
            .collect();
        return Some(serde_json::json!({"op": "in", "field": field, "values": values}));
    }

    if let Some(pos) = part.to_uppercase().find(" BETWEEN ") {
        let field = part[..pos].trim();
        let rest = &part[pos + 9..];
        let range_parts: Vec<&str> = rest.splitn(3, " AND ").collect();
        if range_parts.len() == 3 {
            let low_val = range_parts[0].trim().trim_matches('\'');
            let high_val = range_parts[1].trim().trim_matches('\'');
            let low: serde_json::Value = if let Ok(n) = low_val.parse::<i64>() {
                serde_json::json!(n)
            } else if let Ok(f) = low_val.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::json!(low_val)
            };
            let high: serde_json::Value = if let Ok(n) = high_val.parse::<i64>() {
                serde_json::json!(n)
            } else if let Ok(f) = high_val.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::json!(high_val)
            };
            return Some(serde_json::json!({
                "op": "and",
                "left": {"op": "or", "left": {"op": "gt", "field": field, "value": low}, "right": {"op": "eq", "field": field, "value": low}},
                "right": {"op": "or", "left": {"op": "lt", "field": field, "value": high}, "right": {"op": "eq", "field": field, "value": high}}
            }));
        }
        return None;
    }

    if let Some(pos) = part.to_uppercase().find(" LIKE ") {
        let field = part[..pos].trim();
        let pattern = part[pos + 6..].trim().trim_matches('\'');
        return Some(serde_json::json!({"op": "like", "field": field, "pattern": pattern}));
    }

    let ops = ["!=", ">=", "<=", "=", ">", "<"];
    for &op in &ops {
        if let Some(pos) = part.find(op) {
            let field = part[..pos].trim();
            let val_str = part[pos + op.len()..].trim().trim_matches('\'');
            let value: serde_json::Value = if let Ok(n) = val_str.parse::<i64>() {
                serde_json::json!(n)
            } else if let Ok(f) = val_str.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::json!(val_str)
            };
            return match op {
                "=" => Some(serde_json::json!({"op": "eq", "field": field, "value": value})),
                "!=" => Some(serde_json::json!({"op": "ne", "field": field, "value": value})),
                ">" => Some(serde_json::json!({"op": "gt", "field": field, "value": value})),
                "<" => Some(serde_json::json!({"op": "lt", "field": field, "value": value})),
                ">=" => Some(serde_json::json!({
                    "op": "or",
                    "left": {"op": "gt", "field": field, "value": value},
                    "right": {"op": "eq", "field": field, "value": value}
                })),
                "<=" => Some(serde_json::json!({
                    "op": "or",
                    "left": {"op": "lt", "field": field, "value": value},
                    "right": {"op": "eq", "field": field, "value": value}
                })),
                _ => None,
            };
        }
    }

    None
}

pub struct QueryExecutor {
    #[allow(dead_code)]
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
        if let Ok(mut engines) = self.storage_engines.write() {
            engines.insert(storage_type, engine);
        }
    }

    pub fn execute(&self, plan: &QueryPlan) -> Result<UqlResult> {
        let start = Instant::now();

        let records = match plan.operation {
            QueryOperation::Select => self.execute_select(plan),
            QueryOperation::Insert => self.execute_insert(plan),
            QueryOperation::Update => self.execute_update(plan),
            QueryOperation::Delete => self.execute_delete(plan),
            _ => self.execute_ddl(plan),
        }?;

        let execution_time = start.elapsed().as_millis() as u64;

        let affected_rows = match plan.operation {
            QueryOperation::Insert => records.len() as u64,
            QueryOperation::Update => records
                .first()
                .and_then(|r| r.data.get("rows_affected"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            QueryOperation::Delete => records
                .first()
                .and_then(|r| r.data.get("rows_deleted"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            _ => 0,
        };

        if affected_rows > 0 {
            Ok(UqlResult::mutation_success(
                records,
                affected_rows,
                execution_time,
            ))
        } else {
            Ok(UqlResult::success(records, execution_time))
        }
    }

    fn engine_to_storage_type(engine: &str) -> Result<StorageType> {
        match engine.to_lowercase().as_str() {
            "columnar" => Ok(StorageType::Columnar),
            "vector" => Ok(StorageType::Vector),
            "document" => Ok(StorageType::Document),
            "relational" => Ok(StorageType::Relational),
            "keyvalue" | "key_value" => Ok(StorageType::KeyValue),
            _ => Err(crate::Error::DatabaseError(format!(
                "Unknown storage engine: {}",
                engine
            ))),
        }
    }

    fn execute_select(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        // Execute stages respecting dependencies (simple topological order)
        let stage_count = plan.stages.len();
        let mut stage_results: Vec<Vec<Record>> = vec![vec![]; stage_count];

        for stage_idx in 0..stage_count {
            let stage = &plan.stages[stage_idx];
            let input = if stage.dependencies.is_empty() {
                vec![]
            } else {
                let mut combined = vec![];
                for &dep in &stage.dependencies {
                    if dep < stage_results.len() {
                        combined.extend(stage_results[dep].clone());
                    }
                }
                combined
            };

            let result = self.execute_stage(stage, &input, plan)?;
            stage_results[stage_idx] = result;
        }

        // Return the result from the last stage (or empty)
        Ok(stage_results.into_iter().last().unwrap_or_default())
    }

    fn execute_stage(
        &self,
        stage: &ExecutionStage,
        input: &[Record],
        _plan: &QueryPlan,
    ) -> Result<Vec<Record>> {
        match &stage.operation {
            StageOperation::Scan { table, engine } => {
                self.execute_scan(table, engine, &stage.conditions, stage.limit, stage.offset)
            }
            StageOperation::Join {
                join_type,
                left_table,
                right_table,
                condition,
                cross_engine,
            } => self.execute_join(
                input,
                left_table,
                right_table,
                condition,
                join_type.clone(),
                *cross_engine,
            ),
            StageOperation::Aggregate {
                group_by,
                aggregations,
            } => self.execute_aggregate(input, group_by, aggregations),
            StageOperation::Sort { order_by } => self.execute_sort(input, order_by),
            StageOperation::Filter => self.execute_filter(input, &stage.conditions),
            StageOperation::Project => self.execute_project(input, &stage.projections),
            StageOperation::Limit { count } => Ok(input.iter().take(*count).cloned().collect()),
            StageOperation::Offset { count } => Ok(input.iter().skip(*count).cloned().collect()),
            StageOperation::Insert { table, engine } => {
                self.execute_insert_op(table, engine, &stage.conditions, &stage.projections)
            }
            StageOperation::Update { table, engine } => {
                self.execute_update_op(table, engine, &stage.conditions, &stage.projections)
            }
            StageOperation::Delete { table, engine } => {
                self.execute_delete_op(table, engine, &stage.conditions)
            }
            StageOperation::Create { table, engine } => self.execute_create_table(table, engine),
            StageOperation::Drop { table, engine } => self.execute_drop_table(table, engine),
            StageOperation::Alter { table, engine } => {
                self.execute_alter_table(table, engine, &stage.conditions)
            }
            StageOperation::Truncate { table, engine } => self.execute_truncate(table, engine),
            StageOperation::Noop => Ok(vec![]),
        }
    }

    fn execute_scan(
        &self,
        table: &str,
        engine: &str,
        conditions: &Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            let limit_u64 = limit.unwrap_or(100) as u64;
            let offset_u64 = offset.unwrap_or(0) as u64;

            let conditions_json = conditions
                .as_ref()
                .and_then(|c| sql_str_to_json_condition(c));

            let transaction = crate::transaction::Transaction {
                id: format!("scan_{}", table),
                operations: vec![],
                status: crate::transaction::TransactionStatus::Prepared,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                timeout_ms: 0,
                ..Default::default()
            };

            let records = block_on_sync(async {
                storage_engine
                    .select(
                        table,
                        conditions_json.as_ref(),
                        limit_u64,
                        offset_u64,
                        &transaction,
                    )
                    .await
            })?;

            Ok(records)
        } else {
            // Engine not registered - return error
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not registered. Available: {:?}",
                engine,
                engines.keys().collect::<Vec<_>>()
            )))
        }
    }

    fn execute_filter(
        &self,
        records: &[Record],
        conditions: &Option<String>,
    ) -> Result<Vec<Record>> {
        let Some(cond_str) = conditions else {
            return Ok(records.to_vec());
        };

        // Simple condition evaluation - supports basic comparison expressions
        // Format: "column OP value [AND/OR column OP value ...]"
        let cond_lower = cond_str.to_lowercase();
        let or_parts: Vec<&str> = if cond_lower.contains(" or ") {
            // Split on OR but respect parentheses
            let mut parts = Vec::new();
            let mut depth = 0;
            let mut start = 0;
            let chars: Vec<char> = cond_str.chars().collect();
            for i in 0..chars.len() {
                match chars[i] {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ if depth == 0 && i + 3 < chars.len() => {
                        let slice: String = chars[i..i + 4].iter().collect();
                        if slice.to_lowercase() == " or " {
                            parts.push(&cond_str[start..i]);
                            start = i + 4;
                        }
                    }
                    _ => {}
                }
            }
            parts.push(&cond_str[start..]);
            parts
        } else {
            vec![cond_str]
        };

        // Filter records based on conditions
        let mut result: Vec<Record> = Vec::new();

        for record in records {
            for or_part in &or_parts {
                let and_parts: Vec<&str> = or_part
                    .split(" AND ")
                    .flat_map(|s| {
                        let s_lower = s.to_lowercase();
                        if s_lower.contains(" and ") && !s_lower.starts_with('(') {
                            // Manual split at AND
                            let mut parts = Vec::new();
                            let mut depth = 0;
                            let mut start = 0;
                            let chars: Vec<char> = s.chars().collect();
                            for i in 0..chars.len() {
                                match chars[i] {
                                    '(' => depth += 1,
                                    ')' => depth -= 1,
                                    _ if depth == 0 && i + 4 < chars.len() => {
                                        let slice: String = chars[i..i + 5].iter().collect();
                                        if slice.to_lowercase() == " and " {
                                            parts.push(&s[start..i]);
                                            start = i + 5;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            parts.push(&s[start..]);
                            parts
                        } else {
                            vec![s]
                        }
                    })
                    .collect();

                let all_match = and_parts.iter().all(|part| {
                    let part = part.trim();
                    if part.starts_with('(') && part.ends_with(')') {
                        // Recursive for parenthesized
                        return self
                            .evaluate_simple_condition(record, &part[1..part.len() - 1])
                            .unwrap_or(false);
                    }
                    self.evaluate_simple_condition(record, part)
                        .unwrap_or(false)
                });

                if all_match {
                    result.push(record.clone());
                    break;
                }
            }
        }

        Ok(result)
    }

    fn evaluate_simple_condition(&self, record: &Record, condition: &str) -> Result<bool> {
        let condition = condition.trim();

        // Handle NOT prefix
        if condition.to_uppercase().starts_with("NOT ") {
            let inner = &condition[4..].trim();
            return self.evaluate_simple_condition(record, inner).map(|v| !v);
        }

        // Handle IS NULL / IS NOT NULL
        if condition.to_uppercase().contains(" IS NULL") {
            let col = condition.split_whitespace().next().unwrap_or("");
            let val = record.data.get(col);
            return Ok(val.is_none() || val == Some(&serde_json::Value::Null));
        }
        if condition.to_uppercase().contains(" IS NOT NULL") {
            let col = condition.split_whitespace().next().unwrap_or("");
            let val = record.data.get(col);
            return Ok(val.is_some() && val != Some(&serde_json::Value::Null));
        }

        // Handle LIKE
        if let Some(like_pos) = condition.to_uppercase().find(" LIKE ") {
            let col = condition[..like_pos].trim();
            let pattern = condition[like_pos + 6..].trim().trim_matches('\'');
            if let Some(val) = record.data.get(col).and_then(|v| v.as_str()) {
                let pattern_re = pattern.replace('%', ".*").replace('_', ".");
                if let Ok(re) = regex::Regex::new(&format!("^{}$", pattern_re)) {
                    return Ok(re.is_match(val));
                }
                return Ok(val.contains(pattern));
            }
            return Ok(false);
        }

        // Handle IN (...)
        if let Some(in_pos) = condition.to_uppercase().find(" IN (") {
            let col = condition[..in_pos].trim();
            let list_str = &condition[in_pos + 5..condition.rfind(')').unwrap_or(condition.len())];
            let values: Vec<&str> = list_str
                .split(',')
                .map(|s| s.trim().trim_matches('\''))
                .collect();
            if let Some(val) = record.data.get(col) {
                let val_str = match val {
                    serde_json::Value::String(s) => s.as_str(),
                    serde_json::Value::Number(n) => {
                        return Ok(values.contains(&n.to_string().as_str()))
                    }
                    _ => return Ok(false),
                };
                return Ok(values.contains(&val_str));
            }
            return Ok(false);
        }

        // Handle BETWEEN
        if let Some(between_pos) = condition.to_uppercase().find(" BETWEEN ") {
            let col = condition[..between_pos].trim();
            let rest = &condition[between_pos + 9..];
            let parts: Vec<&str> = rest.splitn(3, " AND ").collect();
            if parts.len() == 3 {
                let low = parts[0].trim().parse::<f64>().unwrap_or(f64::MIN);
                let high = parts[1].trim().parse::<f64>().unwrap_or(f64::MAX);
                if let Some(val) = record.data.get(col).and_then(|v| v.as_f64()) {
                    return Ok(val >= low && val <= high);
                }
            }
            return Ok(false);
        }

        // Handle comparison operators: =, !=, >=, <=, >, <
        let ops = ["!=", ">=", "<=", "=", ">", "<"];
        for &op in &ops {
            if let Some(pos) = condition.find(op) {
                let col = condition[..pos].trim();
                let val_str = condition[pos + op.len()..].trim().trim_matches('\'');
                let col_val = record.data.get(col);

                return Ok(match (col_val, op) {
                    (Some(serde_json::Value::Null), _) => false,
                    (None, _) => false,
                    (Some(actual), "=") => Self::values_equal(actual, val_str),
                    (Some(actual), "!=") => !Self::values_equal(actual, val_str),
                    (Some(actual), ">") => Self::value_greater(actual, val_str),
                    (Some(actual), "<") => Self::value_less(actual, val_str),
                    (Some(actual), ">=") => Self::value_greater_or_equal(actual, val_str),
                    (Some(actual), "<=") => Self::value_less_or_equal(actual, val_str),
                    _ => false,
                });
            }
        }

        // Bare column name (truthy check)
        if let Some(val) = record.data.get(condition) {
            return Ok(val != &serde_json::Value::Null && val != &serde_json::Value::Bool(false));
        }

        Ok(false)
    }

    fn values_equal(actual: &serde_json::Value, expected: &str) -> bool {
        match actual {
            serde_json::Value::String(s) => s == expected,
            serde_json::Value::Number(n) => n.to_string() == expected,
            serde_json::Value::Bool(b) => expected.eq_ignore_ascii_case(&b.to_string()),
            _ => false,
        }
    }

    fn value_greater(actual: &serde_json::Value, expected: &str) -> bool {
        match actual {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(|f| f > expected.parse::<f64>().unwrap_or(f64::MIN))
                .unwrap_or(false),
            serde_json::Value::String(s) => s.as_str() > expected,
            _ => false,
        }
    }

    fn value_less(actual: &serde_json::Value, expected: &str) -> bool {
        match actual {
            serde_json::Value::Number(n) => n
                .as_f64()
                .map(|f| f < expected.parse::<f64>().unwrap_or(f64::MAX))
                .unwrap_or(false),
            serde_json::Value::String(s) => s.as_str() < expected,
            _ => false,
        }
    }

    fn value_greater_or_equal(actual: &serde_json::Value, expected: &str) -> bool {
        Self::value_greater(actual, expected) || Self::values_equal(actual, expected)
    }

    fn value_less_or_equal(actual: &serde_json::Value, expected: &str) -> bool {
        Self::value_less(actual, expected) || Self::values_equal(actual, expected)
    }

    fn execute_project(&self, records: &[Record], columns: &[String]) -> Result<Vec<Record>> {
        if columns.is_empty() || (columns.len() == 1 && columns[0] == "*") {
            return Ok(records.to_vec());
        }

        let projected: Vec<Record> = records
            .iter()
            .map(|record| {
                let mut projected_data = serde_json::Map::new();
                if let serde_json::Value::Object(obj) = &record.data {
                    for col in columns {
                        let col_name = if let Some(dot_pos) = col.find('.') {
                            &col[dot_pos + 1..]
                        } else {
                            col.as_str()
                        };
                        if let Some(val) = obj.get(col_name) {
                            projected_data.insert(col_name.to_string(), val.clone());
                        }
                    }
                }
                Record {
                    id: record.id.clone(),
                    data: serde_json::Value::Object(projected_data),
                    metadata: record.metadata.clone(),
                }
            })
            .collect();

        Ok(projected)
    }

    fn execute_join(
        &self,
        left_records: &[Record],
        left_table: &str,
        right_table: &str,
        condition: &str,
        join_type: JoinType,
        _cross_engine: bool,
    ) -> Result<Vec<Record>> {
        // For now, simulate join. In production, this would fetch from the right engine.
        let mut results = Vec::new();

        for left in left_records {
            let mut joined_data = serde_json::Map::new();
            if let serde_json::Value::Object(obj) = &left.data {
                for (k, v) in obj {
                    joined_data.insert(k.clone(), v.clone());
                }
            }

            // Parse condition to extract join columns
            let (left_col, _right_col) =
                self.parse_join_condition(condition, left_table, right_table);

            let matched = if let Some(lv) = left_col.and_then(|c| left.data.get(c)) {
                // Simulate right-side fetch
                let right_data = serde_json::json!({
                    "right_table": right_table,
                    "join_key": lv,
                    "matched": true
                });
                joined_data.insert(format!("{}_data", right_table), right_data);
                true
            } else {
                false
            };

            if matched
                || join_type == JoinType::Left
                || join_type == JoinType::Full
                || join_type == JoinType::Cross
            {
                results.push(Record {
                    id: left.id.clone(),
                    data: serde_json::Value::Object(joined_data),
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(results)
    }

    fn parse_join_condition<'a>(
        &self,
        condition: &'a str,
        _left_table: &str,
        _right_table: &str,
    ) -> (Option<&'a str>, Option<&'a str>) {
        // Expected format: "left_table.column = right_table.column"
        let eq_parts: Vec<&str> = condition.split('=').collect();
        if eq_parts.len() == 2 {
            let left_part = eq_parts[0].trim();
            let right_part = eq_parts[1].trim();

            let left_col = left_part.split('.').nth(1).or_else(|| Some(left_part));
            let right_col = right_part.split('.').nth(1).or_else(|| Some(right_part));

            (left_col, right_col)
        } else {
            (None, None)
        }
    }

    #[allow(dead_code)]
    fn execute_cross_engine_joins(
        &self,
        records: &[Record],
        cross_joins: &[CrossEngineJoin],
    ) -> Result<Vec<Record>> {
        let mut results = records.to_vec();
        for join in cross_joins {
            // Fetch right-side data from the appropriate engine
            if let Ok(storage_type) = Self::engine_to_storage_type(&join.right_engine) {
                if let Ok(engines) = self.storage_engines.read() {
                    if let Some(engine) = engines.get(&storage_type) {
                        let transaction = crate::transaction::Transaction {
                            id: format!("cross_join_{}", join.join_id),
                            operations: vec![],
                            status: crate::transaction::TransactionStatus::Prepared,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                            timeout_ms: 0,
                            ..Default::default()
                        };
                        if let Ok(right_records) = block_on_sync(async {
                            engine
                                .select(&join.right_table, None, 1000, 0, &transaction)
                                .await
                        }) {
                            // Build lookup from right table
                            let right_col = join
                                .condition
                                .split('=')
                                .nth(1)
                                .unwrap_or("")
                                .trim()
                                .split('.')
                                .nth(1)
                                .unwrap_or("id");
                            let right_map: HashMap<String, &Record> = right_records
                                .iter()
                                .filter_map(|r| {
                                    r.data
                                        .get(right_col)
                                        .and_then(|v| v.as_str())
                                        .map(|k| (k.to_string(), r))
                                })
                                .collect();

                            for record in &mut results {
                                if let serde_json::Value::Object(ref mut obj) = record.data {
                                    let left_col = join
                                        .condition
                                        .split('=')
                                        .nth(0)
                                        .unwrap_or("")
                                        .trim()
                                        .split('.')
                                        .nth(1)
                                        .unwrap_or("id");
                                    if let Some(join_key) =
                                        obj.get(left_col).and_then(|v| v.as_str())
                                    {
                                        if let Some(right_rec) = right_map.get(join_key) {
                                            if let serde_json::Value::Object(right_obj) =
                                                &right_rec.data
                                            {
                                                for (k, v) in right_obj {
                                                    obj.insert(
                                                        format!("{}_{}", join.right_table, k),
                                                        v.clone(),
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(results)
    }

    fn execute_aggregate(
        &self,
        records: &[Record],
        group_by: &[String],
        aggregations: &[AggregationClause],
    ) -> Result<Vec<Record>> {
        if records.is_empty() {
            return Ok(vec![]);
        }

        if group_by.is_empty() {
            // No grouping - single result
            let mut data = serde_json::Map::new();
            for agg in aggregations {
                let val = self.compute_aggregation(records, agg);
                let key = agg
                    .alias
                    .clone()
                    .unwrap_or_else(|| format!("{:?}({})", agg.agg_type, agg.column));
                data.insert(key, val);
            }
            return Ok(vec![Record {
                id: "aggregate".to_string(),
                data: serde_json::Value::Object(data),
                metadata: HashMap::new(),
            }]);
        }

        // Group by specified columns
        let mut groups: HashMap<String, Vec<&Record>> = HashMap::new();
        for record in records {
            let key: String = group_by
                .iter()
                .filter_map(|col| {
                    record.data.get(col).map(|v| match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => format!("{:?}", v),
                    })
                })
                .collect::<Vec<_>>()
                .join("|");
            groups.entry(key).or_default().push(record);
        }

        let results: Vec<Record> = groups
            .into_iter()
            .map(|(group_key, group_records)| {
                let mut data = serde_json::Map::new();
                // Add group by columns
                for (_i, col) in group_by.iter().enumerate() {
                    if let Some(first) = group_records.first() {
                        if let Some(val) = first.data.get(col) {
                            data.insert(col.clone(), val.clone());
                        }
                    }
                }
                // Add aggregations
                for agg in aggregations {
                    let val = self.compute_aggregation(
                        &group_records
                            .iter()
                            .map(|r| (*r).clone())
                            .collect::<Vec<_>>(),
                        agg,
                    );
                    let key = agg
                        .alias
                        .clone()
                        .unwrap_or_else(|| format!("{:?}({})", agg.agg_type, agg.column));
                    data.insert(key, val);
                }
                Record {
                    id: format!("group_{}", group_key),
                    data: serde_json::Value::Object(data),
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(results)
    }

    fn compute_aggregation(
        &self,
        records: &[Record],
        agg: &AggregationClause,
    ) -> serde_json::Value {
        match agg.agg_type {
            AggregationType::Count => {
                serde_json::json!(records.len())
            }
            AggregationType::Sum => {
                let sum: f64 = records
                    .iter()
                    .filter_map(|r| r.data.get(&agg.column).and_then(|v| v.as_f64()))
                    .sum();
                serde_json::json!(sum)
            }
            AggregationType::Avg => {
                let values: Vec<f64> = records
                    .iter()
                    .filter_map(|r| r.data.get(&agg.column).and_then(|v| v.as_f64()))
                    .collect();
                if values.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(values.iter().sum::<f64>() / values.len() as f64)
                }
            }
            AggregationType::Min => records
                .iter()
                .filter_map(|r| r.data.get(&agg.column).and_then(|v| v.as_f64()))
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),
            AggregationType::Max => records
                .iter()
                .filter_map(|r| r.data.get(&agg.column).and_then(|v| v.as_f64()))
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|v| serde_json::json!(v))
                .unwrap_or(serde_json::Value::Null),
            AggregationType::GroupConcat => {
                let values: Vec<String> = records
                    .iter()
                    .filter_map(|r| {
                        r.data
                            .get(&agg.column)
                            .and_then(|v| v.as_str().map(String::from))
                    })
                    .collect();
                serde_json::json!(values.join(","))
            }
            AggregationType::ArrayAgg => {
                let values: Vec<serde_json::Value> = records
                    .iter()
                    .filter_map(|r| r.data.get(&agg.column).cloned())
                    .collect();
                serde_json::json!(values)
            }
        }
    }

    fn execute_sort(&self, records: &[Record], order_by: &[OrderByClause]) -> Result<Vec<Record>> {
        if order_by.is_empty() {
            return Ok(records.to_vec());
        }

        let mut sorted = records.to_vec();

        sorted.sort_by(|a, b| {
            for clause in order_by {
                let a_val = a.data.get(&clause.column);
                let b_val = b.data.get(&clause.column);
                let ascending = clause.direction.to_uppercase() == "ASC";

                let cmp = match (a_val, b_val) {
                    (Some(av), Some(bv)) => match (av, bv) {
                        (serde_json::Value::Number(an), serde_json::Value::Number(bn)) => an
                            .as_f64()
                            .unwrap_or(0.0)
                            .partial_cmp(&bn.as_f64().unwrap_or(0.0))
                            .unwrap_or(std::cmp::Ordering::Equal),
                        (serde_json::Value::String(as_), serde_json::Value::String(bs)) => {
                            as_.cmp(bs)
                        }
                        (serde_json::Value::Bool(ab), serde_json::Value::Bool(bb)) => ab.cmp(bb),
                        _ => std::cmp::Ordering::Equal,
                    },
                    _ => std::cmp::Ordering::Equal,
                };

                if cmp != std::cmp::Ordering::Equal {
                    return if ascending { cmp } else { cmp.reverse() };
                }
            }
            std::cmp::Ordering::Equal
        });

        Ok(sorted)
    }

    fn execute_insert_op(
        &self,
        table: &str,
        engine: &str,
        conditions: &Option<String>,
        columns: &[String],
    ) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            // Build the data payload from conditions (VALUES clause)
            let values_str = conditions.as_deref().unwrap_or("");
            let data = if !values_str.is_empty() {
                let parts: Vec<&str> = values_str.split(',').map(|s| s.trim()).collect();
                let mut map = serde_json::Map::new();
                for (i, val) in parts.iter().enumerate() {
                    let col = columns
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("col_{}", i));
                    let parsed_val: serde_json::Value = if let Ok(n) = val.parse::<i64>() {
                        serde_json::json!(n)
                    } else if let Ok(f) = val.parse::<f64>() {
                        serde_json::json!(f)
                    } else {
                        let s = val.trim_matches('\'');
                        serde_json::json!(s)
                    };
                    map.insert(col, parsed_val);
                }
                serde_json::Value::Object(map)
            } else {
                serde_json::json!({})
            };

            let transaction = crate::transaction::Transaction {
                id: format!("insert_{}", table),
                operations: vec![],
                status: crate::transaction::TransactionStatus::Prepared,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                timeout_ms: 0,
                ..Default::default()
            };

            let count =
                block_on_sync(async { storage_engine.insert(table, &data, &transaction).await })?;

            Ok(vec![Record {
                id: format!("insert_{}", table),
                data: serde_json::json!({
                    "operation": "insert",
                    "table": table,
                    "engine": engine,
                    "rows_inserted": count,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not registered for insert",
                engine
            )))
        }
    }

    fn execute_update_op(
        &self,
        table: &str,
        engine: &str,
        conditions: &Option<String>,
        projections: &[String],
    ) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            // Build conditions JSON from the WHERE clause string
            let conditions_json = conditions
                .as_ref()
                .and_then(|c| sql_str_to_json_condition(c));

            // Build SET data from projections (e.g. "name = 'Alice'")
            let mut data_map = serde_json::Map::new();
            for clause in projections {
                if let Some(eq_pos) = clause.find(" = ") {
                    let col = clause[..eq_pos].trim().to_string();
                    let val_str = clause[eq_pos + 3..].trim().trim_matches('\'');
                    let parsed_val: serde_json::Value = if let Ok(n) = val_str.parse::<i64>() {
                        serde_json::json!(n)
                    } else if let Ok(f) = val_str.parse::<f64>() {
                        serde_json::json!(f)
                    } else {
                        serde_json::json!(val_str)
                    };
                    data_map.insert(col, parsed_val);
                }
            }
            let data = serde_json::Value::Object(data_map);

            let transaction = crate::transaction::Transaction {
                id: format!("update_{}", table),
                operations: vec![],
                status: crate::transaction::TransactionStatus::Prepared,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                timeout_ms: 0,
                ..Default::default()
            };

            let count = block_on_sync(async {
                storage_engine
                    .update(table, conditions_json.as_ref(), &data, &transaction)
                    .await
            })?;

            Ok(vec![Record {
                id: format!("update_{}", table),
                data: serde_json::json!({
                    "operation": "update",
                    "table": table,
                    "engine": engine,
                    "rows_affected": count,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not registered for update",
                engine
            )))
        }
    }

    fn execute_delete_op(
        &self,
        table: &str,
        engine: &str,
        conditions: &Option<String>,
    ) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            let conditions_json = conditions
                .as_ref()
                .and_then(|c| sql_str_to_json_condition(c));

            let transaction = crate::transaction::Transaction {
                id: format!("delete_{}", table),
                operations: vec![],
                status: crate::transaction::TransactionStatus::Prepared,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                timeout_ms: 0,
                ..Default::default()
            };

            let count = block_on_sync(async {
                storage_engine
                    .delete(table, conditions_json.as_ref(), &transaction)
                    .await
            })?;

            Ok(vec![Record {
                id: format!("delete_{}", table),
                data: serde_json::json!({
                    "operation": "delete",
                    "table": table,
                    "engine": engine,
                    "rows_deleted": count,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not registered for delete",
                engine
            )))
        }
    }

    fn execute_create_table(&self, table: &str, engine: &str) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            let schema = crate::storage::Schema {
                fields: vec![],
                indexes: vec![],
                constraints: vec![],
            };

            block_on_sync(async { storage_engine.create_table(table, &schema).await })?;

            Ok(vec![Record {
                id: format!("create_{}", table),
                data: serde_json::json!({
                    "operation": "create_table",
                    "table": table,
                    "engine": engine,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not found for create table",
                engine
            )))
        }
    }

    fn execute_drop_table(&self, table: &str, engine: &str) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            block_on_sync(async { storage_engine.drop_table(table).await })?;

            Ok(vec![Record {
                id: format!("drop_{}", table),
                data: serde_json::json!({
                    "operation": "drop_table",
                    "table": table,
                    "engine": engine,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not found for drop table",
                engine
            )))
        }
    }

    fn execute_alter_table(
        &self,
        table: &str,
        engine: &str,
        conditions: &Option<String>,
    ) -> Result<Vec<Record>> {
        let storage_type = Self::engine_to_storage_type(engine)?;
        let engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = engines.get(&storage_type) {
            // Try downcast to RelationalEngine for alter operations
            if let Some(rel) =
                (*storage_engine).as_any().downcast_ref::<crate::storage::relational::RelationalEngine>()
            {
                if let Some(def) = conditions {
                    let def_upper = def.to_uppercase();
                    if def_upper.contains("RENAME TO") {
                        if let Some(new_name) = def.split_whitespace().last() {
                            rel.rename_table(table, new_name)?;
                        }
                    } else if def_upper.contains("DROP COLUMN") {
                        let parts: Vec<&str> = def.split_whitespace().collect();
                        if parts.len() >= 3 {
                            rel.alter_table_drop_column(table, parts[2])?;
                        }
                    } else if def_upper.contains("DROP CONSTRAINT") {
                        let parts: Vec<&str> = def.split_whitespace().collect();
                        if parts.len() >= 3 {
                            rel.alter_table_drop_constraint(table, parts[2])?;
                        }
                    } else if def_upper.contains("ADD CONSTRAINT") {
                        let parts: Vec<&str> = def.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let constraint = crate::storage::Constraint {
                                constraint_type: crate::storage::ConstraintType::Unique,
                                fields: vec![parts[2].to_string()],
                                name: format!("{}_{}", table, parts[2]),
                                definition: None,
                            };
                            rel.alter_table_add_constraint(table, constraint)?;
                        }
                    }
                }
            }

            Ok(vec![Record {
                id: format!("alter_{}", table),
                data: serde_json::json!({
                    "operation": "alter_table",
                    "table": table,
                    "engine": engine,
                    "definition": conditions,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not found for alter table",
                engine
            )))
        }
    }

    fn execute_truncate(&self, table: &str, engine: &str) -> Result<Vec<Record>> {
        let storage_type = match engine {
            "columnar" => StorageType::Columnar,
            "vector" => StorageType::Vector,
            "document" => StorageType::Document,
            "relational" => StorageType::Relational,
            "keyvalue" => StorageType::KeyValue,
            _ => return Err(crate::Error::DatabaseError(format!("Unknown storage engine: {}", engine))),
        };
        let storage_engines = self.storage_engines.read().map_err(|_| {
            crate::Error::DatabaseError("Storage engines lock poisoned".to_string())
        })?;

        if let Some(storage_engine) = storage_engines.get(&storage_type) {
            block_on_sync(async { storage_engine.truncate_table(table, false).await })?;

            Ok(vec![Record {
                id: format!("truncate_{}", table),
                data: serde_json::json!({
                    "operation": "truncate",
                    "table": table,
                    "engine": engine,
                    "success": true
                }),
                metadata: HashMap::new(),
            }])
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Storage engine '{}' not found for truncate",
                engine
            )))
        }
    }

    fn execute_insert(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        for stage in &plan.stages {
            if let StageOperation::Insert { table, engine } = &stage.operation {
                return self.execute_insert_op(
                    table,
                    engine,
                    &stage.conditions,
                    &stage.projections,
                );
            }
        }
        Ok(vec![])
    }

    fn execute_update(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        self.execute_select(plan)
    }

    fn execute_delete(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        self.execute_select(plan)
    }

    fn execute_ddl(&self, plan: &QueryPlan) -> Result<Vec<Record>> {
        for stage in &plan.stages {
            return match &stage.operation {
                StageOperation::Create { table, engine } => {
                    self.execute_create_table(table, engine)
                }
                StageOperation::Drop { table, engine } => self.execute_drop_table(table, engine),
                StageOperation::Alter { table, engine } => {
                    self.execute_alter_table(table, engine, &stage.conditions)
                }
                StageOperation::Truncate { table, engine } => self.execute_truncate(table, engine),
                _ => Ok(vec![]),
            };
        }
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_str_to_json_condition_eq() {
        let result = sql_str_to_json_condition("col_0 = 2");
        assert!(result.is_some(), "Expected Some for 'col_0 = 2'");
        let v = result.unwrap();
        assert_eq!(v.get("op").and_then(|o| o.as_str()), Some("eq"));
        assert_eq!(v.get("field").and_then(|f| f.as_str()), Some("col_0"));
        assert_eq!(v.get("value").and_then(|v| v.as_i64()), Some(2));
    }

    #[test]
    fn test_sql_str_to_json_condition_gt() {
        let result = sql_str_to_json_condition("age > 18");
        assert!(result.is_some());
        let v = result.unwrap();
        assert_eq!(v.get("op").and_then(|o| o.as_str()), Some("gt"));
        assert_eq!(v.get("field").and_then(|f| f.as_str()), Some("age"));
        assert_eq!(v.get("value").and_then(|f| f.as_i64()), Some(18));
    }

    #[test]
    fn test_sql_str_to_json_condition_and() {
        let result = sql_str_to_json_condition("col_0 = 1 AND col_1 = 'Alice'");
        assert!(result.is_some(), "Expected Some for AND condition");
        let v = result.unwrap();
        assert_eq!(v.get("op").and_then(|o| o.as_str()), Some("and"));
    }
}
