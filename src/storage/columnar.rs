/*
 * PrimusDB Columnar Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: as_any() method for engine-specific features
 */

/*!
# Columnar Storage Engine - Analytics-Optimized Database

The columnar engine provides an analytics-oriented StorageEngine implementation
over an embedded sled database. Records are stored as JSON blobs in a sled tree
per table (keys are nanosecond-timestamp ids), and the engine offers CRUD,
per-table analysis, and table metadata introspection. A true columnar on-disk
layout (per-column files, LZ4/bitmap compression, SIMD vectorization) is a
planned enhancement; the current implementation stores whole rows and keeps
the engine usable for analytical workloads via the query pipeline.

```text
Columnar Engine Data Flow
═══════════════════════════════════════════════════

insert / update / delete ──► ColumnarEngine (sled db "columnar")
        │                       └─► tree "table:{name}" (JSON blobs)
        ▼
select ──► conditions match + offset/limit pagination over tree scan
        │
        ▼
analyze ──► record count + per-field occurrence counts + inferred types
```

## Main Types & Functions

- [`ColumnarEngine`]: the analytics-oriented engine implementing [`StorageEngine`].
- `insert`: store a record under an auto-generated nanosecond-timestamp id.
- `select`: filtered, paginated scans over the table tree.
- `analyze`: per-table record counts, field occurrence counts, and inferred field types.
- `create_table` / `drop_table` / `truncate_table` / `table_info`: table lifecycle.

## Limitations

- Rows are stored as whole JSON blobs; no per-column compression or bitmap
  indexes exist yet, and queries always scan the full table.
*/

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;

use sled::Db;
use std::any::Any;
use std::collections::HashMap;

fn table_key(table: &str) -> String {
    format!("table:{}", table)
}

fn schema_key(table: &str) -> String {
    format!("schema:{}", table)
}

/// Load the persisted schema for a table, if one was captured at creation.
fn load_schema(db: &Db, table: &str) -> crate::Result<Option<Schema>> {
    let schemas = db.open_tree("schemas")?;
    match schemas.get(schema_key(table))? {
        Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        None => Ok(None),
    }
}

/// Persist a table's schema so inserts can be validated against it.
fn save_schema(db: &Db, table: &str, schema: &Schema) -> crate::Result<()> {
    let schemas = db.open_tree("schemas")?;
    schemas.insert(schema_key(table), serde_json::to_vec(schema)?)?;
    schemas.flush()?;
    Ok(())
}

/// Remove a table's persisted schema.
fn clear_schema(db: &Db, table: &str) -> crate::Result<()> {
    let schemas = db.open_tree("schemas")?;
    schemas.remove(schema_key(table))?;
    Ok(())
}

