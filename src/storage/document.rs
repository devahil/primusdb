/*
 * PrimusDB Document Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Collection-level encryption methods
 */

/*!
# PrimusDB Document Storage Engine

The document engine stores JSON documents in schema-less collections with
optional secondary indexes, an aggregation pipeline, and per-collection
encryption flags. Use it when you need flexible, nested, self-describing
records (content management, user profiles, event payloads) where the schema
can evolve without migrations.

```text
Document Engine Data Flow
═══════════════════════════════════════════════════

StorageEngine CRUD ──► DocumentEngine (Arc<RwLock<collections>>)
                             │
                             ├─► secondary indexes
                             │     (BTree / Hash / FullText / GeoSpatial)
                             ├─► aggregation pipeline
                             │     ($match $group $sort $project ...)
                             └─► sled tree per collection (durable)

Collection ──► sled tree "name"
  keys:   doc_1, doc_2, ...
  values: Document { data, version, vector_clock, checksum }
```

## Main Types & Functions

- [`DocumentEngine`]: JSON document storage engine implementing [`StorageEngine`].
- [`DocumentIndexType`]: index structures available for collection fields.
- `insert_many` / `update_many` / `delete_many`: bulk document operations.
- `aggregate`: aggregation pipeline (`$match`, `$group`, `$sort`, `$project`, `$limit`, `$skip`, `$count`).
- `create_index` / `drop_index` / `list_indexes`: secondary index management.
- `enable_collection_encryption` / `disable_collection_encryption` / `is_collection_encrypted`: per-collection encryption flags.
*/

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;

use sha2::Digest;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// An in-memory collection of documents with its secondary indexes.
#[derive(Debug, Serialize, Deserialize)]
struct DocumentCollection {
    /// Collection name.
    name: String,
    /// Documents keyed by `doc_{id}`.
    documents: HashMap<String, Document>,
    /// Secondary indexes keyed by index name.
    indexes: HashMap<String, DocumentIndex>,
    /// Next auto-increment document id.
    next_id: u64,
}

/// A single stored document with versioning metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    /// Document identifier (`doc_{id}`).
    id: String,
    /// Document body as JSON.
    data: serde_json::Value,
    /// Creation timestamp.
    created_at: chrono::DateTime<chrono::Utc>,
    /// Last modification timestamp.
    updated_at: chrono::DateTime<chrono::Utc>,
    /// Monotonic version counter.
    version: u64,
    /// Per-node counters used for conflict reconciliation.
    #[serde(default)]
    vector_clock: HashMap<String, u64>,
    /// SHA-256 checksum of the serialized body.
    #[serde(default)]
    checksum: String,
}

impl Document {
    fn compute_checksum(data: &serde_json::Value) -> Result<String> {
        let serialized = serde_json::to_string(data).map_err(crate::Error::SerializationError)?;
        Ok(format!("{:x}", sha2::Sha256::digest(serialized.as_bytes())))
    }

    fn new_doc(id: String, data: serde_json::Value, node_id: &str) -> Result<Self> {
        let mut vc = HashMap::new();
        vc.insert(node_id.to_string(), 1);
        Ok(Document {
            id,
            checksum: Self::compute_checksum(&data)?,
            data,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
            vector_clock: vc,
        })
    }

    fn increment_version(&mut self, node_id: &str) -> Result<()> {
        self.version += 1;
        self.updated_at = chrono::Utc::now();
        let counter = self.vector_clock.entry(node_id.to_string()).or_insert(0);
        *counter += 1;
        self.checksum = Self::compute_checksum(&self.data)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DocumentIndex {
    field: String,
    index_type: DocumentIndexType,
    data: HashMap<String, Vec<String>>,
}

/// Supported index types for document collections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentIndexType {
    /// Balanced tree index for range scans and ordered traversal.
    BTree,
    /// Hash index for fast equality lookups.
    Hash,
    /// Full-text index for text search on string fields.
    FullText,
    /// Geospatial index for location-based queries.
    GeoSpatial,
}

/// Document-oriented storage engine backed by sled.
///
/// Organises data into collections of JSON documents with optional indexing,
/// aggregation pipelines, and collection-level encryption support.
#[derive(Clone)]
pub struct DocumentEngine {
    config: PrimusDBConfig,
    collections: Arc<RwLock<HashMap<String, DocumentCollection>>>,
    /// Tracks which collections have encryption enabled.
    encrypted_collections: Arc<RwLock<HashMap<String, bool>>>,
    db: sled::Db,
}

impl DocumentEngine {
    /// Create a new document engine instance.
    ///
    /// Opens the sled database at `{data_dir}/document` and loads all
    /// persisted collections (and their documents) into memory.
    ///
    /// # Arguments
    /// * `config` - PrimusDB configuration containing the storage data directory.
    ///
    /// # Returns
    /// A ready-to-use [`DocumentEngine`] with collections loaded from disk.
    ///
    /// # Errors
    /// Returns an error if the sled database cannot be opened or a stored
    /// document cannot be deserialized.
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let path = format!("{}/document", config.storage.data_dir);
        let db = sled::open(&path)?;

