/*
 * PrimusDB Relational Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: as_any() method for engine-specific features
 */

/*!
# PrimusDB Relational Storage Engine

The relational engine implements traditional SQL-like table storage over an
embedded sled database. It manages typed schemas, rows, foreign keys,
auto-increment sequences, views, triggers, joins, grouped queries, and
Hyperledger-style reconciliation (vector clocks, checksums, Merkle roots).
Use it for applications that need fixed schemas, referential integrity, and
relational operations (JOIN, GROUP BY, constraints) on top of a lightweight
embedded store.

```text
Relational Engine Data Flow
═══════════════════════════════════════════════════

RelationalQuery ──► execute_query ──► query executor
                        ├─► SELECT / INSERT / UPDATE / DELETE (+ RETURNING)
                        ├─► JOIN (inner / left / right / full / cross)
                        ├─► GROUP BY / HAVING / ORDER BY
                        └─► TRUNCATE

Tables (schema + rows) ──► sled trees
  table:{name}   row data            _sequences   SERIAL sequences
  _schemas       schema JSON         _views       materialized views
  _next_ids      auto-increment IDs  _triggers    event triggers
  _created_at / _updated_at          metadata timestamps
```

## Main Types & Functions

- [`RelationalEngine`]: the SQL-like relational engine implementing [`StorageEngine`].
- [`RelationalQuery`]: typed query variants executable via `execute_query`.
- [`QueryResult`]: either a set of records or an affected-row count.
- [`ForeignKey`], [`RelationalSequence`], [`RelationalView`], [`Trigger`]: database objects.
- `create_index` / `drop_index` / `analyze_table`: table analysis and indexing.
- `create_sequence` / `nextval` / `currval` / `setval`: sequence management.
- `create_view` / `refresh_view` / `query_view`: view management.
- `create_trigger` / `fire_triggers`: trigger management.
- `alter_table_*` / `rename_table`: DDL operations.
- `cascade_delete` / `cascade_update` / `set_null_foreign_keys` / `set_default_foreign_keys`: referential actions.
- `get_rows_for_reconciliation` / `compute_table_merkle_root` / `apply_reconciled_rows`: cluster reconciliation.
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

/// Variants of relational queries that can be executed against the engine.
#[allow(dead_code)]
pub enum RelationalQuery<'a> {
    /// Select rows from a table with optional field projection, conditions,
    /// pagination, and `DISTINCT` handling. `order_by` is accepted but
    /// currently ignored by the executor.
    Select {
        table: &'a str,
        fields: Option<Vec<String>>,
        conditions: Option<&'a serde_json::Value>,
        order_by: Option<&'a str>,
        limit: u64,
        offset: u64,
        distinct: bool,
    },
    /// Insert a single row into a table.
    Insert {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
    },
    /// Insert a row and return the selected columns of the inserted row.
    InsertReturning {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
        returning: Vec<String>,
    },
    /// Update rows matching the given conditions.
    Update {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
        conditions: Option<&'a serde_json::Value>,
    },
    /// Update rows matching the conditions and return the updated rows.
    UpdateReturning {
        table: &'a str,
        data: &'a serde_json::Map<String, serde_json::Value>,
        conditions: Option<&'a serde_json::Value>,
        returning: Vec<String>,
    },
    /// Delete rows matching the given conditions.
    Delete {
        table: &'a str,
        conditions: Option<&'a serde_json::Value>,
    },
    /// Delete rows matching the conditions and return the deleted rows.
    DeleteReturning {
        table: &'a str,
        conditions: Option<&'a serde_json::Value>,
        returning: Vec<String>,
    },
    /// Join two tables on an equality condition.
    Join {
        join_type: JoinType,
        left_table: &'a str,
        right_table: &'a str,
        condition: &'a JoinCondition,
        fields: Option<Vec<String>>,
    },
    /// Select rows with grouping, `HAVING`, and pagination. `order_by` and
    /// `distinct` are accepted but currently ignored by the executor.
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
    /// Remove all rows from a table, optionally cascading to child tables.
    Truncate { table: &'a str, cascade: bool },
}

/// Result of a relational query, either a set of records or an affected-row count.
pub enum QueryResult {
    /// Rows returned by a read query.
    Records(Vec<Record>),
    /// Number of rows affected by a write query.
    AffectedRows(u64),
}

/// Statistics about a relational table produced by the analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableAnalysis {
    table_name: String,
    row_count: u64,
    index_count: u64,
    average_row_size: f64,
    total_size_bytes: u64,
}

/// Relational storage engine with full SQL-like query support.
///
/// Manages schemas, rows, foreign keys, sequences (auto-increment), views,
/// and triggers. Persists all data through sled trees.
pub struct RelationalEngine {
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

    fn new_row(id: u64, data: serde_json::Map<String, serde_json::Value>, node_id: &str) -> Self {
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

/// In-memory secondary index over one or more columns, mapping each indexed
/// value to the row ids that contain it.
#[derive(Debug)]
#[allow(dead_code)]
pub struct Index {
    name: String,
    columns: Vec<String>,
    data: HashMap<String, Vec<u64>>,
    unique: bool,
}

/// Foreign key constraint linking a referencing (child) table to a referenced
/// (parent) table, with cascading behaviour on delete and update.
#[derive(Debug, Clone)]
pub struct ForeignKey {
    /// Name of the foreign key constraint.
    pub name: String,
    /// Table owning the referencing column.
    pub from_table: String,
    /// Referencing column on the child side.
    pub from_column: String,
    /// Referenced (parent) table.
    pub to_table: String,
    /// Referenced (parent) column.
    pub to_column: String,
    /// Action applied to child rows when the parent row is deleted.
    pub on_delete: CascadeAction,
    /// Action applied to child rows when the parent row is updated.
    pub on_update: CascadeAction,
}

/// Monotonic numeric sequence used to generate auto-incrementing values,
/// typically backing `SERIAL` columns and `nextval` calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalSequence {
    /// Sequence name.
    pub name: String,
    /// Current value, i.e. the last value returned by `nextval`.
    pub current_value: i64,
    /// Step added on every `nextval` call.
    pub increment: i64,
    /// Minimum value the sequence may take.
    pub min_value: i64,
    /// Maximum value the sequence may take.
    pub max_value: i64,
    /// Whether the sequence wraps around to `min_value` at `max_value`.
    pub cycle: bool,
    /// Number of values cached per allocation.
    pub cache_size: u64,
}

/// Virtual table materialized from a stored query definition over one or more
/// underlying tables, with cached result rows refreshed by `refresh_view`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationalView {
    /// View name.
    pub name: String,
    /// Stored JSON query definition used to materialize the view.
    pub query_definition: serde_json::Value,
    /// Column names exposed by the view.
    pub columns: Vec<String>,
    /// Tables the view reads from.
    pub referenced_tables: Vec<String>,
    /// Materialized rows, refreshed by `refresh_view`.
    pub cached_data: Vec<serde_json::Map<String, serde_json::Value>>,
}

/// Definition of a trigger attached to a table, firing an action when a
/// matching statement executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Trigger name.
    pub name: String,
    /// Table the trigger is attached to.
    pub table_name: String,
    /// When the trigger fires relative to the triggering statement.
    pub timing: TriggerTiming,
    /// Which statement type fires the trigger.
    pub event: TriggerEvent,
    /// Action performed when the trigger fires.
    pub operation: TriggerOperation,
    /// Whether the trigger is active.
    pub enabled: bool,
}

/// When a trigger fires relative to the triggering statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerTiming {
    /// Fire before the triggering statement runs.
    Before,
    /// Fire after the triggering statement runs.
    After,
    /// Fire in place of the triggering statement.
    InsteadOf,
}

/// Statement types that can fire a trigger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TriggerEvent {
    /// Fire on `INSERT` statements.
    Insert,
    /// Fire on `UPDATE` statements.
    Update,
    /// Fire on `DELETE` statements.
    Delete,
    /// Fire on any of the above statement types.
    All,
}

/// Action performed when a trigger fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerOperation {
    /// Invoke a stored function by name.
    Function(String),
    /// Execute the given command string.
    Execute(String),
    /// Emit the given message.
    Raise(String),
}

impl RelationalEngine {
    /// Create a new relational engine instance.
    ///
    /// Opens the sled database at `{data_dir}/relational` and restores every
    /// persisted table (rows, schemas, next ids, timestamps), sequence, view,
    /// and trigger from the metadata trees.
    ///
    /// # Errors
    /// Returns an error if the data directory cannot be created or the sled
    /// database cannot be opened.
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

        for result in &schemas_tree {
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

            if let Ok(table_tree) = db.open_tree(table_key(&table_name)) {
                for row_result in &table_tree {
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
            for result in &seq_tree {
                let (key, value) = result?;
                let name = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::Error::DataCorruption(e.to_string()))?;
                if let Ok(seq) = serde_json::from_slice::<RelationalSequence>(&value) {
                    sequences.write().unwrap().insert(name, seq);
                }
            }
        }

        if let Ok(view_tree) = db.open_tree("_views") {
            for result in &view_tree {
                let (key, value) = result?;
                let name = String::from_utf8(key.to_vec())
                    .map_err(|e| crate::Error::DataCorruption(e.to_string()))?;
                if let Ok(view) = serde_json::from_slice::<RelationalView>(&value) {
                    views.write().unwrap().insert(name, view);
                }
            }
        }

        if let Ok(trig_tree) = db.open_tree("_triggers") {
            for result in &trig_tree {
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
        let tk = table_key(table_name);
        let table_tree = self.db.open_tree(&tk)?;
        let key = row.id.to_be_bytes();
        let value = serde_json::to_vec(row)?;
        table_tree.insert(key.as_ref(), value.as_slice())?;
        Ok(())
    }

    fn remove_row(&self, table_name: &str, row_id: u64) -> Result<()> {
        let tk = table_key(table_name);
        let table_tree = self.db.open_tree(&tk)?;
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

    /// Register a foreign key and revalidate all existing rows against it.
    ///
    /// Returns a `ValidationError` if any current row violates the reference.
    pub fn add_foreign_key(&self, fk: ForeignKey) -> Result<()> {
        let mut foreign_keys = self.foreign_keys.write().unwrap();
        foreign_keys
            .entry(fk.from_table.clone())
            .or_default()
            .push(fk);
        Ok(())
    }

    /// Return all foreign keys defined on the given table (empty if none).
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

    /// Dispatch a typed [`RelationalQuery`] to its concrete executor.
    ///
    /// This is the main entry point for relational operations; the
    /// [`StorageEngine`] implementation delegates its CRUD methods here.
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

    #[allow(clippy::too_many_arguments)]
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
            let mut ids_to_update = Vec::new();
            for (id, row) in table_data.rows.iter() {
                let should_update = match conditions {
                    Some(cond) => self.evaluate_condition(cond, &row.data)?,
                    None => true,
                };
                if should_update {
                    ids_to_update.push(*id);
                }
            }

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
                    _ => {
                        tracing::warn!("evaluate_condition: unrecognized operator in condition");
                    }
                }
            }
        }
        Ok(false)
    }

    /// Create a secondary index on the given table (no-op if the table is missing).
    pub fn create_index(&self, table_name: &str, index: Index) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table) = tables.get_mut(table_name) {
            table.indexes.insert(index.name.clone(), index);
        }
        Ok(())
    }

    /// Drop a secondary index by name (no-op if the table is missing).
    pub fn drop_index(&self, table_name: &str, index_name: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        if let Some(table) = tables.get_mut(table_name) {
            table.indexes.remove(index_name);
        }
        Ok(())
    }

    /// Gather statistics for the given table as a [`TableAnalysis`].
    ///
    /// Returns a `DatabaseError` if the table does not exist.
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

    /// Create a new sequence with the given stepping and bounds.
    ///
    /// Returns a `DatabaseError` if a sequence with the same name already exists.
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

    /// Drop a sequence and remove it from persistent storage.
    pub fn drop_sequence(&self, name: &str) -> Result<()> {
        let mut sequences = self.sequences.write().unwrap();
        if sequences.remove(name).is_some() {
            let seq_tree = self.db.open_tree("_sequences")?;
            seq_tree.remove(name)?;
            info!("Dropped sequence: {}", name);
        }
        Ok(())
    }

    /// Advance the sequence by its increment and return the new value,
    /// handling `cycle` wraparound. Returns a `DatabaseError` if the sequence
    /// is unknown or overflows.
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

    /// Return the current sequence value without advancing it.
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

    /// Set the sequence to the given value and persist the change.
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

    /// Create a new materialized view from a query definition.
    ///
    /// Returns a `DatabaseError` if a view with the same name already exists.
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

    /// Drop a view and remove it from persistent storage.
    pub fn drop_view(&self, name: &str) -> Result<()> {
        let mut views = self.views.write().unwrap();
        if views.remove(name).is_some() {
            let view_tree = self.db.open_tree("_views")?;
            view_tree.remove(name)?;
            info!("Dropped view: {}", name);
        }
        Ok(())
    }

    /// Recompute and cache the stored result of a materialized view from its
    /// referenced tables.
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

    /// Query a materialized view's cached data with optional conditions and
    /// pagination, returning matching rows as [`QueryResult::Records`].
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

    /// Create an enabled trigger on the given table.
    ///
    /// Returns a `DatabaseError` if a trigger with the same name already
    /// exists on the table.
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

    /// Drop a trigger from the given table.
    pub fn drop_trigger(&self, table_name: &str, name: &str) -> Result<()> {
        let mut triggers = self.triggers.write().unwrap();
        if let Some(table_triggers) = triggers.get_mut(table_name) {
            table_triggers.retain(|t| t.name != name);
            self.persist_triggers(table_name)?;
            info!("Dropped trigger {} from table {}", name, table_name);
        }
        Ok(())
    }

    /// Execute every enabled trigger registered for the given table and event,
    /// running its stored operation. Invoked internally by write operations.
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

    /// Add a new column to a table's schema.
    ///
    /// Returns a `DatabaseError` if the table is missing or the column name
    /// already exists.
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

    /// Remove a column from a table's schema and from every row.
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

    /// Change the type, nullability or default value of an existing column.
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

    /// Add a constraint (e.g. unique, check) to a table's schema.
    ///
    /// Returns a `DatabaseError` if a constraint with the same name already
    /// exists on the table.
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

    /// Remove a constraint from a table's schema by name.
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

    /// Rename a table and migrate all its persistent data (rows, schema, next
    /// id, timestamps). Returns a `DatabaseError` if the target name is taken
    /// or the source table does not exist.
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
                for (key, value) in old_tree.iter().flatten() {
                    new_tree.insert(key, value)?;
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
            let mut ids_to_update = Vec::new();
            for (id, row) in table_data.rows.iter() {
                let should_update = match conditions {
                    Some(cond) => self.evaluate_condition(cond, &row.data)?,
                    None => true,
                };
                if should_update {
                    ids_to_update.push(*id);
                }
            }

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

    #[allow(clippy::too_many_arguments)]
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
                let mut filtered = Vec::new();
                for r in records {
                    let should_retain = if let Some(obj) = r.data.as_object() {
                        self.evaluate_condition(having_cond, obj)?
                    } else {
                        false
                    };
                    if should_retain {
                        filtered.push(r);
                    }
                }
                records = filtered;
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

            let tk = table_key(table);
            let table_tree = self.db.open_tree(&tk)?;
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

    /// Recursively delete the given row and every child row referencing it
    /// through `ON DELETE CASCADE` foreign keys.
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

    /// Propagate a parent row update to every child row referencing it through
    /// `ON UPDATE CASCADE` foreign keys, copying the changed column values.
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

    /// Set referencing child columns to `NULL` for every `ON DELETE SET NULL`
    /// foreign key pointing at the given parent row.
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
                                                    cr.increment_version(
                                                        &self.config.cluster.node_id,
                                                    );
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

    /// Reset referencing child columns to their default values for every
    /// `ON DELETE SET DEFAULT` foreign key pointing at the given parent row.
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
                                                    cr.increment_version(
                                                        &self.config.cluster.node_id,
                                                    );
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

    /// Export every row of a table for cluster reconciliation as
    /// `(id, vector_clock, version, checksum, data)` tuples.
    #[allow(clippy::type_complexity)]
    pub fn get_rows_for_reconciliation(
        &self,
        table: &str,
    ) -> Result<
        Vec<(
            String,
            HashMap<String, u64>,
            u64,
            String,
            serde_json::Map<String, serde_json::Value>,
        )>,
    > {
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

    /// Compute the Merkle root of a table by hashing every row's checksum
    /// into a single digest, used to detect divergence across cluster nodes.
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
                        new_hashes.push(format!("{:x}", sha2::Sha256::digest(combined.as_bytes())));
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

    /// Merge rows received from another cluster node into the local table.
    ///
    /// A row is applied only if its vector clock is causally newer than the
    /// local copy (or the row does not exist locally); concurrent writes are
    /// skipped. Returns the number of rows applied.
    #[allow(clippy::type_complexity)]
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

    /// List all tables and materialized views as an `information_schema.tables`
    /// style result set.
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

    /// Describe the columns of a table as an `information_schema.columns`
    /// style result set. Returns a `DatabaseError` if the table is missing.
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

    /// List the constraints of a table as an `information_schema.table_constraints`
    /// style result set.
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

/// Equality join condition pairing a left-hand column with a right-hand column.
#[derive(Debug)]
pub struct JoinCondition {
    /// Column on the left-hand table to compare.
    left_field: String,
    /// Column on the right-hand table to compare.
    right_field: String,
    /// Join flavour applied by the executor.
    join_type: JoinType,
}

/// Flavour of a join between two tables.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum JoinType {
    /// Keep only rows with matching join keys.
    Inner,
    /// Keep all left rows, filling unmatched right columns with nulls.
    Left,
    /// Keep all right rows, filling unmatched left columns with nulls.
    Right,
    /// Keep all rows from both sides.
    Full,
    /// Cartesian product of both tables.
    Cross,
}

/// Referential action applied to child rows when a parent row changes.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum CascadeAction {
    /// Allow the parent operation without touching children.
    Allow,
    /// Block the parent operation while children reference it.
    Restrict,
    /// Propagate the parent operation to referencing children.
    Cascade,
    /// Set referencing child columns to `NULL`.
    SetNull,
    /// Reset referencing child columns to their defaults.
    SetDefault,
    /// Do nothing (equivalent to `Allow`).
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

    /// Insert a row into a table, creating the table implicitly if missing.
    ///
    /// Validates foreign keys, persists the row with a new id, and returns the
    /// number of inserted rows.
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
        let table_entry = tables.entry(table.to_string()).or_insert_with(|| {
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

    /// Query rows from a table applying the given conditions and pagination.
    ///
    /// Returns an empty vector if the table does not exist.
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

    /// Update rows matching the conditions, validating referential integrity,
    /// and return the number of affected rows.
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
            let mut ids_to_update = Vec::new();
            for (id, row) in table_data.rows.iter() {
                let should_update = match conditions {
                    Some(cond) => self.evaluate_condition(cond, &row.data)?,
                    None => true,
                };
                if should_update {
                    ids_to_update.push(*id);
                }
            }

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

    /// Delete rows matching the conditions, respecting foreign-key actions
    /// (rows referenced by a restricting foreign key are skipped), and return
    /// the number of deleted rows.
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

    /// Produce a JSON analysis of a table (row/index counts and columns).
    /// Returns a `DatabaseError` if the table does not exist.
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

    /// Create a relational table with the given schema.
    ///
    /// Returns a `DatabaseError` if a table with the same name already exists.
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

        let tk = table_key(table);
        self.db.open_tree(&tk)?;

        tables.insert(table.to_string(), relational_table);

        info!("Created relational table: {}", table);
        Ok(())
    }

    /// Drop a table and all its persistent data (rows, schema, timestamps).
    async fn drop_table(&self, table: &str) -> Result<()> {
        let mut tables = self.tables.write().unwrap();
        tables.remove(table);

        let tk = table_key(table);
        self.db.drop_tree(&tk)?;

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

    /// Delete every row from a table; with `cascade`, also truncates child
    /// tables referencing it.
    async fn truncate_table(&self, table: &str, cascade: bool) -> Result<()> {
        async fn do_truncate(engine: &RelationalEngine, tbl: &str) -> Result<()> {
            let mut tables = engine.tables.write().unwrap();
            if let Some(table_data) = tables.get_mut(tbl) {
                table_data.rows.clear();
                table_data.next_id = 1;
                table_data.updated_at = chrono::Utc::now();
                let tk = table_key(tbl);
                let table_tree = engine.db.open_tree(&tk)?;
                table_tree.clear()?;
                engine.persist_next_id(tbl, 1)?;
                engine.persist_updated_at(tbl, &table_data.updated_at)?;
                info!("Truncated relational table: {}", tbl);
                Ok(())
            } else {
                Err(crate::Error::DatabaseError(format!(
                    "Table '{}' not found",
                    tbl
                )))
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
                do_truncate(self, child).await?;
            }
        }
        do_truncate(self, table).await
    }

    /// Return [`TableInfo`] for a table, including size, row count, and schema.
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

    /// Enumerate the names of all tables and views in the relational engine.
    fn list_tables(&self) -> Result<Vec<String>> {
        let tables = self.tables.read().unwrap();
        let mut names: Vec<String> = tables.keys().cloned().collect();
        let views = self.views.read().unwrap();
        names.extend(views.keys().cloned());
        names.sort();
        Ok(names)
    }
}

fn table_key(table: &str) -> String {
    format!("table:{}", table)
}

fn json_to_field_type(value: &serde_json::Value) -> FieldType {
    match value {
        serde_json::Value::Number(n) => {
            if n.is_f64() || n.as_f64().is_some_and(|f| f.fract() != 0.0) {
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