/// Validate a record against a persisted schema, rejecting clearly
/// incompatible column types while allowing unknown extra fields and nulls.
fn validate_against_schema(schema: &Schema, data: &serde_json::Value) -> crate::Result<()> {
    use crate::storage::validation::field_type_accepts;
    if let Some(obj) = data.as_object() {
        for field in &schema.fields {
            if let Some(value) = obj.get(&field.name) {
                if !field_type_accepts(&field.field_type, value) {
                    return Err(crate::Error::ValidationError(format!(
                        "column '{}' got incompatible value {}",
                        field.name, value
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Equality matching over a stored record. The special `id` key compares
/// against the record's storage id as a string.
fn matches_conditions(data: &serde_json::Value, conditions: &serde_json::Value, id: &str) -> bool {
    if conditions.is_null() || conditions.as_object().is_none_or(|o| o.is_empty()) {
        return true;
    }
    if let Some(obj) = conditions.as_object() {
        for (key, cond_val) in obj {
            if key == "id" {
                if id != cond_val.as_str().unwrap_or("") {
                    return false;
                }
                continue;
            }
            match data.get(key) {
                Some(data_val) => {
                    if data_val != cond_val {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    } else {
        false
    }
}

/// Columnar storage engine implementation
///
/// Analytics-oriented engine backed by a sled database. Each table is a sled
/// tree of JSON blobs keyed by nanosecond-timestamp ids. The engine implements
/// the [`StorageEngine`] CRUD surface, filtered paginated scans, and per-table
/// analysis; a true columnar layout with per-column compression and bitmap
/// indexes is planned but not yet implemented.
///
/// # Key Features
/// - Sled-backed table trees with atomic insert/update/delete
/// - Condition filtering over stored JSON with offset/limit pagination
/// - `analyze` reporting record counts and inferred field types
/// - Thread-safe concurrent reads and writes
///
/// # Performance Characteristics
/// - Queries scan the whole table tree (no secondary indexes)
/// - Writes flush eagerly for durability
/// - Suitable for analytical workloads on datasets where full scans are viable
pub struct ColumnarEngine {
    db: Db,
}

impl ColumnarEngine {
    /// Create a new columnar storage engine instance
    ///
    /// Initializes the columnar engine with the provided configuration.
    /// Creates the necessary directory structure and opens the embedded database.
    ///
    /// # Arguments
    /// * `config` - PrimusDB configuration containing storage settings
    ///
    /// # Returns
    /// A new ColumnarEngine instance ready for operations
    ///
    /// # Errors
    /// Returns an error if:
    /// - Database directory cannot be created
    /// - Sled database cannot be opened
    /// - Configuration is invalid
    ///
    /// # Example
    /// ```ignore
    /// let config = PrimusDBConfig::default();
    /// let engine = ColumnarEngine::new(&config)?;
    /// ```
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let db_path = format!("{}/columnar", config.storage.data_dir);
        let db = sled::open(&db_path)?;

        Ok(ColumnarEngine { db })
    }
}

#[async_trait]
impl StorageEngine for ColumnarEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Insert a new record into the columnar storage
    ///
    /// Serializes the JSON record to binary and stores it in the table's sled
    /// tree under an auto-generated nanosecond-timestamp id.
    ///
    /// # Arguments
    /// * `table` - Target table name
    /// * `data` - JSON data to insert (object with field-value pairs)
    /// * `_transaction` - Transaction context (currently unused in columnar engine)
    ///
    /// # Returns
    /// Unique record ID (timestamp-based nanosecond precision)
    ///
    /// # Implementation Details
    /// - Uses tokio::task::spawn_blocking for CPU-intensive operations
    /// - Generates unique IDs using system timestamp
    /// - Serializes data to binary format for storage
    /// - Flushes data immediately for consistency
    ///
    /// # Performance Notes
    /// - Single inserts flush eagerly for durability
    /// - ID generation is monotonic but not guaranteed to be gap-free
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        if !data.is_object() {
            return Err(crate::Error::ValidationError(
                "columnar records must be JSON objects".to_string(),
            ));
        }
        let value = serde_json::to_vec(data)?;
        let table_owned = table.to_string();
        let data = data.clone();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<u64> {
                if let Some(schema) = load_schema(&db, &table_owned)? {
                    validate_against_schema(&schema, &data)?;
                }
                let tree = db.open_tree(table_key)?;
                let id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;
                let key = id.to_be_bytes();
                tree.insert(key, value)?;
                tree.flush()?;
                Ok(id)
            }
        })
        .await??;

        Ok(result)
    }

    /// Query records from columnar storage with pagination
    ///
    /// Iterates the table's sled tree in key (timestamp) order, skipping
    /// `offset` records and returning up to `limit` records. Each stored JSON
    /// blob is deserialized and the record id is recovered from the 8-byte
    /// big-endian timestamp key.
    ///
    /// # Arguments
    /// * `table` - Target table name
    /// * `_conditions` - Optional JSON filter conditions (currently ignored;
    ///   the engine has no predicate pushdown yet)
    /// * `limit` - Maximum number of records to return
    /// * `offset` - Number of records to skip for pagination
    /// * `_transaction` - Transaction context for consistency
    ///
    /// # Returns
    /// Vector of matching records
    ///
    /// # Limitations
    /// - Performs a full tree scan; conditions are not applied (the caller
    ///   must filter in the query pipeline)
    /// - Records are read as whole JSON blobs; no per-column pruning
    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);
        let table_owned = table.to_string();
        let result: Vec<Record> = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<Vec<Record>> {
                let tree = db.open_tree(table_key)?;
                let mut records = Vec::new();
                let limit = if limit == 0 { u64::MAX } else { limit };

                for (i, item) in tree.iter().enumerate() {
                    let (key, value) = item?;
                    let ts_bytes: [u8; 8] = key.as_ref().try_into().map_err(|_| {
                        crate::Error::DataCorruption("columnar key is not 8 bytes".into())
                    })?;
                    let id = u64::from_be_bytes(ts_bytes);
                    let data: serde_json::Value = serde_json::from_slice(&value)?;

                    if !matches_conditions(&data, &conditions, &id.to_string()) {
                        continue;
                    }
                    if records.len() >= limit as usize {
                        break;
                    }
                    if i < offset as usize {
                        continue;
                    }

                    records.push(Record {
                        id: id.to_string(),
                        data,
                        metadata: HashMap::new(),
                    });
                }

                Ok(records)
            }
        })
        .await??;

        Ok(result)
    }

    /// Update a single record identified by an `id` field in `data` or in
    /// `conditions`.
    ///
    /// The record is replaced wholesale with the new value and flushed to disk.
    /// Fails with a validation error when no `id` can be resolved (a silent
    /// no-op would mask a misdirected query).
    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        if !data.is_object() {
            return Err(crate::Error::ValidationError(
                "columnar update payload must be a JSON object".to_string(),
            ));
        }
        let id_str = data
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                conditions
                    .and_then(|c| c.get("id"))
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| {
                crate::Error::ValidationError("columnar update requires an 'id' field".to_string())
            })?
            .to_string();

        let value = serde_json::to_vec(data)?;
        let id = id_str.parse::<u64>()?;
        let table_owned = table.to_string();
        let data = data.clone();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<u64> {
                if let Some(schema) = load_schema(&db, &table_owned)? {
                    validate_against_schema(&schema, &data)?;
                }
                let tree = db.open_tree(table_key)?;
                let key = id.to_be_bytes();
                if !tree.contains_key(key)? {
                    return Err(crate::Error::ValidationError(format!(
                        "columnar record {} not found in {}",
                        id, table_owned
                    )));
                }
                tree.insert(key, value)?;
                tree.flush()?;
                Ok(1)
            }
        })
        .await??;
        Ok(result)
    }

    /// Delete a single record matching the `id` condition.
    ///
    /// Returns `1` on success. Fails with a validation error when no `id` is
    /// supplied so a misdirected delete can never silently delete nothing.
    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let id_str = conditions
            .and_then(|c| c.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::Error::ValidationError(
                    "columnar delete requires an 'id' condition".to_string(),
                )
            })?;
        let id = id_str.parse::<u64>()?;
        let table_owned = table.to_string();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<u64> {
                let tree = db.open_tree(table_key)?;
                let key = id.to_be_bytes();
                let existed = tree.remove(key)?;
                tree.flush()?;
                Ok(if existed.is_some() { 1 } else { 0 })
            }
        })
        .await??;
        Ok(result)
    }

    /// Produce a JSON analysis of a table: total record count, per-field
    /// occurrence counts, and inferred field types.
    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        let table_owned = table.to_string();
        let result: serde_json::Value = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<serde_json::Value> {
                let tree = db.open_tree(table_key)?;
                let mut total_records = 0u64;
                let mut field_counts: HashMap<String, u64> = HashMap::new();
                let mut field_types: HashMap<String, String> = HashMap::new();

                for item in &tree {
                    let (_, value) = item?;
                    total_records += 1;
                    let data: serde_json::Value = serde_json::from_slice(&value)?;
                    if let Some(obj) = data.as_object() {
                        for (key, val) in obj {
                            *field_counts.entry(key.clone()).or_insert(0) += 1;
                            if !field_types.contains_key(key) {
                                let type_str = match val {
                                    serde_json::Value::Null => "null",
                                    serde_json::Value::Bool(_) => "boolean",
                                    serde_json::Value::Number(_) => "number",
                                    serde_json::Value::String(_) => "string",
                                    serde_json::Value::Array(_) => "array",
                                    serde_json::Value::Object(_) => "object",
                                };
                                field_types.insert(key.clone(), type_str.to_string());
                            }
                        }
                    }
                }

                Ok(serde_json::json!({
                    "table": table_owned,
                    "total_records": total_records,
                    "fields": field_counts,
                    "field_types": field_types,
                    "engine": "columnar"
                }))
            }
        })
        .await??;

        Ok(serde_json::to_string(&result)?)
    }

    /// Create a table by opening its backing sled tree and persisting the
    /// schema for insert-time type validation.
    async fn create_table(&self, table: &str, schema: &Schema) -> Result<()> {
        let table_owned = table.to_string();
        let schema = schema.clone();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<()> {
                db.open_tree(table_key)?;
                save_schema(&db, &table_owned, &schema)?;
                Ok(())
            }
        })
        .await??;
        Ok(())
    }

    /// Drop a table by deleting its backing sled tree.
    async fn drop_table(&self, table: &str) -> Result<()> {
        let table_owned = table.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<()> {
                db.drop_tree(table_key)?;
                clear_schema(&db, &table_owned)?;
                Ok(())
            }
        })
        .await??;
        Ok(())
    }

    /// Delete every record from a table by removing all keys in its tree.
    async fn truncate_table(&self, table: &str, _cascade: bool) -> Result<()> {
        let table_owned = table.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<()> {
                let tree = db.open_tree(table_key)?;
                let mut iter = tree.iter();
                while let Some(Ok((key, _))) = iter.next() {
                    tree.remove(key)?;
                }
                tree.flush()?;
                Ok(())
            }
        })
        .await??;
        Ok(())
    }

    /// Return [`TableInfo`] for a table: row count, field names (sampled from
    /// the first 20 records), and an inferred `Text`-typed schema.
    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let (count, size, fields): (usize, u64, Vec<String>) = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(table);
            move || -> crate::Result<(usize, u64, Vec<String>)> {
                let tree = db.open_tree(table_key)?;
                let count = tree.len();
                let mut field_names: Vec<String> = Vec::new();
                for (_key, value) in tree.iter().take(20).flatten() {
                    if let serde_json::Value::Object(map) =
                        &serde_json::from_slice::<serde_json::Value>(&value).unwrap_or_default()
                    {
                        for key in map.keys() {
                            if !field_names.contains(key) {
                                field_names.push(key.clone());
                            }
                        }
                    }
                }
                Ok((count, 0, field_names))
            }
        })
        .await??;

        Ok(TableInfo {
            name: table.to_string(),
            schema: Schema {
                fields: fields
                    .into_iter()
                    .map(|name| crate::storage::Field {
                        name,
                        field_type: crate::storage::FieldType::Text,
                        nullable: true,
                        default_value: None,
                        constraints: vec![],
                    })
                    .collect(),
                indexes: vec![],
                constraints: vec![],
            },
            row_count: count as u64,
            size_bytes: size,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Enumerate the names of all columnar tables from their sled trees.
    fn list_tables(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> = self
            .db
            .tree_names()
            .into_iter()
            .filter_map(|name| {
                let name = String::from_utf8(name.to_vec()).ok()?;
                name.strip_prefix("table:").map(|t| t.to_string())
            })
            .collect();
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Field, FieldType};
    use crate::transaction::{IsolationLevel, Transaction, TransactionStatus};

    fn config(dir: &tempfile::TempDir) -> PrimusDBConfig {
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_string_lossy().into_owned();
        config
    }

    fn tx() -> Transaction {
        Transaction {
            id: "test".to_string(),
            operations: vec![],
            status: TransactionStatus::Prepared,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            isolation_level: IsolationLevel::ReadCommitted,
            timeout_ms: 0,
        }
    }

    #[tokio::test]
    async fn test_insert_rejects_non_object() {
        let dir = tempfile::tempdir().unwrap();
        let engine = ColumnarEngine::new(&config(&dir)).unwrap();

        let err = engine
            .insert("t", &serde_json::json!([1, 2, 3]), &tx())
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_schema_type_validation_on_insert() {
        let dir = tempfile::tempdir().unwrap();
        let engine = ColumnarEngine::new(&config(&dir)).unwrap();

        let schema = Schema {
            fields: vec![Field {
                name: "amount".to_string(),
                field_type: FieldType::Integer,
                nullable: true,
                default_value: None,
                constraints: vec![],
            }],
            indexes: vec![],
            constraints: vec![],
        };
        engine.create_table("typed", &schema).await.unwrap();

        let err = engine
            .insert(
                "typed",
                &serde_json::json!({"amount": "not-a-number"}),
                &tx(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));

        engine
            .insert("typed", &serde_json::json!({"amount": 42}), &tx())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_select_filters_by_conditions() {
        let dir = tempfile::tempdir().unwrap();
        let engine = ColumnarEngine::new(&config(&dir)).unwrap();

        engine
            .insert("sales", &serde_json::json!({"product_id": 1}), &tx())
            .await
            .unwrap();
        engine
            .insert("sales", &serde_json::json!({"product_id": 2}), &tx())
            .await
            .unwrap();

        let records = engine
            .select(
                "sales",
                Some(&serde_json::json!({"product_id": 2})),
                10,
                0,
                &tx(),
            )
            .await
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].data["product_id"], 2);
    }

    #[tokio::test]
    async fn test_update_and_delete_require_id() {
        let dir = tempfile::tempdir().unwrap();
        let engine = ColumnarEngine::new(&config(&dir)).unwrap();

        let err = engine
            .update("sales", None, &serde_json::json!({"product_id": 1}), &tx())
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));

        let err = engine.delete("sales", None, &tx()).await.unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));
    }
}
