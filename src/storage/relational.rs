/*
 * PrimusDB Relational Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: as_any() method for engine-specific features
 */

#[allow(unused_imports)]
use crate::{
    storage::{
        Constraint, ConstraintType, Field, FieldType, ReferentialAction, Schema,
        Sequence as SchemaSequence, StorageEngine, TableInfo, Trigger as SchemaTrigger,
        TriggerEvent as SchemaTriggerEvent, TriggerOperation as SchemaTriggerOperation,
        TriggerTiming as SchemaTriggerTiming, View as SchemaView,
    },
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

#[allow(dead_code)]
pub enum RelationalQuery<'a> {
    Select {
        table: &'a str,
        fields: Option<Vec<String>>,
        conditions: Option<&'a serde_json::Value>,
        order_by: Option<&'a str>,
        limit: u64,
        offset: u64,
        distinct: bool,
    },
    Insert {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
    },
    InsertReturning {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
        returning: Vec<String>,
    },
    Update {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
        conditions: Option<&'a serde_json::Value>,
    },
    UpdateReturning {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
        conditions: Option<&'a serde_json::Value>,
        returning: Vec<String>,
    },
    Delete {
        table: &'a str,
        conditions: Option<&'a serde_json::Value>,
    },
    DeleteReturning {
        table: &'a str,
        conditions: Option<&'a serde_json::Value>,
        returning: Vec<String>,
    },
    Join {
        join_type: JoinType,
        left_table: &'a str,
        right_table: &'a str,
        condition: &'a JoinCondition,
        fields: Option<Vec<String>>,
    },
    SelectGrouped {
        table: &'a str,
        fields: Option<Vec<String>>,
        conditions: Option<&'a serde_json::Value>,
        group_by: Vec<String>,
        having: Option<serde_json::Value>,
        order_by: Option<&'a str>,
        limit: u64,
        offset: u64,
        distinct: bool,
    },
    Truncate {
        table: &'a str,
        cascade: bool,
    },
}