        let engine = DocumentEngine {
            config: config.clone(),
            collections: Arc::new(RwLock::new(HashMap::new())),
            encrypted_collections: Arc::new(RwLock::new(HashMap::new())),
            db,
        };

        engine.load_from_sled()?;
        Ok(engine)
    }

    fn load_from_sled(&self) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        for tree_name in self.db.tree_names() {
            let name = String::from_utf8_lossy(&tree_name).to_string();
            if let Ok(tree) = self.db.open_tree(&name) {
                let mut documents = HashMap::new();
                let mut next_id: u64 = 1;
                for entry in &tree {
                    let (key, value) = entry?;
                    let doc: Document = serde_json::from_slice(&value)?;
                    let id_str = String::from_utf8_lossy(&key).to_string();
                    let numeric_id = id_str
                        .strip_prefix("doc_")
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    if numeric_id >= next_id {
                        next_id = numeric_id + 1;
                    }
                    documents.insert(id_str, doc);
                }
                if !documents.is_empty() || tree.iter().next().is_some() {
                    collections.insert(
                        name.clone(),
                        DocumentCollection {
                            name: name.clone(),
                            documents,
                            indexes: HashMap::new(),
                            next_id,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    /// Mark a collection as encrypted.
    ///
    /// Sets the in-memory encryption flag for `collection`. This is a
    /// flag-only operation; the read/write paths do not currently transform
    /// the stored data.
    pub fn enable_collection_encryption(&self, collection: &str) -> Result<()> {
        let mut encrypted = self.encrypted_collections.write().unwrap();
        encrypted.insert(collection.to_string(), true);
        info!("Encryption enabled for collection: {}", collection);
        Ok(())
    }

    /// Clear the encryption flag for a collection.
    pub fn disable_collection_encryption(&self, collection: &str) -> Result<()> {
        let mut encrypted = self.encrypted_collections.write().unwrap();
        encrypted.insert(collection.to_string(), false);
        info!("Encryption disabled for collection: {}", collection);
        Ok(())
    }

    /// Return whether a collection is currently flagged as encrypted.
    pub fn is_collection_encrypted(&self, collection: &str) -> Result<bool> {
        let encrypted = self.encrypted_collections.read().unwrap();
        Ok(*encrypted.get(collection).unwrap_or(&false))
    }

    /// Return the names of all collections currently flagged as encrypted.
    pub fn get_encrypted_collections(&self) -> Result<Vec<String>> {
        let encrypted = self.encrypted_collections.read().unwrap();
        Ok(encrypted
            .iter()
            .filter(|(_, v)| **v)
            .map(|(k, _)| k.clone())
            .collect())
    }

    // ── Index Methods ──────────────────────────────────────────────

    /// Create a secondary index on a field of a collection.
    ///
    /// Builds the index from the documents already present in the collection,
    /// mapping serialized field values to the IDs of documents that contain
    /// them. The index is then kept up to date on insert/update/delete.
    ///
    /// # Errors
    /// Returns an error if the collection does not exist or an index with the
    /// same name already exists on it.
    pub fn create_index(
        &self,
        collection: &str,
        index_name: &str,
        field: &str,
        index_type: DocumentIndexType,
    ) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        if let Some(col) = collections.get_mut(collection) {
            if col.indexes.contains_key(index_name) {
                return Err(crate::Error::DatabaseError(format!(
                    "Index '{}' already exists on collection '{}'",
                    index_name, collection
                )));
            }
            let mut index = DocumentIndex {
                field: field.to_string(),
                index_type,
                data: HashMap::new(),
            };
            // Populate index from existing documents
            for (doc_id, doc) in &col.documents {
                let field_value = resolve_field(&doc.data, field);
                if let Some(fv) = field_value {
                    let key =
                        serde_json::to_string(&fv).map_err(crate::Error::SerializationError)?;
                    index.data.entry(key).or_default().push(doc_id.clone());
                }
            }
            col.indexes.insert(index_name.to_string(), index);
            info!(
                "Created index '{}' on '{}' field in '{}'",
                index_name, field, collection
            );
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Collection '{}' not found",
                collection
            )))
        }
    }

    /// Remove a named secondary index from a collection.
    pub fn drop_index(&self, collection: &str, index_name: &str) -> Result<()> {
        let mut collections = self.collections.write().unwrap();
        if let Some(col) = collections.get_mut(collection) {
            col.indexes.remove(index_name);
            info!("Dropped index '{}' from '{}'", index_name, collection);
            Ok(())
        } else {
            Err(crate::Error::DatabaseError(format!(
                "Collection '{}' not found",
                collection
            )))
        }
    }

    /// List the names of all secondary indexes on a collection.
    pub fn list_indexes(&self, collection: &str) -> Result<Vec<String>> {
        let collections = self.collections.read().unwrap();
        if let Some(col) = collections.get(collection) {
            Ok(col.indexes.keys().cloned().collect())
        } else {
            Ok(vec![])
        }
    }

    fn update_indexes(
        indexes: &mut HashMap<String, DocumentIndex>,
        doc_id: &str,
        doc_data: &serde_json::Value,
        field: &str,
        old_value: Option<&serde_json::Value>,
        new_value: Option<&serde_json::Value>,
    ) -> Result<()> {
        for index in indexes.values_mut() {
            if index.field != field {
                continue;
            }
            if let Some(old) = old_value {
                let old_key =
                    serde_json::to_string(old).map_err(crate::Error::SerializationError)?;
                if let Some(ids) = index.data.get_mut(&old_key) {
                    ids.retain(|id| id != doc_id);
                }
            }
            if let Some(new) = new_value {
                let new_key =
                    serde_json::to_string(new).map_err(crate::Error::SerializationError)?;
                index
                    .data
                    .entry(new_key)
                    .or_insert_with(Vec::new)
                    .push(doc_id.to_string());
            }
        }
        if field.contains('.') {
            for index in indexes.values_mut() {
                if index.field == field || !field.starts_with(&index.field) {
                    continue;
                }
                let resolved = resolve_field(doc_data, &index.field);
                if let Some(rv) = resolved {
                    let key =
                        serde_json::to_string(&rv).map_err(crate::Error::SerializationError)?;
                    if let Some(entry) = index.data.get_mut(&key) {
                        if !entry.contains(&doc_id.to_string()) {
                            entry.push(doc_id.to_string());
                        }
                    } else {
                        index.data.insert(key, vec![doc_id.to_string()]);
                    }
                }
            }
        }
        Ok(())
    }

    // ── Bulk Operations ────────────────────────────────────────────

    /// Bulk-insert many documents into a collection.
    ///
    /// Creates the collection on demand, assigns sequential `doc_{n}` IDs,
    /// updates secondary indexes, and persists every document to the backing
    /// sled tree.
    ///
    /// # Returns
    /// The number of documents inserted.
    pub fn insert_many(&self, table: &str, documents: &[serde_json::Value]) -> Result<u64> {
        let mut collections = self.collections.write().unwrap();
        let collection =
            collections
                .entry(table.to_string())
                .or_insert_with(|| DocumentCollection {
                    name: table.to_string(),
                    documents: HashMap::new(),
                    indexes: HashMap::new(),
                    next_id: 1,
                });

        let node_id = &self.config.cluster.node_id;
        let mut count = 0u64;
        for data in documents {
            let id = format!("doc_{}", collection.next_id);
            collection.next_id += 1;
            let doc = Document::new_doc(id.clone(), data.clone(), node_id)?;

            // Update indexes for each field
            if let Some(obj) = data.as_object() {
                for (field, _) in obj {
                    Self::update_indexes(
                        &mut collection.indexes,
                        &id,
                        &doc.data,
                        field,
                        None,
                        Some(&doc.data),
                    )?;
                }
            }

            collection.documents.insert(id, doc);
            count += 1;
        }

        // Persist all documents
        let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
        for (id, doc) in &collection.documents {
            let value = serde_json::to_vec(doc).map_err(crate::Error::SerializationError)?;
            tree.insert(id.as_bytes(), value)
                .map_err(crate::Error::SledError)?;
        }
        tree.flush().map_err(crate::Error::SledError)?;

        info!("Inserted {} documents into {}", count, table);
        Ok(count)
    }

    /// Update documents in a collection matching the given conditions.
    ///
    /// Applies a shallow merge of the `update` object onto each matching
    /// document, increments its version and vector clock, refreshes secondary
    /// indexes, and persists the changes.
    ///
    /// # Returns
    /// The number of documents updated.
    pub fn update_many(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        update: &serde_json::Value,
    ) -> Result<u64> {
        let mut collections = self.collections.write().unwrap();
        let Some(collection) = collections.get_mut(table) else {
            return Ok(0);
        };

        let node_id = &self.config.cluster.node_id;
        let ids_to_update: Vec<String> = collection
            .documents
            .values()
            .filter(|doc| match &conditions {
                Some(cond) => Self::match_document(doc, cond),
                None => true,
            })
            .map(|doc| doc.id.clone())
            .collect();

        let updated = ids_to_update.len() as u64;
        for id in &ids_to_update {
            if let Some(doc) = collection.documents.get_mut(id) {
                let old_data = doc.data.clone();
                if let Some(obj) = update.as_object() {
                    for (key, value) in obj {
                        doc.data[key] = value.clone();
                    }
                }
                doc.increment_version(node_id)?;

                // Update indexes
                if let Some(obj) = update.as_object() {
                    for (field, _) in obj {
                        Self::update_indexes(
                            &mut collection.indexes,
                            id,
                            &doc.data,
                            field,
                            Some(&old_data),
                            Some(&doc.data),
                        )?;
                    }
                }

                let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
                let value = serde_json::to_vec(&*doc).map_err(crate::Error::SerializationError)?;
                tree.insert(id.as_bytes(), value)
                    .map_err(crate::Error::SledError)?;
            }
        }

        if updated > 0 {
            let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
            tree.flush().map_err(crate::Error::SledError)?;
        }
        Ok(updated)
    }

    /// Delete documents in a collection matching the given conditions.
    ///
    /// Removes matching documents from memory, the secondary indexes, and the
    /// backing sled tree.
    ///
    /// # Returns
    /// The number of documents deleted.
    pub fn delete_many(&self, table: &str, conditions: Option<&serde_json::Value>) -> Result<u64> {
        let mut collections = self.collections.write().unwrap();
        let Some(collection) = collections.get_mut(table) else {
            return Ok(0);
        };

        let ids_to_delete: Vec<String> = collection
            .documents
            .values()
            .filter(|doc| match &conditions {
                Some(cond) => Self::match_document(doc, cond),
                None => true,
            })
            .map(|doc| doc.id.clone())
            .collect();

        let deleted = ids_to_delete.len() as u64;
        for id in &ids_to_delete {
            // Remove from indexes
            if let Some(doc) = collection.documents.get(id) {
                if let Some(obj) = doc.data.as_object() {
                    for (field, val) in obj {
                        for index in collection.indexes.values_mut() {
                            if index.field == *field {
                                let key = serde_json::to_string(val)
                                    .map_err(crate::Error::SerializationError)?;
                                if let Some(ids) = index.data.get_mut(&key) {
                                    ids.retain(|i| i != id);
                                }
                            }
                        }
                    }
                }
            }
            collection.documents.remove(id);
            let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
            tree.remove(id.as_bytes())
                .map_err(crate::Error::SledError)?;
        }

        if deleted > 0 {
            let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
            tree.flush().map_err(crate::Error::SledError)?;
        }
        Ok(deleted)
    }

    // ── Aggregation Pipeline ───────────────────────────────────────

    /// Run an aggregation pipeline over a collection.
    ///
    /// Each stage object in `pipeline` is applied in order. Supported stages:
    /// `$match`, `$group` (with the accumulators `$sum`, `$avg`, `$min`,
    /// `$max`, `$first`, `$last`, `$count`, `$push`, `$addToSet`), `$sort`,
    /// `$project`, `$limit`, `$skip` and `$count`.
    ///
    /// # Returns
    /// The records produced by the final pipeline stage.
    pub fn aggregate(&self, table: &str, pipeline: &[serde_json::Value]) -> Result<Vec<Record>> {
        let collections = self.collections.read().unwrap();
        let Some(collection) = collections.get(table) else {
            return Ok(vec![]);
        };

        let mut records: Vec<Record> = collection
            .documents
            .values()
            .map(|doc| Record {
                id: doc.id.clone(),
                data: doc.data.clone(),
                metadata: HashMap::new(),
            })
            .collect();

        for stage in pipeline {
            records = self.apply_stage(stage, records)?;
        }

        Ok(records)
    }

    fn apply_stage(&self, stage: &serde_json::Value, input: Vec<Record>) -> Result<Vec<Record>> {
        let stage_obj = match stage.as_object() {
            Some(obj) => obj,
            None => return Ok(input),
        };

        if let Some(match_cond) = stage_obj.get("$match") {
            return Ok(self.stage_match(input, match_cond));
        }
        if let Some(group_spec) = stage_obj.get("$group") {
            return self.stage_group(input, group_spec);
        }
        if let Some(sort_spec) = stage_obj.get("$sort") {
            return Ok(self.stage_sort(input, sort_spec));
        }
        if let Some(project_spec) = stage_obj.get("$project") {
            return Ok(self.stage_project(input, project_spec));
        }
        if let Some(limit_val) = stage_obj.get("$limit").and_then(|v| v.as_u64()) {
            return Ok(self.stage_limit(input, limit_val));
        }
        if let Some(skip_val) = stage_obj.get("$skip").and_then(|v| v.as_u64()) {
            return Ok(self.stage_skip(input, skip_val));
        }
        if let Some(count_name) = stage_obj.get("$count").and_then(|v| v.as_str()) {
            return Ok(self.stage_count(input, count_name));
        }

        Ok(input)
    }

    fn stage_match(&self, input: Vec<Record>, condition: &serde_json::Value) -> Vec<Record> {
        input
            .into_iter()
            .filter(|rec| {
                Self::match_document(
                    &Document {
                        id: rec.id.clone(),
                        data: rec.data.clone(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                        version: 0,
                        vector_clock: HashMap::new(),
                        checksum: String::new(),
                    },
                    condition,
                )
            })
            .collect()
    }

    fn stage_group(
        &self,
        input: Vec<Record>,
        group_spec: &serde_json::Value,
    ) -> Result<Vec<Record>> {
        let id_field = group_spec
            .get("_id")
            .and_then(|v| v.as_str())
            .unwrap_or("null");

        let mut groups: HashMap<String, Vec<&Record>> = HashMap::new();
        for rec in &input {
            let key = if id_field == "null" {
                "null".to_string()
            } else {
                match resolve_field(&rec.data, id_field) {
                    Some(v) => {
                        serde_json::to_string(&v).map_err(crate::Error::SerializationError)?
                    }
                    None => "null".to_string(),
                }
            };
            groups.entry(key).or_default().push(rec);
        }

        let mut results = Vec::new();
        for (_key, group) in groups {
            let mut result = serde_json::Map::new();

            // _id field
            let id_val = if id_field == "null" {
                serde_json::Value::Null
            } else {
                group
                    .first()
                    .and_then(|r| resolve_field(&r.data, id_field))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null)
            };
            result.insert("_id".to_string(), id_val);

            // Accumulators
            for (field, expr) in group_spec.as_object().unwrap() {
                if field == "_id" {
                    continue;
                }
                let acc = self.evaluate_accumulator(field, expr, &group)?;
                if let Some((acc_name, acc_val)) = acc {
                    result.insert(acc_name, acc_val);
                }
            }

            results.push(Record {
                id: String::new(),
                data: serde_json::Value::Object(result),
                metadata: HashMap::new(),
            });
        }

        Ok(results)
    }

    fn evaluate_accumulator(
        &self,
        field: &str,
        expr: &serde_json::Value,
        group: &[&Record],
    ) -> Result<Option<(String, serde_json::Value)>> {
        let op = match expr.as_object().and_then(|o| o.keys().next()) {
            Some(k) => k.clone(),
            None => return Ok(None),
        };
        let arg = expr.as_object().and_then(|o| o.values().next());

        match op.as_str() {
            "$sum" => {
                let total: f64 = group
                    .iter()
                    .filter_map(|r| {
                        arg.and_then(|a| {
                            let field_name = a.as_str()?;
                            resolve_field(&r.data, field_name)?.as_f64()
                        })
                    })
                    .sum();
                let val = if total.fract() == 0.0 {
                    serde_json::json!(total as i64)
                } else {
                    serde_json::json!(total)
                };
                Ok(Some((field.to_string(), val)))
            }
            "$avg" => {
                let values: Vec<f64> = group
                    .iter()
                    .filter_map(|r| {
                        arg.and_then(|a| {
                            let field_name = a.as_str()?;
                            resolve_field(&r.data, field_name)?.as_f64()
                        })
                    })
                    .collect();
                let avg = if values.is_empty() {
                    0.0
                } else {
                    values.iter().sum::<f64>() / values.len() as f64
                };
                Ok(Some((field.to_string(), serde_json::json!(avg))))
            }
            "$min" => {
                let min_val = group
                    .iter()
                    .filter_map(|r| {
                        arg.and_then(|a| {
                            let field_name = a.as_str()?;
                            resolve_field(&r.data, field_name)
                        })
                    })
                    .cloned()
                    .min_by(compare_json);
                Ok(min_val.map(|v| (field.to_string(), v)))
            }
            "$max" => {
                let max_val = group
                    .iter()
                    .filter_map(|r| {
                        arg.and_then(|a| {
                            let field_name = a.as_str()?;
                            resolve_field(&r.data, field_name)
                        })
                    })
                    .cloned()
                    .max_by(compare_json);
                Ok(max_val.map(|v| (field.to_string(), v)))
            }
            "$first" => {
                let val = group.first().and_then(|r| {
                    arg.and_then(|a| {
                        let field_name = a.as_str()?;
                        resolve_field(&r.data, field_name).cloned()
                    })
                });
                Ok(val.map(|v| (field.to_string(), v)))
            }
            "$last" => {
                let val = group.last().and_then(|r| {
                    arg.and_then(|a| {
                        let field_name = a.as_str()?;
                        resolve_field(&r.data, field_name).cloned()
                    })
                });
                Ok(val.map(|v| (field.to_string(), v)))
            }
            "$count" => {
                let count = group.len();
                Ok(Some((field.to_string(), serde_json::json!(count))))
            }
            "$push" => {
                let arr: Vec<serde_json::Value> = group
                    .iter()
                    .filter_map(|r| {
                        arg.and_then(|a| {
                            let field_name = a.as_str()?;
                            resolve_field(&r.data, field_name).cloned()
                        })
                    })
                    .collect();
                Ok(Some((field.to_string(), serde_json::Value::Array(arr))))
            }
            "$addToSet" => {
                let mut seen = std::collections::HashSet::new();
                let arr: Vec<serde_json::Value> = group
                    .iter()
                    .filter_map(|r| {
                        arg.and_then(|a| {
                            let field_name = a.as_str()?;
                            let val = resolve_field(&r.data, field_name)?;
                            if seen.insert(serde_json::to_string(val).unwrap_or_default()) {
                                Some(val.clone())
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                Ok(Some((field.to_string(), serde_json::Value::Array(arr))))
            }
            _ => Ok(None),
        }
    }

    fn stage_sort(&self, input: Vec<Record>, sort_spec: &serde_json::Value) -> Vec<Record> {
        let mut records = input;
        let sort_obj = match sort_spec.as_object() {
            Some(obj) => obj,
            None => return records,
        };

        if let Some((field, order)) = sort_obj.iter().next() {
            let descending = order.as_i64().unwrap_or(1) < 0;
            records.sort_by(|a, b| {
                let a_val = resolve_field(&a.data, field);
                let b_val = resolve_field(&b.data, field);
                let cmp = compare_json_option(a_val, b_val);
                if descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        records
    }

    fn stage_project(&self, input: Vec<Record>, project_spec: &serde_json::Value) -> Vec<Record> {
        input
            .into_iter()
            .map(|rec| {
                let mut new_data = serde_json::Map::new();
                if let Some(obj) = project_spec.as_object() {
                    for (field, value) in obj {
                        let inclusion = value.as_bool().unwrap_or(false);
                        if inclusion {
                            if let Some(val) = resolve_field(&rec.data, field) {
                                new_data.insert(field.clone(), val.clone());
                            }
                        }
                    }
                }
                Record {
                    id: rec.id,
                    data: serde_json::Value::Object(new_data),
                    metadata: rec.metadata,
                }
            })
            .collect()
    }

    fn stage_limit(&self, input: Vec<Record>, limit: u64) -> Vec<Record> {
        input.into_iter().take(limit as usize).collect()
    }

    fn stage_skip(&self, input: Vec<Record>, skip: u64) -> Vec<Record> {
        input.into_iter().skip(skip as usize).collect()
    }

    fn stage_count(&self, input: Vec<Record>, name: &str) -> Vec<Record> {
        let count = input.len();
        vec![Record {
            id: String::new(),
            data: serde_json::json!({ name: count }),
            metadata: HashMap::new(),
        }]
    }

    fn match_document(document: &Document, conditions: &serde_json::Value) -> bool {
        match conditions {
            serde_json::Value::Object(obj) => {
                for (key, value) in obj {
                    if key == "$and" {
                        if let Some(arr) = value.as_array() {
                            if !arr.iter().all(|c| Self::match_document(document, c)) {
                                return false;
                            }
                        }
                    } else if key == "$or" {
                        if let Some(arr) = value.as_array() {
                            if !arr.iter().any(|c| Self::match_document(document, c)) {
                                return false;
                            }
                        }
                    } else if key == "$nor" {
                        if let Some(arr) = value.as_array() {
                            if arr.iter().any(|c| Self::match_document(document, c)) {
                                return false;
                            }
                        }
                    } else if key == "$not" {
                        if Self::match_document(document, value) {
                            return false;
                        }
                    } else {
                        if !Self::match_field(&document.data, key, value) {
                            return false;
                        }
                    }
                }
                true
            }
            _ => true,
        }
    }

    fn match_field(data: &serde_json::Value, field: &str, condition: &serde_json::Value) -> bool {
        let parts: Vec<&str> = field.split('.').collect();
        let mut current = data;

        for part in parts {
            match current.get(part) {
                Some(value) => current = value,
                None => return false,
            }
        }

        match condition {
            serde_json::Value::Object(op_obj) => {
                if let Some((op, operand)) = op_obj.into_iter().next() {
                    return match op.as_str() {
                        "$gt" => compare_gt(current, operand),
                        "$lt" => compare_lt(current, operand),
                        "$gte" => compare_gte(current, operand),
                        "$lte" => compare_lte(current, operand),
                        "$ne" => current != operand,
                        "$in" => operand.as_array().is_some_and(|arr| arr.contains(current)),
                        "$nin" => operand.as_array().is_none_or(|arr| !arr.contains(current)),
                        "$regex" => {
                            if let (Some(s), Some(pattern)) = (current.as_str(), operand.as_str()) {
                                regex::Regex::new(pattern)
                                    .map(|re| re.is_match(s))
                                    .unwrap_or(false)
                            } else {
                                false
                            }
                        }
                        "$exists" => {
                            let exists = !current.is_null();
                            if operand.as_bool().unwrap_or(true) {
                                exists
                            } else {
                                !exists
                            }
                        }
                        "$size" => {
                            if let (Some(arr), Some(size)) = (current.as_array(), operand.as_u64())
                            {
                                arr.len() as u64 == size
                            } else {
                                false
                            }
                        }
                        "$all" => {
                            if let (Some(arr), Some(values)) =
                                (current.as_array(), operand.as_array())
                            {
                                values.iter().all(|v| arr.contains(v))
                            } else {
                                false
                            }
                        }
                        "$elemMatch" => {
                            if let Some(arr) = current.as_array() {
                                arr.iter().any(|elem| match operand {
                                    serde_json::Value::Object(_) => Self::match_document(
                                        &Document {
                                            id: String::new(),
                                            data: elem.clone(),
                                            created_at: chrono::Utc::now(),
                                            updated_at: chrono::Utc::now(),
                                            version: 0,
                                            vector_clock: HashMap::new(),
                                            checksum: String::new(),
                                        },
                                        operand,
                                    ),
                                    _ => elem == operand,
                                })
                            } else {
                                false
                            }
                        }
                        _ => current == condition,
                    };
                }
                true
            }
            serde_json::Value::String(s) if s.starts_with("$regex:") => {
                let regex_str = &s[7..];
                if let Ok(re) = regex::Regex::new(regex_str) {
                    if let Some(s) = current.as_str() {
                        re.is_match(s)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            serde_json::Value::String(s) if s == "$exists:true" => !current.is_null(),
            serde_json::Value::String(s) if s == "$exists:false" => current.is_null(),
            _ => current == condition,
        }
    }
}

fn compare_gt(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(an), serde_json::Value::Number(bn)) => {
            an.as_f64().unwrap_or(0.0) > bn.as_f64().unwrap_or(0.0)
        }
        (serde_json::Value::String(as_), serde_json::Value::String(bs)) => as_ > bs,
        _ => false,
    }
}

fn compare_lt(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(an), serde_json::Value::Number(bn)) => {
            an.as_f64().unwrap_or(0.0) < bn.as_f64().unwrap_or(0.0)
        }
        (serde_json::Value::String(as_), serde_json::Value::String(bs)) => as_ < bs,
        _ => false,
    }
}

fn compare_gte(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    compare_gt(a, b) || a == b
}

fn compare_lte(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    compare_lt(a, b) || a == b
}

fn resolve_field<'a>(data: &'a serde_json::Value, field: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = field.split('.').collect();
    let mut current = data;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

fn compare_json(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    match (a, b) {
        (serde_json::Value::Number(an), serde_json::Value::Number(bn)) => an
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&bn.as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal),
        (serde_json::Value::String(as_), serde_json::Value::String(bs)) => as_.cmp(bs),
        (serde_json::Value::Bool(ab), serde_json::Value::Bool(bb)) => ab.cmp(bb),
        _ => std::cmp::Ordering::Equal,
    }
}

fn compare_json_option(
    a: Option<&serde_json::Value>,
    b: Option<&serde_json::Value>,
) -> std::cmp::Ordering {
    match (a, b) {
        (Some(av), Some(bv)) => compare_json(av, bv),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[async_trait]
impl StorageEngine for DocumentEngine {
    /// Insert a single JSON document into a collection.
    ///
    /// Assigns a sequential `doc_{n}` ID, records creation metadata, updates
    /// secondary indexes, and persists the document to sled.
    ///
    /// # Returns
    /// `1` on success.
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        info!("Document insert into {}: {:?}", table, data);

        let mut collections = self.collections.write().unwrap();
        let collection =
            collections
                .entry(table.to_string())
                .or_insert_with(|| DocumentCollection {
                    name: table.to_string(),
                    documents: HashMap::new(),
                    indexes: HashMap::new(),
                    next_id: 1,
                });

        let id = format!("doc_{}", collection.next_id);
        collection.next_id += 1;

        let node_id = &self.config.cluster.node_id;
        let document = Document::new_doc(id.clone(), data.clone(), node_id)?;

        // Update indexes
        if let Some(obj) = data.as_object() {
            for (field, _) in obj {
                Self::update_indexes(
                    &mut collection.indexes,
                    &id,
                    &document.data,
                    field,
                    None,
                    Some(&document.data),
                )?;
            }
        }

        collection.documents.insert(id.clone(), document.clone());

        let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
        let value = serde_json::to_vec(&document).map_err(crate::Error::SerializationError)?;
        tree.insert(id.as_bytes(), value)
            .map_err(crate::Error::SledError)?;
        tree.flush().map_err(crate::Error::SledError)?;

        Ok(1)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Query documents from a collection with optional filter conditions.
    ///
    /// Supports MongoDB-style operators (`$and`, `$or`, `$nor`, `$not`,
    /// `$gt`, `$gte`, `$lt`, `$lte`, `$ne`, `$in`, `$nin`, `$regex`,
    /// `$exists`, `$size`, `$all`, `$elemMatch`) and dot-notation field
    /// paths for nested documents. Results are paginated with `limit`/`offset`.
    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        info!(
            "Document query from {} with conditions: {:?}",
            table, conditions
        );

        let collections = self.collections.read().unwrap();
        if let Some(collection) = collections.get(table) {
            let records: Vec<Record> = collection
                .documents
                .values()
                .filter(|doc| match &conditions {
                    Some(cond) => Self::match_document(doc, cond),
                    None => true,
                })
                .skip(offset as usize)
                .take(limit as usize)
                .map(|doc| Record {
                    id: doc.id.clone(),
                    data: doc.data.clone(),
                    metadata: HashMap::new(),
                })
                .collect();
            Ok(records)
        } else {
            Ok(vec![])
        }
    }

    /// Update documents in a collection matching the given conditions.
    ///
    /// Shallow-merges `data` into each matching document, bumps the version
    /// and vector clock, and persists the change.
    ///
    /// # Returns
    /// The number of documents updated.
    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        info!(
            "Document update in {} with conditions: {:?}",
            table, conditions
        );

        let mut collections = self.collections.write().unwrap();
        let Some(collection) = collections.get_mut(table) else {
            return Ok(0);
        };

        let ids_to_update: Vec<String> = collection
            .documents
            .values()
            .filter(|doc| match &conditions {
                Some(cond) => Self::match_document(doc, cond),
                None => true,
            })
            .map(|doc| doc.id.clone())
            .collect();

        let updated = ids_to_update.len() as u64;
        let node_id = &self.config.cluster.node_id;

        for id in &ids_to_update {
            if let Some(doc) = collection.documents.get_mut(id) {
                if let Some(obj) = data.as_object() {
                    for (key, value) in obj {
                        doc.data[key] = value.clone();
                    }
                }
                doc.increment_version(node_id)?;

                let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
                let value = serde_json::to_vec(&*doc).map_err(crate::Error::SerializationError)?;
                tree.insert(id.as_bytes(), value)
                    .map_err(crate::Error::SledError)?;
            }
        }

        if updated > 0 {
            let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
            tree.flush().map_err(crate::Error::SledError)?;
        }

        Ok(updated)
    }

    /// Delete documents in a collection matching the given conditions.
    ///
    /// # Returns
    /// The number of documents deleted.
    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        info!(
            "Document delete from {} with conditions: {:?}",
            table, conditions
        );

        let mut collections = self.collections.write().unwrap();
        let Some(collection) = collections.get_mut(table) else {
            return Ok(0);
        };

        let ids_to_delete: Vec<String> = collection
            .documents
            .values()
            .filter(|doc| match &conditions {
                Some(cond) => Self::match_document(doc, cond),
                None => true,
            })
            .map(|doc| doc.id.clone())
            .collect();

        let deleted = ids_to_delete.len() as u64;

        for id in &ids_to_delete {
            collection.documents.remove(id);
            let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
            tree.remove(id.as_bytes())
                .map_err(crate::Error::SledError)?;
        }

        if deleted > 0 {
            let tree = self.db.open_tree(table).map_err(crate::Error::SledError)?;
            tree.flush().map_err(crate::Error::SledError)?;
        }

        Ok(deleted)
    }

    /// Produce collection statistics as a JSON string.
    ///
    /// Reports document count, average number of fields per document, index
    /// count, and the next auto-generated document ID.
    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        info!("Document analyze for collection: {}", table);

        let collections = self.collections.read().unwrap();
        let stats = if let Some(collection) = collections.get(table) {
            let doc_count = collection.documents.len();
            let total_fields: usize = collection
                .documents
                .values()
                .map(|doc| match &doc.data {
                    serde_json::Value::Object(obj) => obj.len(),
                    _ => 0,
                })
                .sum();
            serde_json::json!({
                "collection": table,
                "document_count": doc_count,
                "average_fields_per_document": if doc_count > 0 {
                    total_fields as f64 / doc_count as f64
                } else {
                    0.0
                },
                "index_count": collection.indexes.len(),
                "next_id": collection.next_id,
            })
        } else {
            serde_json::json!({
                "collection": table,
                "document_count": 0,
                "average_fields_per_document": 0.0,
                "index_count": 0,
                "next_id": 0,
            })
        };

        Ok(stats.to_string())
    }

    /// Create an empty document collection.
    ///
    /// Opens the backing sled tree and registers the collection in memory.
    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        info!("Creating document collection: {}", table);

        let _tree = self.db.open_tree(table)?;

        let mut collections = self.collections.write().unwrap();
        collections.insert(
            table.to_string(),
            DocumentCollection {
                name: table.to_string(),
                documents: HashMap::new(),
                indexes: HashMap::new(),
                next_id: 1,
            },
        );

        Ok(())
    }

    /// Drop a collection and its backing sled tree.
    async fn drop_table(&self, table: &str) -> Result<()> {
        info!("Dropping document collection: {}", table);

        self.db.drop_tree(table)?;

        let mut collections = self.collections.write().unwrap();
        collections.remove(table);

        Ok(())
    }

    /// Remove all documents from a collection and reset its ID counter.
    async fn truncate_table(&self, table: &str, _cascade: bool) -> Result<()> {
        info!("Truncating document collection: {}", table);

        let mut collections = self.collections.write().unwrap();
        if let Some(collection) = collections.get_mut(table) {
            collection.documents.clear();
            collection.next_id = 1;
        }

        self.db.drop_tree(table)?;
        let _tree = self.db.open_tree(table)?;

        Ok(())
    }

    /// Return schema and statistics about a collection.
    ///
    /// Builds a [`TableInfo`] with inferred field types, secondary indexes,
    /// and the current document count.
    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        info!("Getting document collection info for: {}", table);

        let collections = self.collections.read().unwrap();
        if let Some(collection) = collections.get(table) {
            let indexes: Vec<crate::storage::Index> = collection
                .indexes
                .values()
                .map(|i| crate::storage::Index {
                    name: format!("idx_{}", i.field),
                    fields: vec![i.field.clone()],
                    index_type: crate::storage::IndexType::BTree,
                    unique: false,
                })
                .collect();
            let mut field_names: Vec<String> = Vec::new();
            for doc in collection.documents.values() {
                if let serde_json::Value::Object(map) = &doc.data {
                    for key in map.keys() {
                        if !field_names.contains(key) {
                            field_names.push(key.clone());
                        }
                    }
                }
            }
            let fields: Vec<crate::storage::Field> = field_names
                .into_iter()
                .map(|name| crate::storage::Field {
                    name,
                    field_type: crate::storage::FieldType::Text,
                    nullable: true,
                    default_value: None,
                    constraints: vec![],
                })
                .collect();
            Ok(TableInfo {
                name: table.to_string(),
                schema: Schema {
                    fields,
                    indexes,
                    constraints: vec![],
                },
                row_count: collection.documents.len() as u64,
                size_bytes: 0,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        } else {
            Err(crate::Error::DatabaseError(
                "Collection not found".to_string(),
            ))
        }
    }

    /// Enumerate the names of all document collections.
    fn list_tables(&self) -> Result<Vec<String>> {
        let collections = self.collections.read().unwrap();
        let mut names: Vec<String> = collections.keys().cloned().collect();
        names.sort();
        Ok(names)
    }
}
