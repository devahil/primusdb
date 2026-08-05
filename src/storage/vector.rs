/*
 * PrimusDB Vector Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: as_any() method for engine-specific features
 */

/*!
# Vector Storage Engine - Similarity Search Database

The vector storage engine stores JSON records in sled-backed collections and
answers cosine-similarity queries. A record's `vector` field (an array of
numbers) is compared against a query vector with a brute-force cosine
similarity scan, and the top-K matches are returned ranked by descending
similarity (exposed in each record's `similarity` metadata). Use it for ML
embeddings, semantic search, and feature-vector storage where an exhaustive
scan over the collection is acceptable.

```text
Vector Engine Data Flow
═══════════════════════════════════════════════════

insert / update / delete ──► VectorEngine (sled db "vector")
        │                       └─► tree "table:{collection}" (JSON records)
        ▼
select with { "query_vector": [...] } ──► brute-force cosine scan
        │                                   └─► top-K ranked by similarity
        └─► plain paginated scan (offset/limit) when no query_vector
```

## Main Types & Functions

- [`VectorEngine`]: the vector similarity engine implementing [`StorageEngine`].
- `insert`: store a record under an auto-generated nanosecond-timestamp id.
- `select`: cosine-similarity search when `query_vector` is supplied, otherwise
  a paginated scan of the collection.
- `analyze`: per-collection record count, field occurrence counts, and inferred
  field types.
- `create_table` / `drop_table` / `truncate_table` / `table_info`: collection
  lifecycle and metadata inspection.

## Limitations

- Search is an exhaustive linear scan; no approximate (IVF/HNSW) indexes or
  quantized layouts are built yet, so latency scales linearly with collection
  size.
*/

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use tracing::info;

use sled::Db;
use std::any::Any;
use std::collections::HashMap;

fn table_key(table: &str) -> String {
    format!("table:{}", table)
}

fn dim_key(table: &str) -> String {
    format!("dim:{}", table)
}

/// Read the established embedding dimension for a collection, if any.
///
/// Dimensions are tracked per collection in a shared `dims` sled tree so that
/// inserts and queries are validated against a consistent shape instead of
/// failing mid-scan.
fn read_collection_dim(db: &Db, table: &str) -> crate::Result<Option<usize>> {
    let dims = db.open_tree("dims")?;
    match dims.get(dim_key(table))? {
        Some(bytes) => {
            let dim = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| crate::Error::DataCorruption("corrupt dimension metadata".into()))?;
            Ok(Some(dim))
        }
        None => Ok(None),
    }
}

/// Persist the embedding dimension for a collection.
fn write_collection_dim(db: &Db, table: &str, dim: usize) -> crate::Result<()> {
    let dims = db.open_tree("dims")?;
    dims.insert(dim_key(table), dim.to_string().as_bytes())?;
    dims.flush()?;
    Ok(())
}

/// Remove the embedding dimension for a collection.
fn clear_collection_dim(db: &Db, table: &str) -> crate::Result<()> {
    let dims = db.open_tree("dims")?;
    dims.remove(dim_key(table))?;
    Ok(())
}

/// Validate a record's `vector` field against the collection dimension,
/// establishing the dimension on first insert. Records without a `vector`
/// field are allowed (payload records) but are skipped by similarity scans.
fn validate_and_track_dimension(
    db: &Db,
    table: &str,
    data: &serde_json::Value,
) -> crate::Result<()> {
    let Some(vector) = data.get("vector") else {
        return Ok(());
    };
    let dim = crate::storage::validation::vector_dimension(vector)?;
    match read_collection_dim(db, table)? {
        Some(d) if d != dim => {
            return Err(crate::Error::InvalidRequest(format!(
                "vector dimension {} does not match collection dimension {}",
                dim, d
            )));
        }
        Some(_) => {}
        None => write_collection_dim(db, table, dim)?,
    }
    Ok(())
}