pub enum QueryResult {
    Records(Vec<Record>),
    AffectedRows(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableAnalysis {
    table_name: String,
    row_count: u64,
    index_count: u64,
    average_row_size: f64,
    total_size_bytes: u64,
}

pub struct RelationalEngine {
    #[allow(dead_code)]
    config: PrimusDBConfig,
    db: sled::Db,
    tables: Arc<RwLock<HashMap<String, RelationalTable>>>,
    foreign_keys: Arc<RwLock<HashMap<String, Vec<ForeignKey>>>>,
    sequences: Arc<RwLock<HashMap<String, RelationalSequence>>>,
    views: Arc<RwLock<HashMap<String, RelationalView>>>,
    triggers: Arc<RwLock<HashMap<String, Vec<Trigger>>>>,
}

#[derive(Debug)]
struct RelationalTable {
    name: String,
    schema: Schema,
    rows: HashMap<u64, Row>,
    next_id: u64,
    indexes: HashMap<String, Index>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Row {
    id: u64,
    data: serde_json::Map<String, serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    version: u64,
    #[serde(default)]
    vector_clock: HashMap<String, u64>,
    #[serde(default)]
    checksum: String,
}

impl Row {
    fn compute_checksum(data: &serde_json::Map<String, serde_json::Value>) -> String {
        use sha2::Digest;
        let serialized = serde_json::to_string(data).unwrap_or_default();
        format!("{:x}", sha2::Sha256::digest(serialized.as_bytes()))
    }

    fn new_row(
        id: u64,
        data: serde_json::Map<String, serde_json::Value>,
        node_id: &str,
    ) -> Self {
        let mut vc = HashMap::new();
        vc.insert(node_id.to_string(), 1);
        Row {
            id,
            checksum: Self::compute_checksum(&data),
            data,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
            vector_clock: vc,
        }
    }

    fn increment_version(&mut self, node_id: &str) {
        self.version += 1;
        self.updated_at = chrono::Utc::now();
        let counter = self.vector_clock.entry(node_id.to_string()).or_insert(0);
        *counter += 1;
        self.checksum = Self::compute_checksum(&self.data);
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Index {
    name: String,
    columns: Vec<String>,
    data: HashMap<String, Vec<u64>>,
    unique: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForeignKey {
    pub name: String,
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub on_delete: CascadeAction,
    pub on_update: CascadeAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalSequence {
    pub name: String,
    pub current_value: i64,
    pub increment: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub cycle: bool,
    pub cache_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalView {
    pub name: String,
    pub query_definition: serde_json::Value,
    pub columns: Vec<String>,
    pub referenced_tables: Vec<String>,
    pub cached_data: Vec<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub name: String,
    pub table_name: String,
    pub timing: TriggerTiming,
    pub event: TriggerEvent,
    pub operation: TriggerOperation,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerTiming {
    Before,
    After,
    InsteadOf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerEvent {
    Insert,
    Update,
    Delete,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerOperation {
    Function(String),
    Execute(String),
    Raise(String),
}

impl RelationalEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let db_path = format!("{}/relational", config.storage.data_dir);
        std::fs::create_dir_all(&db_path)?;
        let db: sled::Db = sled::open(&db_path)?;

        let tables = Arc::new(RwLock::new(HashMap::new()));
        let foreign_keys = Arc::new(RwLock::new(HashMap::new()));
        let sequences = Arc::new(RwLock::new(HashMap::new()));
        let views = Arc::new(RwLock::new(HashMap::new()));
        let triggers = Arc::new(RwLock::new(HashMap::new()));

        let schemas_tree = db.open_tree("_schemas")?;

        for result in schemas_tree.iter() {
            let (key, value) = result?;
            let table_name = String::from_utf8(key.to_vec())
                .map_err(|e| crate::Error::DataCorruption(e.to_string()))?;
            let schema: Schema = serde_json::from_slice(&value)?;

            let next_ids_tree = db.open_tree("_next_ids")?;
            let next_id: u64 = next_ids_tree
                .get(&key)?
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or(1);

            let created_at_tree = db.open_tree("_created_at")?;
            let created_at: chrono::DateTime<chrono::Utc> = created_at_tree
                .get(&key)?
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or_else(chrono::Utc::now);

            let updated_at_tree = db.open_tree("_updated_at")?;
            let updated_at: chrono::DateTime<chrono::Utc> = updated_at_tree
                .get(&key)?
                .and_then(|v| serde_json::from_slice(&v).ok())
                .unwrap_or_else(chrono::Utc::now);

            let mut table = RelationalTable {
                name: table_name.clone(),
                schema,
                rows: HashMap::new(),
                next_id,
                indexes: HashMap::new(),
                created_at,
                updated_at,
            };

            if let Ok(table_tree) = db.open_tree(format!("table:{}", table_name)) {
                for row_result in table_tree.iter() {
                    let (row_key_bytes, row_value) = row_result?;
                    let mut row: Row = serde_json::from_slice(&row_value)?;
                    let arr: [u8; 8] = row_key_bytes
                        .as_ref()
                        .try_into()
                        .map_err(|_| crate::Error::DataCorruption("Invalid row key".to_string()))?;
                    row.id = u64::from_be_bytes(arr);
                    table.rows.insert(row.id, row);
                }
            }

            tables.write().unwrap().insert(table_name, table);
        }

        if let Ok(seq_tree) = db.open_tree("_sequences") {
            for result in seq_tree.iter() {
                let (key, value) = result?;
                let name = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::Error::DataCorruption(e.to_string()))?;
                if let Ok(seq) = serde_json::from_slice::<RelationalSequence>(&value) {
                    sequences.write().unwrap().insert(name, seq);
                }
            }
        }

        if let Ok(view_tree) = db.open_tree("_views") {
            for result in view_tree.iter() {
                let (key, value) = result?;
                let name = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::Error::DataCorruption(e.to_string()))?;
                if let Ok(view) = serde_json::from_slice::<RelationalView>(&value) {
                    views.write().unwrap().insert(name, view);
                }
            }
        }

        if let Ok(trig_tree) = db.open_tree("_triggers") {
            for result in trig_tree.iter() {
                let (key, value) = result?;
                let table_name = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::Error::DataCorruption(e.to_string()))?;
                if let Ok(t) = serde_json::from_slice::<Vec<Trigger>>(&value) {
                    triggers.write().unwrap().insert(table_name, t);
                }
            }
        }

        info!("Relational storage engine initialized at {}", db_path);

        Ok(RelationalEngine {
            config: config.clone(),
            db,
            tables,
            foreign_keys,
            sequences,
            views,
            triggers,
        })
    }

    fn persist_row(&self, table_name: &str, row: &Row) -> Result<()> {
        let table_tree = self.db.open_tree(format!("table:{}", table_name))?;
        let key = row.id.to_be_bytes();
        let value = serde_json::to_vec(row)?;
        table_tree.insert(key.as_ref(), value.as_slice())?;
        Ok(())
    }

    fn remove_row(&self, table_name: &str, row_id: u64) -> Result<()> {
        let table_tree = self.db.open_tree(format!("table:{}", table_name))?;
        table_tree.remove(row_id.to_be_bytes())?;
        Ok(())
    }

    fn persist_next_id(&self, table_name: &str, next_id: u64) -> Result<()> {
        let next_ids_tree = self.db.open_tree("_next_ids")?;
        next_ids_tree.insert(table_name, serde_json::to_vec(&next_id)?)?;
        Ok(())
    }

    fn persist_updated_at(
        &self,
        table_name: &str,
        ts: &chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let tree = self.db.open_tree("_updated_at")?;
        tree.insert(table_name, serde_json::to_vec(ts)?)?;
        Ok(())
    }

    #[allow(dead_code)]
    fn validate_constraints(&self, table_name: &str, row: &Row) -> Result<()> {
        let tables = self.tables.read().unwrap();
        if let Some(table) = tables.get(table_name) {
            for constraint in &table.schema.constraints {
                match &constraint.constraint_type {
                    ConstraintType::NotNull => {
                        for field_name in &constraint.fields {
                            if row.data.get(field_name).is_none_or(|v| v.is_null()) {
                                return Err(crate::Error::ValidationError(format!(
                                    "Field {} cannot be null",
                                    field_name
                                )));
                            }
                        }
                    }
                    ConstraintType::Unique => {
                        for field_name in &constraint.fields {
                            if let Some(value) = row.data.get(field_name) {
                                for other_row in table.rows.values() {
                                    if other_row.id != row.id {
                                        if let Some(other_value) = other_row.data.get(field_name) {
                                            if value == other_value {
                                                return Err(crate::Error::ValidationError(
                                                    format!(
                                                        "Unique constraint violated for field {}",
                                                        field_name
                                                    ),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    ConstraintType::Check { expression } => {
                        info!("Evaluating check constraint: {}", expression);
                    }
                    ConstraintType::ForeignKey {
                        references_table,
                        references_field,
                        ..
                    } => {
                        for field_name in &constraint.fields {
                            if let Some(value) = row.data.get(field_name) {
                                if let Some(ref_table) = tables.get(references_table) {
                                    let mut found = false;
                                    for ref_row in ref_table.rows.values() {
                                        if let Some(ref_val) = ref_row.data.get(references_field) {
                                            if value == ref_val {
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found {
                                        return Err(crate::Error::ValidationError(format!(
                                            "Foreign key constraint violated: {} references non-existent {} in table {}",
                                            field_name, value, references_table
                                        )));
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn validate_foreign_key_on_insert(&self, table_name: &str, row: &Row) -> Result<()> {
        let foreign_keys = self.foreign_keys.read().unwrap();
        if let Some(fks) = foreign_keys.get(table_name) {
            for fk in fks {
                if let Some(value) = row.data.get(&fk.from_column) {
                    let tables = self.tables.read().unwrap();
                    if let Some(ref_table) = tables.get(&fk.to_table) {
                        let mut found = false;
                        for ref_row in ref_table.rows.values() {
                            if let Some(ref_val) = ref_row.data.get(&fk.to_column) {
                                if value == ref_val {
                                    found = true;
                                    break;
                                }
                            }
                        }
                        if !found {
                            return Err(crate::Error::ValidationError(format!(
                                "Foreign key constraint violated: {}={} does not exist in {}.{}",
                                fk.from_column, value, fk.to_table, fk.to_column
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_foreign_key_on_delete(
        &self,
        table_name: &str,
        row_id: u64,
    ) -> Result<CascadeAction> {
        let tables = self.tables.read().unwrap();
        let foreign_keys = self.foreign_keys.read().unwrap();

        for (fk_table, fks) in foreign_keys.iter() {
            for fk in fks {
                if fk.to_table == table_name {
                    if let Some(child_table) = tables.get(fk_table) {
                        for child_row in child_table.rows.values() {
                            if let Some(fk_value) = child_row.data.get(&fk.to_column) {
                                if let Some(parent_row) = tables.get(table_name) {
                                    if let Some(parent_row) = parent_row.rows.get(&row_id) {
                                        if let Some(parent_id_val) =
                                            parent_row.data.get(&fk.to_column)
                                        {
                                            if fk_value == parent_id_val {
                                                return Ok(fk.on_delete);
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
        Ok(CascadeAction::Allow)
    }

    fn check_referential_integrity(
        &self,
        table_name: &str,
        old_data: &serde_json::Map<String, serde_json::Value>,
        new_data: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        let foreign_keys = self.foreign_keys.read().unwrap();

        for (fk_table, fks) in foreign_keys.iter() {
            if fk_table == table_name {
                for fk in fks {
                    let old_val = old_data.get(&fk.from_column);
                    let new_val = new_data.get(&fk.from_column);

                    if old_val != new_val {
                        if let Some(value) = new_val {
                            let tables = self.tables.read().unwrap();
                            if let Some(ref_table) = tables.get(&fk.to_table) {
                                let mut found = false;
                                for ref_row in ref_table.rows.values() {
                                    if let Some(ref_val) = ref_row.data.get(&fk.to_column) {
                                        if value == ref_val {
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                                if !found {
                                    return Err(crate::Error::ValidationError(format!(
                                        "Referential integrity violated: {}={} does not exist in {}.{}",
                                        fk.from_column, value, fk.to_table, fk.to_column
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn add_foreign_key(&self, fk: ForeignKey) -> Result<()> {
        let mut foreign_keys = self.foreign_keys.write().unwrap();
        foreign_keys
            .entry(fk.from_table.clone())
            .or_insert_with(Vec::new)
            .push(fk);
        Ok(())
    }

    pub fn get_foreign_keys(&self, table_name: &str) -> Result<Vec<ForeignKey>> {
        let foreign_keys = self.foreign_keys.read().unwrap();
        Ok(foreign_keys.get(table_name).cloned().unwrap_or_default())
    }

    fn join_tables(
        &self,
        left_table: &str,
        right_table: &str,
        join_condition: &JoinCondition,
    ) -> Result<Vec<JoinedRow>> {
        info!("Performing join between {} and {}", left_table, right_table);

        let mut joined_rows = Vec::new();
        let mut left_unmatched: Vec<&Row> = Vec::new();
        let mut right_unmatched: Vec<&Row> = Vec::new();

        let tables = self.tables.read().unwrap();
        if let Some(left_rel_table) = tables.get(left_table) {
            if let Some(right_rel_table) = tables.get(right_table) {
                let left_rows: Vec<&Row> = left_rel_table.rows.values().collect();
                let right_rows: Vec<&Row> = right_rel_table.rows.values().collect();

                let mut matched_left: Vec<bool> = vec![false; left_rows.len()];
                let mut matched_right: Vec<bool> = vec![false; right_rows.len()];

                for (left_idx, left_row) in left_rows.iter().enumerate() {
                    let mut found_match = false;

                    for (right_idx, right_row) in right_rows.iter().enumerate() {
                        let should_join = match join_condition.join_type {
                            JoinType::Inner | JoinType::Left | JoinType::Right | JoinType::Full => {
                                if let Some(left_val) =
                                    left_row.data.get(&join_condition.left_field)
                                {
                                    if let Some(right_val) =
                                        right_row.data.get(&join_condition.right_field)
                                    {
                                        left_val == right_val
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            }
                            JoinType::Cross => true,
                        };

                        if should_join {
                            found_match = true;
                            matched_right[right_idx] = true;
                            joined_rows.push(JoinedRow {
                                left_row: (*left_row).clone(),
                                right_row: Some((*right_row).clone()),
                            });
                        }
                    }

                    if found_match {
                        matched_left[left_idx] = true;
                    }
                }

                match join_condition.join_type {
                    JoinType::Left => {
                        for (idx, matched) in matched_left.iter().enumerate() {
                            if !*matched {
                                left_unmatched.push(left_rows[idx]);
                            }
                        }
                        for row in left_unmatched {
                            joined_rows.push(JoinedRow {
                                left_row: row.clone(),
                                right_row: None,
                            });
                        }
                    }
                    JoinType::Right => {
                        for (idx, matched) in matched_right.iter().enumerate() {
                            if !*matched {
                                right_unmatched.push(right_rows[idx]);
                            }
                        }
                        for row in right_unmatched {
                            joined_rows.push(JoinedRow {
                                left_row: Row {
                                    id: 0,
                                    data: serde_json::Map::new(),
                                    created_at: chrono::Utc::now(),
                                    updated_at: chrono::Utc::now(),
                                    version: 0,
                                    vector_clock: HashMap::new(),
                                    checksum: String::new(),
                                },
                                right_row: Some(row.clone()),
                            });
                        }
                    }
                    JoinType::Full => {
                        for (idx, matched) in matched_left.iter().enumerate() {
                            if !*matched {
                                left_unmatched.push(left_rows[idx]);
                            }
                        }
                        for (idx, matched) in matched_right.iter().enumerate() {
                            if !*matched {
                                right_unmatched.push(right_rows[idx]);
                            }
                        }
                        for row in left_unmatched {
                            joined_rows.push(JoinedRow {
                                left_row: row.clone(),
                                right_row: None,
                            });
                        }
                        for row in right_unmatched {
                            joined_rows.push(JoinedRow {
                                left_row: Row {
                                    id: 0,
                                    data: serde_json::Map::new(),
                                    created_at: chrono::Utc::now(),
                                    updated_at: chrono::Utc::now(),
                                    version: 0,
                                    vector_clock: HashMap::new(),
                                    checksum: String::new(),
                                },
                                right_row: Some(row.clone()),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(joined_rows)
    }

    pub fn execute_query(&self, query: &RelationalQuery) -> Result<QueryResult> {
        match query {
            RelationalQuery::Select {
                table,
                fields,
                conditions,
                order_by,
                limit,
                offset,
                distinct,
            } => self.execute_select(
                table,
                fields.as_deref(),
                conditions.as_deref(),
                *order_by,
                *distinct,
                *limit,
                *offset,
            ),
            RelationalQuery::Insert { table, data } => self.execute_insert(table, data),
            RelationalQuery::InsertReturning {
                table,
                data,
                returning,
            } => self.execute_insert_returning(table, data, returning),
            RelationalQuery::Update {
                table,
                data,
                conditions,
            } => self.execute_update(table, data, conditions.as_deref()),
            RelationalQuery::UpdateReturning {
                table,
                data,
                conditions,
                returning,
            } => self.execute_update_returning(table, data, conditions.as_deref(), returning),
            RelationalQuery::Delete { table, conditions } => {
                self.execute_delete(table, conditions.as_deref())
            }
            RelationalQuery::DeleteReturning {
                table,
                conditions,
                returning,
            } => self.execute_delete_returning(table, conditions.as_deref(), returning),
            RelationalQuery::Join {
                join_type,
                left_table,
                right_table,
                condition,
                fields,
            } => self.execute_join(
                join_type,
                left_table,
                right_table,
                condition,
                fields.as_deref(),
            ),
            RelationalQuery::SelectGrouped {
                table,
                fields,
                conditions,
                group_by,
                having,
                order_by,
                limit,
                offset,
                distinct,
            } => self.execute_select_grouped(
                table,
                fields.as_deref(),
                conditions.as_deref(),
                group_by,
                having,
                *order_by,
                *limit,
                *offset,
                *distinct,
            ),
            RelationalQuery::Truncate { table, cascade } => self.execute_truncate(table, *cascade),
        }
    }

    fn execute_select(
        &self,
        table: &str,
        fields: Option<&[String]>,
        conditions: Option<&serde_json::Value>,
        _order_by: Option<&str>,
        _distinct: bool,
        limit: u64,
        offset: u64,
    ) -> Result<QueryResult> {
        let tables = self.tables.read().unwrap();

        if let Some(table_data) = tables.get(table) {
            let mut records = Vec::new();

            for row in table_data.rows.values() {
                let should_include = if let Some(cond) = conditions {
                    self.evaluate_condition(cond, &row.data)?
                } else {
                    true
                };

                if should_include {
                    let selected_data = if let Some(fields) = fields {
                        let mut data = serde_json::Map::new();
                        for field in fields {
                            if let Some(val) = row.data.get(field) {
                                data.insert(field.clone(), val.clone());
                            }
                        }
                        data
                    } else {
                        row.data.clone()
                    };

                    records.push(Record {
                        id: row.id.to_string(),
                        data: serde_json::Value::Object(selected_data),
                        metadata: HashMap::new(),
                    });
                }
            }

            let records: Vec<Record> = records
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();
            Ok(QueryResult::Records(records))
        } else {
            Ok(QueryResult::Records(vec![]))
        }
    }

    fn execute_insert(
        &self,
        table: &str,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<QueryResult> {
        let node_id = &self.config.cluster.node_id;
        let row = Row::new_row(0, data.clone(), node_id);

        self.validate_foreign_key_on_insert(table, &row)?;

        let mut tables = self.tables.write().unwrap();
        let table_entry = tables
            .entry(table.to_string())
            .or_insert_with(|| RelationalTable {
                name: table.to_string(),
                schema: Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                },
                rows: HashMap::new(),
                next_id: 1,
                indexes: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });

        let id = table_entry.next_id;
        table_entry.next_id += 1;

        let row = Row::new_row(id, data.clone(), node_id);

        self.persist_row(table, &row)?;
        self.persist_next_id(table, table_entry.next_id)?;

        table_entry.rows.insert(id, row);

        Ok(QueryResult::AffectedRows(1))
    }

    fn execute_update(
        &self,
        table: &str,
        data: &serde_json::Map<String, serde_json::Value>,
        conditions: Option<&serde_json::Value>,
    ) -> Result<QueryResult> {
        let mut tables = self.tables.write().unwrap();
        let mut affected = 0u64;

        if let Some(table_data) = tables.get_mut(table) {
            let ids_to_update: Vec<u64> = table_data
                .rows
                .iter()
                .filter(|(_, row)| {
                    conditions.map_or(true, |cond| {
                        self.evaluate_condition(cond, &row.data).unwrap_or(false)
                    })
                })
                .map(|(id, _)| *id)
                .collect();

            for id in ids_to_update {
                if let Some(row) = table_data.rows.get_mut(&id) {
                    self.check_referential_integrity(table, &row.data, data)?;
                    row.data = data.clone();
                    row.increment_version(&self.config.cluster.node_id);
                    self.persist_row(table, row)?;
                    affected += 1;
                }
            }

            if affected > 0 {
                table_data.updated_at = chrono::Utc::now();
                self.persist_updated_at(table, &table_data.updated_at)?;
            }
        }

        Ok(QueryResult::AffectedRows(affected))
    }

    fn execute_delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
    ) -> Result<QueryResult> {
        let mut tables = self.tables.write().unwrap();
        let mut affected = 0u64;
        let mut to_delete: Vec<u64> = Vec::new();

        if let Some(table_data) = tables.get_mut(table) {
            for (id, row) in table_data.rows.iter() {
                let should_delete = if let Some(cond) = conditions {
                    self.evaluate_condition(cond, &row.data)?
                } else {
                    true
                };

                if should_delete {
                    if let Ok(action) = self.validate_foreign_key_on_delete(table, *id) {
                        if matches!(action, CascadeAction::Allow) {
                            to_delete.push(*id);
                        }
                    }
                }
            }

            for id in &to_delete {
                table_data.rows.remove(id);
                self.remove_row(table, *id)?;
                affected += 1;
            }

            if affected > 0 {
                table_data.updated_at = chrono::Utc::now();
                self.persist_updated_at(table, &table_data.updated_at)?;
            }
        }

        Ok(QueryResult::AffectedRows(affected))
    }

    fn execute_join(
        &self,
        join_type: &JoinType,
        left_table: &str,
        right_table: &str,
        condition: &JoinCondition,
        _fields: Option<&[String]>,
    ) -> Result<QueryResult> {
        let join_condition = JoinCondition {
            left_field: condition.left_field.clone(),
            right_field: condition.right_field.clone(),
            join_type: *join_type,
        };

        let joined = self.join_tables(left_table, right_table, &join_condition)?;

        let mut records = Vec::new();
        for jr in joined {
            let mut combined_data = jr.left_row.data.clone();
            if let Some(right) = jr.right_row {
                for (k, v) in right.data {
                    combined_data.insert(format!("{}.{}", right_table, k), v);
                }
            }

            records.push(Record {
                id: jr.left_row.id.to_string(),
                data: serde_json::Value::Object(combined_data),
                metadata: HashMap::new(),
            });
        }

        Ok(QueryResult::Records(records))
    }

    fn evaluate_condition(
        &self,
        condition: &serde_json::Value,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<bool> {
        if let Some(obj) = condition.as_object() {
            if let Some(op) = obj.get("op").and_then(|v| v.as_str()) {
                match op {
                    "eq" => {
                        if let (Some(field), Some(value)) =
                            (obj.get("field").and_then(|v| v.as_str()), obj.get("value"))
                        {
                            if let Some(data_val) = data.get(field) {
                                return Ok(data_val == value);
                            }
                            return Ok(false);
                        }
                    }
                    "ne" => {
                        if let (Some(field), Some(value)) =
                            (obj.get("field").and_then(|v| v.as_str()), obj.get("value"))
                        {
                            if let Some(data_val) = data.get(field) {
                                return Ok(data_val != value);
                            }
                            return Ok(false);
                        }
                    }
                    "gt" => {
                        if let (Some(field), Some(value)) =
                            (obj.get("field").and_then(|v| v.as_str()), obj.get("value"))
                        {
                            if let (Some(data_val), Some(cond_val)) =
                                (data.get(field), value.as_f64())
                            {
                                if let Some(data_f64) = data_val.as_f64() {
                                    return Ok(data_f64 > cond_val);
                                }
                            }
                        }
                    }
                    "lt" => {
                        if let (Some(field), Some(value)) =
                            (obj.get("field").and_then(|v| v.as_str()), obj.get("value"))
                        {
                            if let (Some(data_val), Some(cond_val)) =
                                (data.get(field), value.as_f64())
                            {
                                if let Some(data_f64) = data_val.as_f64() {
                                    return Ok(data_f64 < cond_val);
                                }
                            }
                        }
                    }
                    "and" => {
                        if let (Some(left), Some(right)) = (obj.get("left"), obj.get("right")) {
                            return Ok(self.evaluate_condition(left, data)?
                                && self.evaluate_condition(right, data)?);
                        }
                    }
                    "or" => {
                        if let (Some(left), Some(right)) = (obj.get("left"), obj.get("right")) {
                            return Ok(self.evaluate_condition(left, data)?
                                || self.evaluate_condition(right, data)?);
                        }
                    }
                    "in" => {
                        if let (Some(field), Some(values)) = (
                            obj.get("field").and_then(|v| v.as_str()),
                            obj.get("values").and_then(|v| v.as_array()),
                        ) {
                            if let Some(data_val) = data.get(field) {
                                for v in values {
                                    if data_val == v {
                                        return Ok(true);
                                    }
                                }
                                return Ok(false);
                            }
                        }
                    }
                    "like" => {
                        if let (Some(field), Some(pattern)) = (
                            obj.get("field").and_then(|v| v.as_str()),
                            obj.get("pattern").and_then(|v| v.as_str()),
                        ) {
                            if let Some(data_val) = data.get(field).and_then(|v| v.as_str()) {
                                let pattern_regex = pattern.replace("%", ".*").replace("_", ".");
                                if let Ok(re) = regex::Regex::new(&pattern_regex) {
                                    return Ok(re.is_match(data_val));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(false)
    }

    pub fn create_index(&self, table_name: &str, index: Index) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table) = tables.get_mut(table_name) {
            table.indexes.insert(index.name.clone(), index);
        }
        Ok(())
    }

    pub fn drop_index(&self, table_name: &str, index_name: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table) = tables.get_mut(table_name) {
            table.indexes.remove(index_name);
        }
        Ok(())
    }

    pub fn analyze_table(&self, table_name: &str) -> Result<TableAnalysis> {
        let tables = self.tables.read().unwrap();

        if let Some(table) = tables.get(table_name) {
            let row_count = table.rows.len() as u64;
            let index_count = table.indexes.len() as u64;

            Ok(TableAnalysis {
                table_name: table_name.to_string(),
                row_count,
                index_count,
                average_row_size: 0.0,
                total_size_bytes: 0,
            })
        } else {
            Err(crate::Error::DatabaseError("Table not found".to_string()))
        }
    }

    // ─────────────────────────────────────────────
    // Sequence Methods
    // ─────────────────────────────────────────────

    pub fn create_sequence(
        &self,
        name: &str,
        increment: i64,
        min_value: i64,
        max_value: i64,
        cycle: bool,
        cache_size: u64,
    ) -> Result<()> {
        let mut sequences = self.sequences.write().unwrap();
        if sequences.contains_key(name) {
            return Err(crate::Error::DatabaseError(format!(
                "Sequence '{}' already exists",
                name
            )));
        }
        let seq = RelationalSequence {
            name: name.to_string(),
            current_value: min_value,
            increment,
            min_value,
            max_value,
            cycle,
            cache_size,
        };
        sequences.insert(name.to_string(), seq.clone());
        self.persist_sequence(&seq)?;
        info!("Created sequence: {}", name);
        Ok(())
    }

    pub fn drop_sequence(&self, name: &str) -> Result<()> {
        let mut sequences = self.sequences.write().unwrap();
        if sequences.remove(name).is_some() {
            let seq_tree = self.db.open_tree("_sequences")?;
            seq_tree.remove(name)?;
            info!("Dropped sequence: {}", name);
        }
        Ok(())
    }

    pub fn nextval(&self, name: &str) -> Result<i64> {
        let mut sequences = self.sequences.write().unwrap();
        if let Some(seq) = sequences.get_mut(name) {
            let next = seq
                .current_value
                .checked_add(seq.increment)
                .ok_or_else(|| crate::Error::DatabaseError("Sequence overflow".to_string()))?;
            if next > seq.max_value {
                if seq.cycle {
                    seq.current_value = seq.min_value;
                } else {
                    return Err(crate::Error::DatabaseError(format!(
                        "Sequence '{}' exceeded max_value {}",
                        name, seq.max_value
                    )));
                }
            } else {
                seq.current_value = next;
            }
            let val = seq.current_value;
            self.persist_sequence(&*seq)?;
            Ok(val)
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Sequence '{}' not found",
                name
            )))
        }
    }

    pub fn currval(&self, name: &str) -> Result<i64> {
        let sequences = self.sequences.read().unwrap();
        if let Some(seq) = sequences.get(name) {
            Ok(seq.current_value)
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Sequence '{}' not found",
                name
            )))
        }
    }

    pub fn setval(&self, name: &str, value: i64) -> Result<()> {
        let mut sequences = self.sequences.write().unwrap();
        if let Some(seq) = sequences.get_mut(name) {
            seq.current_value = value;
            self.persist_sequence(&*seq)?;
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Sequence '{}' not found",
                name
            )))
        }
    }

    fn persist_sequence(&self, seq: &RelationalSequence) -> Result<()> {
        let seq_tree = self.db.open_tree("_sequences")?;
        seq_tree.insert(&seq.name, serde_json::to_vec(seq)?)?;
        Ok(())
    }

    // ─────────────────────────────────────────────
    // View Methods
    // ─────────────────────────────────────────────

    pub fn create_view(
        &self,
        name: &str,
        query_definition: serde_json::Value,
        columns: Vec<String>,
        referenced_tables: Vec<String>,
    ) -> Result<()> {
        let mut views = self.views.write().unwrap();
        if views.contains_key(name) {
            return Err(crate::Error::DatabaseError(format!(
                "View '{}' already exists",
                name
            )));
        }
        let view = RelationalView {
            name: name.to_string(),
            query_definition,
            columns,
            referenced_tables,
            cached_data: Vec::new(),
        };
        views.insert(name.to_string(), view);
        self.persist_view(name)?;
        info!("Created view: {}", name);
        Ok(())
    }

    pub fn drop_view(&self, name: &str) -> Result<()> {
        let mut views = self.views.write().unwrap();
        if views.remove(name).is_some() {
            let view_tree = self.db.open_tree("_views")?;
            view_tree.remove(name)?;
            info!("Dropped view: {}", name);
        }
        Ok(())
    }

    pub fn refresh_view(&self, name: &str) -> Result<()> {
        let views = self.views.read().unwrap();
        if let Some(view) = views.get(name) {
            let referenced = view.referenced_tables.clone();
            let qd = view.query_definition.clone();
            let cols = view.columns.clone();
            drop(views);

            let mut cached = Vec::new();
            for ref_table in &referenced {
                if let Ok(QueryResult::Records(records)) =
                    self.execute_select(ref_table, Some(&cols), Some(&qd), None, false, u64::MAX, 0)
                {
                    for record in records {
                        if let Some(obj) = record.data.as_object() {
                            cached.push(obj.clone());
                        }
                    }
                }
            }

            let mut views = self.views.write().unwrap();
            if let Some(view) = views.get_mut(name) {
                view.cached_data = cached;
            }
            info!("Refreshed view: {}", name);
        }
        Ok(())
    }

    pub fn query_view(
        &self,
        name: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
    ) -> Result<QueryResult> {
        let views = self.views.read().unwrap();
        if let Some(view) = views.get(name) {
            let mut records = Vec::new();
            for data in &view.cached_data {
                let should_include = if let Some(cond) = conditions {
                    self.evaluate_condition(cond, data)?
                } else {
                    true
                };
                if should_include {
                    records.push(Record {
                        id: String::new(),
                        data: serde_json::Value::Object(data.clone()),
                        metadata: HashMap::new(),
                    });
                }
            }
            let records: Vec<Record> = records
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();
            Ok(QueryResult::Records(records))
        } else {
            Err(crate::Error::DatabaseError(format!(
                "View '{}' not found",
                name
            )))
        }
    }

    fn persist_view(&self, name: &str) -> Result<()> {
        let views = self.views.read().unwrap();
        if let Some(view) = views.get(name) {
            let view_tree = self.db.open_tree("_views")?;
            view_tree.insert(name, serde_json::to_vec(view)?)?;
        }
        Ok(())
    }

    // ─────────────────────────────────────────────
    // Trigger Methods
    // ─────────────────────────────────────────────

    pub fn create_trigger(
        &self,
        name: &str,
        table_name: &str,
        timing: TriggerTiming,
        event: TriggerEvent,
        operation: TriggerOperation,
    ) -> Result<()> {
        let mut triggers = self.triggers.write().unwrap();
        let table_triggers = triggers.entry(table_name.to_string()).or_default();
        if table_triggers.iter().any(|t| t.name == name) {
            return Err(crate::Error::DatabaseError(format!(
                "Trigger '{}' already exists on table '{}'",
                name, table_name
            )));
        }
        table_triggers.push(Trigger {
            name: name.to_string(),
            table_name: table_name.to_string(),
            timing,
            event,
            operation,
            enabled: true,
        });
        self.persist_triggers(table_name)?;
        info!("Created trigger {} on table {}", name, table_name);
        Ok(())
    }

    pub fn drop_trigger(&self, table_name: &str, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write().unwrap();
        if let Some(table_triggers) = triggers.get_mut(table_name) {
            table_triggers.retain(|t| t.name != name);
            self.persist_triggers(table_name)?;
            info!("Dropped trigger {} from table {}", name, table_name);
        }
        Ok(())
    }

    pub fn fire_triggers(
        &self,
        table_name: &str,
        event: &TriggerEvent,
        _row_data: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        let triggers = self.triggers.read().unwrap();
        if let Some(table_triggers) = triggers.get(table_name) {
            for trigger in table_triggers {
                if !trigger.enabled {
                    continue;
                }
                let matches = match &trigger.event {
                    TriggerEvent::All => true,
                    e => e == event,
                };
                if matches {
                    match &trigger.operation {
                        TriggerOperation::Raise(msg) => {
                            info!(
                                "Trigger '{}' raised: {} (table: {}, event: {:?})",
                                trigger.name, msg, table_name, event
                            );
                        }
                        TriggerOperation::Function(func) => {
                            info!(
                                "Trigger '{}' function: {} (table: {}, event: {:?})",
                                trigger.name, func, table_name, event
                            );
                        }
                        TriggerOperation::Execute(cmd) => {
                            info!(
                                "Trigger '{}' execute: {} (table: {}, event: {:?})",
                                trigger.name, cmd, table_name, event
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn persist_triggers(&self, table_name: &str) -> Result<()> {
        let triggers = self.triggers.read().unwrap();
        let trig_tree = self.db.open_tree("_triggers")?;
        if let Some(table_triggers) = triggers.get(table_name) {
            trig_tree.insert(table_name, serde_json::to_vec(table_triggers)?)?;
        } else {
            trig_tree.remove(table_name)?;
        }
        Ok(())
    }

    // ─────────────────────────────────────────────
    // DDL Methods
    // ─────────────────────────────────────────────

    pub fn alter_table_add_column(&self, table: &str, field: Field) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table_data) = tables.get_mut(table) {
            if table_data
                .schema
                .fields
                .iter()
                .any(|f| f.name == field.name)
            {
                return Err(crate::Error::DatabaseError(format!(
                    "Column '{}' already exists in table '{}'",
                    field.name, table
                )));
            }
            table_data.schema.fields.push(field);
            let schemas_tree = self.db.open_tree("_schemas")?;
            schemas_tree.insert(table, serde_json::to_vec(&table_data.schema)?)?;
            table_data.updated_at = chrono::Utc::now();
            self.persist_updated_at(table, &table_data.updated_at)?;
            info!("Added column to table {}", table);
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    pub fn alter_table_drop_column(&self, table: &str, column_name: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table_data) = tables.get_mut(table) {
            table_data.schema.fields.retain(|f| f.name != column_name);
            for row in table_data.rows.values_mut() {
                row.data.remove(column_name);
            }
            let schemas_tree = self.db.open_tree("_schemas")?;
            schemas_tree.insert(table, serde_json::to_vec(&table_data.schema)?)?;
            table_data.updated_at = chrono::Utc::now();
            self.persist_updated_at(table, &table_data.updated_at)?;
            info!("Dropped column {} from table {}", column_name, table);
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    pub fn alter_table_modify_column(&self, table: &str, field: Field) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table_data) = tables.get_mut(table) {
            if let Some(existing) = table_data
                .schema
                .fields
                .iter_mut()
                .find(|f| f.name == field.name)
            {
                existing.field_type = field.field_type;
                existing.nullable = field.nullable;
                existing.default_value = field.default_value;
                existing.constraints = field.constraints;
            } else {
                return Err(crate::Error::DatabaseError(format!(
                    "Column '{}' not found in table '{}'",
                    field.name, table
                )));
            }
            let schemas_tree = self.db.open_tree("_schemas")?;
            schemas_tree.insert(table, serde_json::to_vec(&table_data.schema)?)?;
            table_data.updated_at = chrono::Utc::now();
            self.persist_updated_at(table, &table_data.updated_at)?;
            info!("Modified column {} in table {}", field.name, table);
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    pub fn alter_table_add_constraint(&self, table: &str, constraint: Constraint) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table_data) = tables.get_mut(table) {
            if table_data
                .schema
                .constraints
                .iter()
                .any(|c| c.name == constraint.name)
            {
                return Err(crate::Error::DatabaseError(format!(
                    "Constraint '{}' already exists on table '{}'",
                    constraint.name, table
                )));
            }
            table_data.schema.constraints.push(constraint);
            let schemas_tree = self.db.open_tree("_schemas")?;
            schemas_tree.insert(table, serde_json::to_vec(&table_data.schema)?)?;
            table_data.updated_at = chrono::Utc::now();
            self.persist_updated_at(table, &table_data.updated_at)?;
            info!("Added constraint to table {}", table);
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    pub fn alter_table_drop_constraint(&self, table: &str, constraint_name: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table_data) = tables.get_mut(table) {
            table_data
                .schema
                .constraints
                .retain(|c| c.name != constraint_name);
            let schemas_tree = self.db.open_tree("_schemas")?;
            schemas_tree.insert(table, serde_json::to_vec(&table_data.schema)?)?;
            table_data.updated_at = chrono::Utc::now();
            self.persist_updated_at(table, &table_data.updated_at)?;
            info!(
                "Dropped constraint {} from table {}",
                constraint_name, table
            );
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    pub fn rename_table(&self, old_name: &str, new_name: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if tables.contains_key(new_name) {
            return Err(crate::Error::DatabaseError(format!(
                "Table '{}' already exists",
                new_name
            )));
        }
        if let Some(mut table_data) = tables.remove(old_name) {
            table_data.name = new_name.to_string();

            let old_tree_name = format!("table:{}", old_name);
            let new_tree_name = format!("table:{}", new_name);

            if let Ok(old_tree) = self.db.open_tree(&old_tree_name) {
                let new_tree = self.db.open_tree(&new_tree_name)?;
                for result in old_tree.iter() {
                    if let Ok((key, value)) = result {
                        new_tree.insert(key, value)?;
                    }
                }
                self.db.drop_tree(old_tree_name)?;
            }

            let schemas_tree = self.db.open_tree("_schemas")?;
            if let Some(schema_val) = schemas_tree.get(old_name)? {
                schemas_tree.insert(new_name, schema_val)?;
                schemas_tree.remove(old_name)?;
            }

            let next_ids_tree = self.db.open_tree("_next_ids")?;
            if let Some(val) = next_ids_tree.get(old_name)? {
                next_ids_tree.insert(new_name, val)?;
                next_ids_tree.remove(old_name)?;
            }

            let created_at_tree = self.db.open_tree("_created_at")?;
            if let Some(val) = created_at_tree.get(old_name)? {
                created_at_tree.insert(new_name, val)?;
                created_at_tree.remove(old_name)?;
            }

            let updated_at_tree = self.db.open_tree("_updated_at")?;
            if let Some(val) = updated_at_tree.get(old_name)? {
                updated_at_tree.insert(new_name, val)?;
                updated_at_tree.remove(old_name)?;
            }

            tables.insert(new_name.to_string(), table_data);
            info!("Renamed table {} to {}", old_name, new_name);
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                old_name
            )))
        }
    }

    // ─────────────────────────────────────────────
    // Enhanced Query Methods
    // ─────────────────────────────────────────────

    fn execute_insert_returning(
        &self,
        table: &str,
        data: &serde_json::Map<String, serde_json::Value>,
        returning: &[String],
    ) -> Result<QueryResult> {
        let node_id = &self.config.cluster.node_id;
        let row = Row::new_row(0, data.clone(), node_id);

        self.validate_foreign_key_on_insert(table, &row)?;

        let mut tables = self.tables.write().unwrap();
        let table_entry = tables
            .entry(table.to_string())
            .or_insert_with(|| RelationalTable {
                name: table.to_string(),
                schema: Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                },
                rows: HashMap::new(),
                next_id: 1,
                indexes: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            });

        let id = table_entry.next_id;
        table_entry.next_id += 1;

        let row = Row::new_row(id, data.clone(), node_id);

        self.persist_row(table, &row)?;
        self.persist_next_id(table, table_entry.next_id)?;

        table_entry.rows.insert(id, row);

        let selected_data = if returning.is_empty() {
            data.clone()
        } else {
            let mut filtered = serde_json::Map::new();
            for field in returning {
                if let Some(val) = data.get(field) {
                    filtered.insert(field.clone(), val.clone());
                }
            }
            filtered
        };

        Ok(QueryResult::Records(vec![Record {
            id: id.to_string(),
            data: serde_json::Value::Object(selected_data),
            metadata: HashMap::new(),
        }]))
    }

    fn execute_update_returning(
        &self,
        table: &str,
        data: &serde_json::Map<String, serde_json::Value>,
        conditions: Option<&serde_json::Value>,
        returning: &[String],
    ) -> Result<QueryResult> {
        let mut tables = self.tables.write().unwrap();
        let mut updated_records = Vec::new();

        if let Some(table_data) = tables.get_mut(table) {
            let ids_to_update: Vec<u64> = table_data
                .rows
                .iter()
                .filter(|(_, row)| {
                    conditions.map_or(true, |cond| {
                        self.evaluate_condition(cond, &row.data).unwrap_or(false)
                    })
                })
                .map(|(id, _)| *id)
                .collect();

            for id in ids_to_update {
                if let Some(row) = table_data.rows.get_mut(&id) {
                    self.check_referential_integrity(table, &row.data, data)?;
                    row.data = data.clone();
                    row.increment_version(&self.config.cluster.node_id);
                    self.persist_row(table, row)?;

                    let selected_data = if returning.is_empty() {
                        row.data.clone()
                    } else {
                        let mut filtered = serde_json::Map::new();
                        for field in returning {
                            if let Some(val) = row.data.get(field) {
                                filtered.insert(field.clone(), val.clone());
                            }
                        }
                        filtered
                    };

                    updated_records.push(Record {
                        id: id.to_string(),
                        data: serde_json::Value::Object(selected_data),
                        metadata: HashMap::new(),
                    });
                }
            }

            if !updated_records.is_empty() {
                table_data.updated_at = chrono::Utc::now();
                self.persist_updated_at(table, &table_data.updated_at)?;
            }
        }

        Ok(QueryResult::Records(updated_records))
    }

    fn execute_delete_returning(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        returning: &[String],
    ) -> Result<QueryResult> {
        let mut tables = self.tables.write().unwrap();
        let mut deleted_records = Vec::new();
        let mut to_delete: Vec<u64> = Vec::new();

        if let Some(table_data) = tables.get_mut(table) {
            for (id, row) in table_data.rows.iter() {
                let should_delete = if let Some(cond) = conditions {
                    self.evaluate_condition(cond, &row.data)?
                } else {
                    true
                };

                if should_delete {
                    if let Ok(action) = self.validate_foreign_key_on_delete(table, *id) {
                        match action {
                            CascadeAction::Allow | CascadeAction::NoAction => {
                                to_delete.push(*id);
                            }
                            CascadeAction::Cascade => {
                                self.cascade_delete(table, *id)?;
                                to_delete.push(*id);
                            }
                            CascadeAction::SetNull => {
                                self.set_null_foreign_keys(table, *id)?;
                                to_delete.push(*id);
                            }
                            CascadeAction::SetDefault => {
                                self.set_default_foreign_keys(table, *id)?;
                                to_delete.push(*id);
                            }
                            CascadeAction::Restrict => {}
                        }
                    }
                }
            }

            for id in &to_delete {
                if let Some(row) = table_data.rows.get(id) {
                    let selected_data = if returning.is_empty() {
                        row.data.clone()
                    } else {
                        let mut filtered = serde_json::Map::new();
                        for field in returning {
                            if let Some(val) = row.data.get(field) {
                                filtered.insert(field.clone(), val.clone());
                            }
                        }
                        filtered
                    };
                    deleted_records.push(Record {
                        id: id.to_string(),
                        data: serde_json::Value::Object(selected_data),
                        metadata: HashMap::new(),
                    });
                }
                table_data.rows.remove(id);
                self.remove_row(table, *id)?;
            }

            if !to_delete.is_empty() {
                table_data.updated_at = chrono::Utc::now();
                self.persist_updated_at(table, &table_data.updated_at)?;
            }
        }

        Ok(QueryResult::Records(deleted_records))
    }

    fn execute_select_grouped(
        &self,
        table: &str,
        fields: Option<&[String]>,
        conditions: Option<&serde_json::Value>,
        group_by: &[String],
        having: &Option<serde_json::Value>,
        _order_by: Option<&str>,
        limit: u64,
        offset: u64,
        _distinct: bool,
    ) -> Result<QueryResult> {
        let tables = self.tables.read().unwrap();

        if let Some(table_data) = tables.get(table) {
            let mut records = Vec::new();

            for row in table_data.rows.values() {
                let should_include = if let Some(cond) = conditions {
                    self.evaluate_condition(cond, &row.data)?
                } else {
                    true
                };

                if should_include {
                    let selected_data = if let Some(fields) = fields {
                        let mut data = serde_json::Map::new();
                        for field in fields {
                            if let Some(val) = row.data.get(field) {
                                data.insert(field.clone(), val.clone());
                            }
                        }
                        data
                    } else {
                        row.data.clone()
                    };

                    records.push(Record {
                        id: row.id.to_string(),
                        data: serde_json::Value::Object(selected_data),
                        metadata: HashMap::new(),
                    });
                }
            }

            if !group_by.is_empty() {
                let mut grouped: HashMap<String, Vec<Record>> = HashMap::new();
                for record in records {
                    let group_key = record
                        .data
                        .as_object()
                        .map(|obj| {
                            group_by
                                .iter()
                                .map(|g| obj.get(g).map(|v| v.to_string()).unwrap_or_default())
                                .collect::<Vec<_>>()
                                .join("\x00")
                        })
                        .unwrap_or_default();
                    grouped.entry(group_key).or_default().push(record);
                }

                records = grouped
                    .into_values()
                    .filter_map(|g| g.into_iter().next())
                    .collect();
            }

            if let Some(having_cond) = having {
                records.retain(|r| {
                    r.data
                        .as_object()
                        .map(|obj| self.evaluate_condition(having_cond, obj).unwrap_or(false))
                        .unwrap_or(false)
                });
            }

            let records: Vec<Record> = records
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();

            Ok(QueryResult::Records(records))
        } else {
            Ok(QueryResult::Records(vec![]))
        }
    }

    fn execute_truncate(&self, table: &str, cascade: bool) -> Result<QueryResult> {
        if cascade {
            let child_tables: Vec<String> = {
                let foreign_keys = self.foreign_keys.read().unwrap();
                foreign_keys
                    .iter()
                    .filter(|(_, fks)| fks.iter().any(|fk| fk.to_table == table))
                    .map(|(ft, _)| ft.clone())
                    .collect()
            };
            for child in child_tables {
                self.execute_truncate(&child, false)?;
            }
        }

        let mut tables = self.tables.write().unwrap();
        if let Some(table_data) = tables.get_mut(table) {
            let count = table_data.rows.len() as u64;
            table_data.rows.clear();
            table_data.next_id = 1;
            table_data.updated_at = chrono::Utc::now();

            let table_tree = self.db.open_tree(format!("table:{}", table))?;
            table_tree.clear()?;
            self.persist_next_id(table, 1)?;
            self.persist_updated_at(table, &table_data.updated_at)?;

            info!(
                "Truncated table: {} ({} rows removed, cascade: {})",
                table, count, cascade
            );
            Ok(QueryResult::AffectedRows(count))
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    // ─────────────────────────────────────────────
    // Cascade Methods
    // ─────────────────────────────────────────────

    pub fn cascade_delete(&self, table_name: &str, row_id: u64) -> Result<()> {
        let child_entries: Vec<(String, u64)> = {
            let foreign_keys = self.foreign_keys.read().unwrap();
            let tables = self.tables.read().unwrap();
            let mut entries = Vec::new();

            for (fk_table, fks) in foreign_keys.iter() {
                for fk in fks {
                    if fk.to_table == table_name && fk.on_delete == CascadeAction::Cascade {
                        if let Some(child_table) = tables.get(fk_table) {
                            if let Some(parent_table) = tables.get(table_name) {
                                if let Some(parent_row) = parent_table.rows.get(&row_id) {
                                    if let Some(parent_val) = parent_row.data.get(&fk.to_column) {
                                        for child_row in child_table.rows.values() {
                                            if child_row.data.get(&fk.from_column)
                                                == Some(parent_val)
                                            {
                                                entries.push((fk_table.clone(), child_row.id));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            entries
        };

        for (child_table, child_id) in child_entries {
            self.cascade_delete(&child_table, child_id)?;
            {
                let mut tables = self.tables.write().unwrap();
                if let Some(t) = tables.get_mut(&child_table) {
                    t.rows.remove(&child_id);
                }
            }
            self.remove_row(&child_table, child_id)?;
        }
        Ok(())
    }

    pub fn cascade_update(
        &self,
        table_name: &str,
        row_id: u64,
        _old_values: &serde_json::Map<String, serde_json::Value>,
        new_values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<()> {
        let update_entries: Vec<(String, u64)> = {
            let foreign_keys = self.foreign_keys.read().unwrap();
            let tables = self.tables.read().unwrap();
            let mut entries = Vec::new();

            for (fk_table, fks) in foreign_keys.iter() {
                for fk in fks {
                    if fk.to_table == table_name && fk.on_update == CascadeAction::Cascade {
                        if let Some(child_table) = tables.get(fk_table) {
                            if let Some(parent_table) = tables.get(table_name) {
                                if let Some(parent_row) = parent_table.rows.get(&row_id) {
                                    if let Some(parent_val) = parent_row.data.get(&fk.to_column) {
                                        for child_row in child_table.rows.values() {
                                            if child_row.data.get(&fk.from_column)
                                                == Some(parent_val)
                                            {
                                                entries.push((fk_table.clone(), child_row.id));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            entries
        };

        for (child_table, child_id) in update_entries {
            {
                let mut tables = self.tables.write().unwrap();
                if let Some(t) = tables.get_mut(&child_table) {
                    if let Some(child_row) = t.rows.get_mut(&child_id) {
                        let foreign_keys = self.foreign_keys.read().unwrap();
                        if let Some(fks) = foreign_keys.get(&child_table) {
                            for fk in fks {
                                if fk.to_table == table_name {
                                    if let Some(new_val) = new_values.get(&fk.to_column) {
                                        child_row
                                            .data
                                            .insert(fk.from_column.clone(), new_val.clone());
                                    }
                                }
                            }
                        }
                        child_row.increment_version(&self.config.cluster.node_id);
                        let row_copy = child_row.clone();
                        self.persist_row(&child_table, &row_copy)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_null_foreign_keys(&self, table_name: &str, row_id: u64) -> Result<()> {
        let foreign_keys = self.foreign_keys.read().unwrap();
        let tables = self.tables.read().unwrap();

        for (fk_table, fks) in foreign_keys.iter() {
            for fk in fks {
                if fk.to_table == table_name && fk.on_delete == CascadeAction::SetNull {
                    if let Some(child_table) = tables.get(fk_table) {
                        if let Some(parent_table) = tables.get(table_name) {
                            if let Some(parent_row) = parent_table.rows.get(&row_id) {
                                if let Some(parent_val) = parent_row.data.get(&fk.to_column) {
                                    for child_row in child_table.rows.values() {
                                        if child_row.data.get(&fk.from_column) == Some(parent_val) {
                                            let mut tables = self.tables.write().unwrap();
                                            if let Some(t) = tables.get_mut(fk_table) {
                                                if let Some(cr) = t.rows.get_mut(&child_row.id) {
                                                    cr.data.insert(
                                                        fk.from_column.clone(),
                                                        serde_json::Value::Null,
                                                    );
                                                    cr.increment_version(&self.config.cluster.node_id);
                                                    let row_copy = cr.clone();
                                                    self.persist_row(fk_table, &row_copy)?;
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
        Ok(())
    }

    pub fn set_default_foreign_keys(&self, table_name: &str, row_id: u64) -> Result<()> {
        let foreign_keys = self.foreign_keys.read().unwrap();
        let tables = self.tables.read().unwrap();

        for (fk_table, fks) in foreign_keys.iter() {
            for fk in fks {
                if fk.to_table == table_name && fk.on_delete == CascadeAction::SetDefault {
                    if let Some(child_table) = tables.get(fk_table) {
                        if let Some(parent_table) = tables.get(table_name) {
                            if let Some(parent_row) = parent_table.rows.get(&row_id) {
                                if let Some(parent_val) = parent_row.data.get(&fk.to_column) {
                                    for child_row in child_table.rows.values() {
                                        if child_row.data.get(&fk.from_column) == Some(parent_val) {
                                            let mut tables = self.tables.write().unwrap();
                                            if let Some(t) = tables.get_mut(fk_table) {
                                                if let Some(cr) = t.rows.get_mut(&child_row.id) {
                                                    let default_val = t
                                                        .schema
                                                        .fields
                                                        .iter()
                                                        .find(|f| f.name == fk.from_column)
                                                        .and_then(|f| f.default_value.clone())
                                                        .unwrap_or(serde_json::Value::Null);
                                                    cr.data.insert(
                                                        fk.from_column.clone(),
                                                        default_val,
                                                    );
                                                    cr.increment_version(&self.config.cluster.node_id);
                                                    let row_copy = cr.clone();
                                                    self.persist_row(fk_table, &row_copy)?;
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
        Ok(())
    }

    // ─────────────────────────────────────────────
    // Cluster Reconciliation Methods (Hyperledger-style)
    // ─────────────────────────────────────────────

    pub fn get_rows_for_reconciliation(
        &self,
        table: &str,
    ) -> Result<Vec<(String, HashMap<String, u64>, u64, String, serde_json::Map<String, serde_json::Value>)>>
    {
        let tables = self.tables.read().unwrap();
        if let Some(table_data) = tables.get(table) {
            let mut rows = Vec::new();
            for row in table_data.rows.values() {
                rows.push((
                    row.id.to_string(),
                    row.vector_clock.clone(),
                    row.version,
                    row.checksum.clone(),
                    row.data.clone(),
                ));
            }
            Ok(rows)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn compute_table_merkle_root(&self, table: &str) -> Result<String> {
        use sha2::Digest;
        let tables = self.tables.read().unwrap();
        if let Some(table_data) = tables.get(table) {
            let mut hashes: Vec<String> = table_data
                .rows
                .values()
                .map(|row| {
                    let h = sha2::Sha256::digest(
                        format!("{}:{}:{}", row.id, row.checksum, row.version).as_bytes(),
                    );
                    format!("{:x}", h)
                })
                .collect();
            hashes.sort();
            if hashes.is_empty() {
                return Ok("empty".to_string());
            }
            while hashes.len() > 1 {
                let mut new_hashes = Vec::new();
                for chunk in hashes.chunks(2) {
                    if chunk.len() == 2 {
                        let combined = format!("{}{}", chunk[0], chunk[1]);
                        new_hashes
                            .push(format!("{:x}", sha2::Sha256::digest(combined.as_bytes())));
                    } else {
                        new_hashes.push(chunk[0].clone());
                    }
                }
                hashes = new_hashes;
            }
            Ok(hashes[0].clone())
        } else {
            Ok("empty".to_string())
        }
    }

    pub fn apply_reconciled_rows(
        &self,
        table: &str,
        rows: Vec<(
            String,
            HashMap<String, u64>,
            u64,
            String,
            serde_json::Map<String, serde_json::Value>,
        )>,
    ) -> Result<u64> {
        let mut tables = self.tables.write().unwrap();
        let mut applied = 0u64;
        if let Some(table_data) = tables.get_mut(table) {
            for (id_str, vc, version, checksum, data) in rows {
                let id: u64 = id_str.parse().unwrap_or(0);
                let existing = table_data.rows.get(&id);
                let should_apply = match existing {
                    None => true,
                    Some(existing_row) => {
                        let existing_vc = &existing_row.vector_clock;
                        let incoming_vc = &vc;
                        let mut incoming_newer = false;
                        let mut existing_newer = false;
                        for (node, clock) in incoming_vc {
                            let existing_clock = existing_vc.get(node).unwrap_or(&0);
                            if clock > existing_clock {
                                incoming_newer = true;
                            }
                            if clock < existing_clock {
                                existing_newer = true;
                            }
                        }
                        for (node, clock) in existing_vc {
                            if !incoming_vc.contains_key(node) && *clock > 0 {
                                existing_newer = true;
                            }
                        }
                        incoming_newer && !existing_newer
                    }
                };
                if should_apply {
                    let row = Row {
                        id,
                        data,
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        version,
                        vector_clock: vc,
                        checksum,
                    };
                    table_data.rows.insert(id, row);
                    applied += 1;
                }
            }
            if applied > 0 {
                table_data.updated_at = chrono::Utc::now();
            }
        }
        Ok(applied)
    }

    // ─────────────────────────────────────────────
    // Information Schema Methods
    // ─────────────────────────────────────────────

    pub fn get_information_schema_tables(&self) -> Result<QueryResult> {
        let tables = self.tables.read().unwrap();
        let mut records = Vec::new();

        for (name, table_data) in tables.iter() {
            let info = serde_json::json!({
                "table_name": name,
                "table_type": "BASE TABLE",
                "row_count": table_data.rows.len(),
                "column_count": table_data.schema.fields.len(),
                "index_count": table_data.indexes.len(),
                "constraint_count": table_data.schema.constraints.len(),
                "created_at": table_data.created_at.to_rfc3339(),
                "updated_at": table_data.updated_at.to_rfc3339(),
            });
            if let Some(obj) = info.as_object() {
                records.push(Record {
                    id: name.clone(),
                    data: serde_json::Value::Object(obj.clone()),
                    metadata: HashMap::new(),
                });
            }
        }

        {
            let views = self.views.read().unwrap();
            for (name, view) in views.iter() {
                let info = serde_json::json!({
                    "table_name": name,
                    "table_type": "VIEW",
                    "row_count": view.cached_data.len(),
                    "column_count": view.columns.len(),
                    "index_count": 0,
                    "constraint_count": 0,
                    "referenced_tables": view.referenced_tables,
                });
                if let Some(obj) = info.as_object() {
                    records.push(Record {
                        id: name.clone(),
                        data: serde_json::Value::Object(obj.clone()),
                        metadata: HashMap::new(),
                    });
                }
            }
        }

        Ok(QueryResult::Records(records))
    }

    pub fn get_information_schema_columns(&self, table: &str) -> Result<QueryResult> {
        let tables = self.tables.read().unwrap();
        let mut records = Vec::new();

        if let Some(table_data) = tables.get(table) {
            for field in &table_data.schema.fields {
                let info = serde_json::json!({
                    "table_name": table,
                    "column_name": field.name,
                    "data_type": format!("{:?}", field.field_type),
                    "nullable": field.nullable,
                    "default_value": field.default_value,
                    "ordinal_position": table_data.schema.fields.iter()
                        .position(|f| f.name == field.name).unwrap_or(0) + 1,
                });
                if let Some(obj) = info.as_object() {
                    records.push(Record {
                        id: format!("{}.{}", table, field.name),
                        data: serde_json::Value::Object(obj.clone()),
                        metadata: HashMap::new(),
                    });
                }
            }
            Ok(QueryResult::Records(records))
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    pub fn get_information_schema_constraints(&self, table: &str) -> Result<QueryResult> {
        let tables = self.tables.read().unwrap();
        let mut records = Vec::new();

        if let Some(table_data) = tables.get(table) {
            for constraint in &table_data.schema.constraints {
                let constraint_type = match &constraint.constraint_type {
                    ConstraintType::PrimaryKey => "PRIMARY KEY",
                    ConstraintType::ForeignKey { .. } => "FOREIGN KEY",
                    ConstraintType::Unique => "UNIQUE",
                    ConstraintType::Check { .. } => "CHECK",
                    ConstraintType::NotNull => "NOT NULL",
                    ConstraintType::DefaultValue { .. } => "DEFAULT",
                    ConstraintType::Generated { .. } => "GENERATED",
                };
                let info = serde_json::json!({
                    "table_name": table,
                    "constraint_name": constraint.name,
                    "constraint_type": constraint_type,
                    "fields": constraint.fields,
                    "definition": constraint.definition,
                });
                if let Some(obj) = info.as_object() {
                    records.push(Record {
                        id: format!("{}.{}", table, constraint.name),
                        data: serde_json::Value::Object(obj.clone()),
                        metadata: HashMap::new(),
                    });
                }
            }
            Ok(QueryResult::Records(records))
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }
}

#[derive(Debug)]
pub struct JoinCondition {
    left_field: String,
    right_field: String,
    join_type: JoinType,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum CascadeAction {
    Allow,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
    NoAction,
}

#[derive(Debug)]
struct JoinedRow {
    left_row: Row,
    right_row: Option<Row>,
}

#[async_trait]
impl StorageEngine for RelationalEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let data_obj = data.as_object().ok_or_else(|| {
            crate::Error::ValidationError("Data must be a JSON object".to_string())
        })?;

        let node_id = &self.config.cluster.node_id;
        let row = Row::new_row(0, data_obj.clone(), node_id);

        self.validate_foreign_key_on_insert(table, &row)?;

        let mut tables = self.tables.write().unwrap();
        let _is_new = !tables.contains_key(table);
        let table_entry = tables
            .entry(table.to_string())
            .or_insert_with(|| {
                let fields: Vec<Field> = data_obj
                    .iter()
                    .map(|(name, val)| Field {
                        name: name.clone(),
                        field_type: json_to_field_type(val),
                        nullable: true,
                        default_value: None,
                        constraints: vec![],
                    })
                    .collect();
                RelationalTable {
                    name: table.to_string(),
                    schema: Schema {
                        fields,
                        indexes: vec![],
                        constraints: vec![],
                    },
                    rows: HashMap::new(),
                    next_id: 1,
                    indexes: HashMap::new(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }
            });

        let id = table_entry.next_id;
        table_entry.next_id += 1;

        let row = Row::new_row(id, data_obj.clone(), node_id);

        self.persist_row(table, &row)?;
        self.persist_next_id(table, table_entry.next_id)?;

        table_entry.rows.insert(id, row);

        info!("Inserted row {} into table {}", id, table);
        Ok(1)
    }

    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let tables = self.tables.read().unwrap();

        if let Some(table_data) = tables.get(table) {
            let mut records = Vec::new();

            for row in table_data.rows.values() {
                let should_include = match conditions {
                    Some(cond) => self.evaluate_condition(cond, &row.data)?,
                    None => true,
                };

                if should_include {
                    records.push(Record {
                        id: row.id.to_string(),
                        data: serde_json::Value::Object(row.data.clone()),
                        metadata: HashMap::new(),
                    });
                }
            }

            let records: Vec<Record> = records
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect();

            Ok(records)
        } else {
            Ok(vec![])
        }
    }

    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let data_obj = data.as_object().ok_or_else(|| {
            crate::Error::ValidationError("Update data must be a JSON object".to_string())
        })?;

        let mut tables = self.tables.write().unwrap();
        let mut affected = 0u64;

        if let Some(table_data) = tables.get_mut(table) {
            let ids_to_update: Vec<u64> = table_data
                .rows
                .iter()
                .filter(|(_, row)| {
                    conditions.map_or(true, |cond| {
                        self.evaluate_condition(cond, &row.data).unwrap_or(false)
                    })
                })
                .map(|(id, _)| *id)
                .collect();

            for id in ids_to_update {
                if let Some(row) = table_data.rows.get_mut(&id) {
                    self.check_referential_integrity(table, &row.data, data_obj)?;
                    for (k, v) in data_obj {
                        row.data.insert(k.clone(), v.clone());
                    }
                    row.increment_version(&self.config.cluster.node_id);
                    self.persist_row(table, row)?;
                    affected += 1;
                }
            }

            if affected > 0 {
                table_data.updated_at = chrono::Utc::now();
                self.persist_updated_at(table, &table_data.updated_at)?;
            }
        }

        info!("Updated {} rows in table {}", affected, table);
        Ok(affected)
    }

    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let to_delete: Vec<u64> = {
            let tables = self.tables.read().unwrap();
            let mut ids = Vec::new();
            if let Some(table_data) = tables.get(table) {
                for (id, row) in table_data.rows.iter() {
                    let should_delete = if let Some(cond) = conditions {
                        self.evaluate_condition(cond, &row.data)?
                    } else {
                        true
                    };

                    if should_delete {
                        if let Ok(action) = self.validate_foreign_key_on_delete(table, *id) {
                            if matches!(action, CascadeAction::Allow) {
                                ids.push(*id);
                            }
                        }
                    }
                }
            }
            ids
        };

        let mut affected = 0u64;
        if !to_delete.is_empty() {
            let mut tables = self.tables.write().unwrap();
            if let Some(table_data) = tables.get_mut(table) {
                for id in &to_delete {
                    table_data.rows.remove(id);
                    self.remove_row(table, *id)?;
                    affected += 1;
                }

                if affected > 0 {
                    table_data.updated_at = chrono::Utc::now();
                    self.persist_updated_at(table, &table_data.updated_at)?;
                }
            }
        }

        info!("Deleted {} rows from table {}", affected, table);
        Ok(affected)
    }

    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        let tables = self.tables.read().unwrap();

        if let Some(table_data) = tables.get(table) {
            let row_count = table_data.rows.len() as u64;
            let index_count = table_data.indexes.len() as u64;

            let analysis = serde_json::json!({
                "table_name": table,
                "row_count": row_count,
                "index_count": index_count,
                "columns": table_data.schema.fields.iter().map(|f| {
                    serde_json::json!({
                        "name": f.name,
                        "field_type": format!("{:?}", f.field_type),
                        "nullable": f.nullable,
                    })
                }).collect::<Vec<_>>(),
            });

            info!("Analyzed table: {}", table);
            Ok(serde_json::to_string_pretty(&analysis)?)
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }

    async fn create_table(&self, table: &str, schema: &Schema) -> Result<()> {
        let mut tables = self.tables.write().unwrap();

        if tables.contains_key(table) {
            return Err(crate::Error::DatabaseError(format!(
                "Table '{}' already exists",
                table
            )));
        }

        let now = chrono::Utc::now();

        let relational_table = RelationalTable {
            name: table.to_string(),
            schema: schema.clone(),
            rows: HashMap::new(),
            next_id: 1,
            indexes: HashMap::new(),
            created_at: now,
            updated_at: now,
        };

        let schemas_tree = self.db.open_tree("_schemas")?;
        schemas_tree.insert(table, serde_json::to_vec(schema)?)?;

        let next_ids_tree = self.db.open_tree("_next_ids")?;
        next_ids_tree.insert(table, serde_json::to_vec(&1u64)?)?;

        let created_at_tree = self.db.open_tree("_created_at")?;
        created_at_tree.insert(table, serde_json::to_vec(&now)?)?;

        let updated_at_tree = self.db.open_tree("_updated_at")?;
        updated_at_tree.insert(table, serde_json::to_vec(&now)?)?;

        self.db.open_tree(format!("table:{}", table))?;

        tables.insert(table.to_string(), relational_table);

        info!("Created relational table: {}", table);
        Ok(())
    }

    async fn drop_table(&self, table: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        tables.remove(table);

        self.db.drop_tree(format!("table:{}", table))?;

        let schemas_tree = self.db.open_tree("_schemas")?;
        schemas_tree.remove(table)?;

        let next_ids_tree = self.db.open_tree("_next_ids")?;
        next_ids_tree.remove(table)?;

        let created_at_tree = self.db.open_tree("_created_at")?;
        created_at_tree.remove(table)?;

        let updated_at_tree = self.db.open_tree("_updated_at")?;
        updated_at_tree.remove(table)?;

        info!("Dropped relational table: {}", table);
        Ok(())
    }

    async fn truncate_table(&self, table: &str, cascade: bool) -> Result<()> {
        async fn do_truncate(engine: &RelationalEngine, tbl: &str) -> Result<()> {
            let mut tables = engine.tables.write().unwrap();
            if let Some(table_data) = tables.get_mut(tbl) {
                table_data.rows.clear();
                table_data.next_id = 1;
                table_data.updated_at = chrono::Utc::now();
                let table_tree = engine.db.open_tree(format!("table:{}", tbl))?;
                table_tree.clear()?;
                engine.persist_next_id(tbl, 1)?;
                engine.persist_updated_at(tbl, &table_data.updated_at)?;
                info!("Truncated relational table: {}", tbl);
                Ok(())
            } else {
                Err(crate::Error::DatabaseError(format!("Table '{}' not found", tbl)))
            }
        }

        if cascade {
            let child_tables = {
                let foreign_keys = self.foreign_keys.read().unwrap();
                let mut tables_to_truncate = Vec::new();
                for (child_table, fks) in foreign_keys.iter() {
                    if fks.iter().any(|fk| fk.to_table == table) {
                        tables_to_truncate.push(child_table.clone());
                    }
                }
                tables_to_truncate
            };
            for child in &child_tables {
                do_truncate(self, &child).await?;
            }
        }
        do_truncate(self, table).await
    }

    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let tables = self.tables.read().unwrap();

        if let Some(table_data) = tables.get(table) {
            let size_bytes: u64 = table_data
                .rows
                .values()
                .map(|row| serde_json::to_vec(row).map(|v| v.len() as u64).unwrap_or(0))
                .sum();

            info!("Getting table info for: {}", table);
            Ok(TableInfo {
                name: table_data.name.clone(),
                schema: table_data.schema.clone(),
                row_count: table_data.rows.len() as u64,
                size_bytes,
                created_at: table_data.created_at,
                updated_at: table_data.updated_at,
            })
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Table '{}' not found",
                table
            )))
        }
    }
}

fn json_to_field_type(value: &serde_json::Value) -> FieldType {
    match value {
        serde_json::Value::Number(n) => {
            if n.is_f64() || n.as_f64().map_or(false, |f| f.fract() != 0.0) {
                FieldType::Float
            } else {
                FieldType::Integer
            }
        }
        serde_json::Value::String(_) => FieldType::String,
        serde_json::Value::Bool(_) => FieldType::Boolean,
        serde_json::Value::Array(_) => FieldType::Array(Box::new(FieldType::String)),
        serde_json::Value::Object(_) => FieldType::Text,
        serde_json::Value::Null => FieldType::String,
    }
}