/// Vector storage engine for brute-force similarity search
///
/// Stores JSON records in sled-backed collections (one sled tree per
/// collection) and answers cosine-similarity queries. Each record is inserted
/// under a nanosecond-timestamp id; a `vector` field holds the embedding to
/// compare against.
///
/// # Key Features
/// - Brute-force cosine similarity search via `query_vector` conditions
/// - Paginated scans of a collection when no query vector is given
/// - Per-collection analysis (record counts, field types) via `analyze`
/// - Thread-safe concurrent reads and writes through sled trees
///
/// # Use Cases
/// - Semantic search and retrieval
/// - ML model embedding storage
/// - Recommendation features and anomaly detection features
///
/// # Performance Characteristics
/// - Search is O(n) over the collection with no approximate indexes
/// - Suitable for datasets where an exhaustive cosine scan is acceptable
pub struct VectorEngine {
    db: Db,
}

impl VectorEngine {
    /// Create a new vector engine instance.
    ///
    /// Opens the sled database at `{data_dir}/vector`. Collections are created
    /// lazily on first use.
    ///
    /// # Errors
    /// Returns an error if the sled database cannot be opened.
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let db_path = format!("{}/vector", config.storage.data_dir);
        let db = sled::open(&db_path)?;

        Ok(VectorEngine { db })
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> crate::Result<f32> {
        if a.len() != b.len() {
            return Err(crate::Error::InvalidRequest(
                "vector dimensions must match".into(),
            ));
        }
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }
        Ok(dot_product / (norm_a * norm_b))
    }

    fn matches_conditions(data: &serde_json::Value, conditions: &serde_json::Value) -> bool {
        if conditions.is_null() || conditions.as_object().is_none_or(|o| o.is_empty()) {
            return true;
        }
        if let Some(obj) = conditions.as_object() {
            for (key, cond_val) in obj {
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
}

#[async_trait]
impl StorageEngine for VectorEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Insert a new vector record, returning its timestamp-based unique id.
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let value = serde_json::to_vec(data)?;
        let table_owned = table.to_string();
        let data = data.clone();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<u64> {
                validate_and_track_dimension(&db, &table_owned, &data)?;
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

    /// Query records from a vector collection.
    ///
    /// If `conditions` contains a `query_vector` array, performs a cosine
    /// similarity search and returns the top-K matches ranked by descending
    /// similarity (exposed in each record's `similarity` metadata). Otherwise
    /// performs a plain paginated scan of all records.
    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        if let Some(conditions) = conditions {
            if let Some(query_vec_val) = conditions.get("query_vector") {
                let query_vec = crate::storage::validation::parse_finite_vector(query_vec_val)?;
                let limit = if limit == 0 { 10 } else { limit };

                let result: Vec<Record> = tokio::task::spawn_blocking({
                    let db = self.db.clone();
                    let table_key = table_key(table);
                    let table_owned = table.to_string();
                    let query_vec = query_vec.clone();
                    move || -> crate::Result<Vec<Record>> {
                        let tree = db.open_tree(table_key)?;
                        if let Some(d) = read_collection_dim(&db, &table_owned)? {
                            if d != query_vec.len() {
                                return Err(crate::Error::InvalidRequest(format!(
                                "query vector dimension {} does not match collection dimension {}",
                                query_vec.len(),
                                d
                            )));
                            }
                        }
                        let mut similarities = Vec::new();

                        for item in &tree {
                            let (key, value) = item?;
                            let ts_bytes: [u8; 8] = key.as_ref().try_into().map_err(|_| {
                                crate::Error::DataCorruption("vector key is not 8 bytes".into())
                            })?;
                            let id = u64::from_be_bytes(ts_bytes);
                            let data: serde_json::Value = serde_json::from_slice(&value)?;

                            let Some(vec_val) = data.get("vector") else {
                                continue;
                            };
                            let vec = crate::storage::validation::parse_finite_vector(vec_val)?;
                            let similarity = Self::cosine_similarity(&query_vec, &vec)?;
                            similarities.push((id, data, similarity));
                        }

                        similarities.sort_by(|a, b| {
                            b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        let mut records = Vec::new();
                        for (id, data, similarity) in similarities
                            .into_iter()
                            .skip(offset as usize)
                            .take(limit as usize)
                        {
                            let mut metadata = HashMap::new();
                            metadata.insert("similarity".to_string(), similarity.to_string());
                            records.push(Record {
                                id: id.to_string(),
                                data,
                                metadata,
                            });
                        }

                        Ok(records)
                    }
                })
                .await??;

                Ok(result)
            } else {
                // Normal select
                let result: Vec<Record> = tokio::task::spawn_blocking({
                    let db = self.db.clone();
                    let table_key = table_key(table);
                    let limit = if limit == 0 { u64::MAX } else { limit };
                    move || -> crate::Result<Vec<Record>> {
                        let tree = db.open_tree(table_key)?;
                        let mut records = Vec::new();

                        for (i, item) in tree.iter().enumerate() {
                            if i < offset as usize {
                                continue;
                            }
                            if records.len() >= limit as usize {
                                break;
                            }

                            let (key, value) = item?;
                            let ts_bytes: [u8; 8] = key.as_ref().try_into().map_err(|_| {
                                crate::Error::DataCorruption("vector key is not 8 bytes".into())
                            })?;
                            let id = u64::from_be_bytes(ts_bytes);
                            let data: serde_json::Value = serde_json::from_slice(&value)?;

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
        } else {
            // No conditions, return all
            self.select(
                table,
                Some(&serde_json::json!({})),
                limit,
                offset,
                _transaction,
            )
            .await
        }
    }

    /// Update records matching the conditions by merging the new field values,
    /// returning the number of updated records.
    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);
        let data = data.clone();
        let table_owned = table.to_string();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<u64> {
                validate_and_track_dimension(&db, &table_owned, &data)?;
                let tree = db.open_tree(table_key)?;
                let mut updated = 0u64;
                let mut batch = Vec::new();

                for item in &tree {
                    let (key, value) = item?;
                    let stored: serde_json::Value = serde_json::from_slice(&value)?;

                    if Self::matches_conditions(&stored, &conditions) {
                        let merged = if let (Some(stored_obj), Some(data_obj)) =
                            (stored.as_object(), data.as_object())
                        {
                            let mut merged = stored_obj.clone();
                            for (k, v) in data_obj {
                                merged.insert(k.clone(), v.clone());
                            }
                            serde_json::Value::Object(merged)
                        } else {
                            data.clone()
                        };
                        let new_value = serde_json::to_vec(&merged)?;
                        batch.push((key.to_vec(), new_value));
                        updated += 1;
                    }
                }

                for (key, value) in batch {
                    tree.insert(key, value)?;
                }

                if updated > 0 {
                    tree.flush()?;
                }

                info!(
                    "Vector update in {}: {} records updated",
                    table_owned, updated
                );
                Ok(updated)
            }
        })
        .await??;

        Ok(result)
    }

    /// Delete records matching the conditions, returning the number deleted.
    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);
        let table_owned = table.to_string();
        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<u64> {
                let tree = db.open_tree(table_key)?;
                let mut deleted = 0u64;
                let mut to_remove = Vec::new();

                for item in &tree {
                    let (key, value) = item?;
                    let stored: serde_json::Value = serde_json::from_slice(&value)?;

                    if Self::matches_conditions(&stored, &conditions) {
                        to_remove.push(key.to_vec());
                        deleted += 1;
                    }
                }

                for key in &to_remove {
                    tree.remove(key)?;
                }

                if deleted > 0 {
                    tree.flush()?;
                }

                info!(
                    "Vector delete from {}: {} records deleted",
                    table_owned, deleted
                );
                Ok(deleted)
            }
        })
        .await??;

        Ok(result)
    }

    /// Produce a JSON analysis of a vector collection: total records,
    /// per-field occurrence counts, and inferred field types.
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
                    "engine": "vector"
                }))
            }
        })
        .await??;

        info!(
            "Vector analyze for table: {} - {} records",
            table, result["total_records"]
        );
        Ok(serde_json::to_string(&result)?)
    }

    /// Create a vector collection by opening its backing sled tree.
    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(table);
            move || -> crate::Result<()> {
                db.open_tree(table_key)?;
                Ok(())
            }
        })
        .await??;

        info!("Vector collection created: {}", table);
        Ok(())
    }

    /// Drop a vector collection by deleting its backing sled tree.
    async fn drop_table(&self, table: &str) -> Result<()> {
        let table_owned = table.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_key = table_key(&table_owned);
            move || -> crate::Result<()> {
                db.drop_tree(table_key)?;
                clear_collection_dim(&db, &table_owned)?;
                Ok(())
            }
        })
        .await??;

        info!("Vector collection dropped: {}", table);
        Ok(())
    }

    /// Delete every record from a vector collection by clearing its tree.
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
                clear_collection_dim(&db, &table_owned)?;
                Ok(())
            }
        })
        .await??;

        info!("Vector collection truncated: {}", table);
        Ok(())
    }

    /// Return [`TableInfo`] for a vector collection, including row count,
    /// sampled field names, and a schema declaring a cosine-similarity index
    /// on the `vector` column.
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

        info!(
            "Vector collection info retrieved: {} ({} rows)",
            table, count
        );
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
                indexes: vec![crate::storage::Index {
                    name: format!("idx_{}_vector", table),
                    fields: vec!["vector".to_string()],
                    index_type: crate::storage::IndexType::VectorSimilarity {
                        distance: crate::storage::DistanceMetric::Cosine,
                    },
                    unique: false,
                }],
                constraints: vec![],
            },
            row_count: count as u64,
            size_bytes: size,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Enumerate the names of all vector collections from their sled trees.
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
    async fn test_vector_dimension_tracking_rejects_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let engine = VectorEngine::new(&config(&dir)).unwrap();

        engine
            .insert(
                "dims",
                &serde_json::json!({"vector": [1.0, 2.0, 3.0]}),
                &tx(),
            )
            .await
            .unwrap();

        let err = engine
            .insert("dims", &serde_json::json!({"vector": [1.0, 2.0]}), &tx())
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn test_vector_rejects_non_numeric_elements() {
        let dir = tempfile::tempdir().unwrap();
        let engine = VectorEngine::new(&config(&dir)).unwrap();

        let err = engine
            .insert("bad", &serde_json::json!({"vector": [1.0, "x"]}), &tx())
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));
    }

    #[tokio::test]
    async fn test_vector_query_dimension_mismatch_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let engine = VectorEngine::new(&config(&dir)).unwrap();

        engine
            .insert("q", &serde_json::json!({"vector": [1.0, 2.0, 3.0]}), &tx())
            .await
            .unwrap();

        let err = engine
            .select(
                "q",
                Some(&serde_json::json!({"query_vector": [1.0, 2.0]})),
                10,
                0,
                &tx(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, crate::Error::InvalidRequest(_)));
    }

    #[tokio::test]
    async fn test_vector_zero_norm_similarity_is_zero_not_nan() {
        let dir = tempfile::tempdir().unwrap();
        let engine = VectorEngine::new(&config(&dir)).unwrap();

        engine
            .insert(
                "zero",
                &serde_json::json!({"vector": [0.0, 0.0, 0.0]}),
                &tx(),
            )
            .await
            .unwrap();

        let records = engine
            .select(
                "zero",
                Some(&serde_json::json!({"query_vector": [1.0, 0.0, 0.0]})),
                10,
                0,
                &tx(),
            )
            .await
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].metadata.get("similarity").unwrap(), "0");
    }
}
