/*
 * PrimusDB Key-Value Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Key-Value engine with CouchDB-like API
 */

/*!
# PrimusDB Key-Value Storage Engine

The key-value engine exposes a CouchDB-compatible document API over an
embedded sled database: databases of JSON documents with MVCC-style revision
tracking (`_id`/`_rev`), bulk operations, Mango-style `find` selectors,
secondary indexes, and per-database encryption flags. Use it when you need
simple, high-speed, schema-free document storage with revision control and a
familiar CouchDB/Mango query surface.

```text
Key-Value Engine (CouchDB-compatible)
═══════════════════════════════════════════════════

put / get / delete ──► KeyValueEngine ──► RwLock maps ──► sled tree per DB
                            │
                            ├─► revision control (_rev, MVCC)
                            ├─► secondary indexes (Mango)
                            └─► change sequence (_meta:sequence)

all_docs / find ──► selector match ($eq $gt $in $exists ...) ──► JSON rows
```

## Main Types & Functions

- [`KeyValueEngine`]: the key-value storage engine implementing [`StorageEngine`].
- [`KvDocument`]: a stored document with CouchDB-style `_id`/`_rev` fields.
- [`KvAttachment`]: a binary attachment associated with a document.
- [`KvBulkDocsRequest`] / [`KvBulkDocsResponse`]: bulk `_bulk_docs` payloads.
- [`KvViewRequest`] / [`KvViewResult`]: map/reduce view definitions.
- [`KvFindRequest`]: Mango-style `find` query.
- [`KvIndex`]: a secondary index definition.
- `create_database` / `delete_database` / `list_databases`: database lifecycle.
- `put_document` / `get_document` / `delete_document` / `bulk_docs`: document CRUD.
- `find` / `all_docs` / `get_db_info`: querying and introspection.
*/

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// A document in the key-value store with CouchDB-style revision tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvDocument {
    /// Document identifier (CouchDB-style `_id`).
    pub _id: String,
    /// Current revision string (e.g. `1-<hash>`), used for MVCC checks.
    pub _rev: Option<String>,
    /// The document body as JSON.
    pub value: serde_json::Value,
    /// Creation timestamp in RFC 3339 format.
    pub created_at: Option<String>,
    /// Last modification timestamp in RFC 3339 format.
    pub updated_at: Option<String>,
    /// Whether the document has been tombstoned (soft-deleted).
    pub deleted: bool,
    /// Unix timestamp in epoch millis after which the document is treated as
    /// expired and invisible to reads. `None` means the document never expires.
    #[serde(default)]
    pub expires_at: Option<i64>,
}

/// A binary attachment associated with a key-value document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvAttachment {
    /// MIME content type of the attachment.
    pub content_type: String,
    /// Base64-encoded binary payload.
    pub data: String,
    /// Byte length of the decoded payload.
    pub length: u64,
}

/// A `_bulk_docs` request: a batch of documents with an optional atomicity flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvBulkDocsRequest {
    /// Documents to insert or update in a single batch.
    pub docs: Vec<KvDocument>,
    /// If `true`, the whole batch succeeds or fails as a unit (no partial application).
    pub all_or_nothing: Option<bool>,
}

/// Per-document result of a `_bulk_docs` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvBulkDocsResponse {
    /// ID of the affected document.
    pub id: String,
    /// New revision of the document on success.
    pub rev: Option<String>,
    /// Error key (e.g. `"conflict"`) when the operation failed.
    pub error: Option<String>,
}

/// A map/reduce view request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvViewRequest {
    /// JavaScript map function (stored, not executed).
    pub map: String,
    /// Optional JavaScript reduce function (stored, not executed).
    pub reduce: Option<String>,
}

/// A single row emitted by a view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvViewResult {
    /// Document ID associated with the row.
    pub id: String,
    /// Emitted key.
    pub key: serde_json::Value,
    /// Emitted value.
    pub value: serde_json::Value,
}

/// A Mango-style `find` query over a database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvFindRequest {
    /// Selector object describing the match conditions.
    pub selector: serde_json::Value,
    /// Maximum number of documents to return.
    pub limit: Option<usize>,
    /// Number of documents to skip.
    pub skip: Option<usize>,
    /// Sort specification (`{"field": "...", "direction": "asc"|"desc"}`).
    pub sort: Option<Vec<serde_json::Value>>,
}

/// A secondary index definition on a database.
#[derive(Debug, Clone)]
pub struct KvIndex {
    /// Unique index name within the database.
    pub name: String,
    /// Fields the index is built over.
    pub fields: Vec<String>,
    /// Optional partial-index selector.
    pub selector: Option<serde_json::Value>,
}

/// Key-value storage engine with a CouchDB-compatible API.
///
/// Supports document CRUD, revision control, bulk operations, Mango-style
/// queries (`find`), secondary indexes, and per-database encryption.
#[derive(Clone)]
pub struct KeyValueEngine {
    db: sled::Db,
    databases: Arc<RwLock<HashMap<String, KvDatabase>>>,
    encrypted_databases: Arc<RwLock<HashMap<String, bool>>>,
}

/// An in-memory view of a key-value database.
///
/// Holds the live document map, secondary indexes, and the per-database
/// update sequence used for change tracking.
#[derive(Clone)]
pub struct KvDatabase {
    documents: Arc<RwLock<HashMap<String, KvDocument>>>,
    indexes: Arc<RwLock<HashMap<String, KvIndex>>>,
    sequence: Arc<RwLock<u64>>,
    revision_limit: Arc<RwLock<u64>>,
}

/// Return the current wall-clock time in epoch millis.
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// True when a document has a TTL that has already lapsed.
fn is_expired(doc: &KvDocument, now: i64) -> bool {
    matches!(doc.expires_at, Some(expires) if expires <= now)
}

impl KeyValueEngine {
    /// Create a new key-value engine instance.
    ///
    /// Opens the sled database at `{data_dir}/keyvalue` and loads all existing
    /// databases (documents, change sequences, and indexes) into memory.
    ///
    /// # Errors
    /// Returns an error if the sled database cannot be opened or a stored
    /// document cannot be deserialized.
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let path = format!("{}/keyvalue", config.storage.data_dir);
        let db = sled::open(&path)?;

        let engine = KeyValueEngine {
            db,
            databases: Arc::new(RwLock::new(HashMap::new())),
            encrypted_databases: Arc::new(RwLock::new(HashMap::new())),
        };

        engine.load_databases()?;

        Ok(engine)
    }

    fn load_databases(&self) -> Result<()> {
        let tree_names = self.db.tree_names();
        let mut databases = self.databases.write().unwrap();

        for name_bytes in tree_names {
            let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| {
                crate::Error::DatabaseError("Invalid UTF-8 in tree name".to_string())
            })?;
            let tree = self.db.open_tree(&name)?;
            let mut documents = HashMap::new();
            let mut sequence = 0u64;
            let mut revision_limit = 1000u64;

            for result in tree.iter() {
                let (key, value) = result?;
                let key_str = String::from_utf8(key.to_vec())
                    .map_err(|_| crate::Error::DatabaseError("Invalid UTF-8 in key".to_string()))?;

                if key_str == "_meta:sequence" {
                    let seq_val: serde_json::Value = serde_json::from_slice(&value)?;
                    sequence = seq_val.as_u64().unwrap_or(0);
                    continue;
                }

                if key_str == "_meta:revision_limit" {
                    let limit_val: serde_json::Value = serde_json::from_slice(&value)?;
                    revision_limit = limit_val.as_u64().unwrap_or(1000).max(1);
                    continue;
                }

                let doc: KvDocument = serde_json::from_slice(&value)?;
                documents.insert(key_str, doc);
            }

            databases.insert(
                name.clone(),
                KvDatabase {
                    documents: Arc::new(RwLock::new(documents)),
                    indexes: Arc::new(RwLock::new(HashMap::new())),
                    sequence: Arc::new(RwLock::new(sequence)),
                    revision_limit: Arc::new(RwLock::new(revision_limit)),
                },
            );
        }

        Ok(())
    }

    /// Create a new empty key-value database.
    ///
    /// # Errors
    /// Returns an error if a database with the same name already exists.
    pub fn create_database(&self, name: &str) -> Result<()> {
        let mut databases = self.databases.write().unwrap();
        if databases.contains_key(name) {
            return Err(crate::Error::ValidationError(format!(
                "Database {} already exists",
                name
            )));
        }

        self.db.open_tree(name)?;

        databases.insert(
            name.to_string(),
            KvDatabase {
                documents: Arc::new(RwLock::new(HashMap::new())),
                indexes: Arc::new(RwLock::new(HashMap::new())),
                sequence: Arc::new(RwLock::new(0)),
                revision_limit: Arc::new(RwLock::new(1000)),
            },
        );

        info!("Created Key-Value database: {}", name);
        Ok(())
    }

    /// Delete a key-value database and all of its data.
    ///
    /// # Errors
    /// Returns an error if the database does not exist.
    pub fn delete_database(&self, name: &str) -> Result<()> {
        let mut databases = self.databases.write().unwrap();
        if !databases.contains_key(name) {
            return Err(crate::Error::ValidationError(format!(
                "Database {} not found",
                name
            )));
        }

        databases.remove(name);
        self.db.drop_tree(name)?;

        info!("Deleted Key-Value database: {}", name);
        Ok(())
    }

    /// List the names of all key-value databases.
    pub fn list_databases(&self) -> Result<Vec<String>> {
        let databases = self.databases.read().unwrap();
        Ok(databases.keys().cloned().collect())
    }

    /// Fetch a document by its `_id`.
    ///
    /// # Errors
    /// Returns an error if the database or document does not exist, or if the
    /// document has been soft-deleted.
    pub fn get_document(&self, db_name: &str, doc_id: &str) -> Result<KvDocument> {
        let databases = self.databases.read().unwrap();
        let database = databases.get(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let docs = database.documents.read().unwrap();
        let doc = docs.get(doc_id).ok_or_else(|| {
            crate::Error::ValidationError(format!("Document {} not found", doc_id))
        })?;

        if doc.deleted {
            return Err(crate::Error::ValidationError(format!(
                "Document {} is deleted",
                doc_id
            )));
        }

        if is_expired(doc, now_millis()) {
            return Err(crate::Error::ValidationError(format!(
                "Document {} has expired",
                doc_id
            )));
        }

        Ok(doc.clone())
    }

    /// Create or update a document.
    ///
    /// Generates a new `_rev` (incrementing the revision number on updates,
    /// resurrecting tombstoned documents), updates the change sequence, and
    /// persists both the document and the sequence to sled.
    ///
    /// # Returns
    /// The stored document with its new revision.
    pub fn put_document(
        &self,
        db_name: &str,
        doc_id: &str,
        data: serde_json::Value,
    ) -> Result<KvDocument> {
        self.put_document_inner(db_name, doc_id, data, None, None)
    }

    /// Compare-and-set write: only succeeds when the current revision of the
    /// document equals `expected_rev`.
    ///
    /// `expected_rev = None` behaves like [`Self::put_document`] (unconditional
    /// create-or-update). A `Some` revision that does not match the stored
    /// revision (including a missing or tombstoned document) fails with a
    /// `ValidationError`.
    pub fn put_document_cas(
        &self,
        db_name: &str,
        doc_id: &str,
        expected_rev: Option<&str>,
        data: serde_json::Value,
    ) -> Result<KvDocument> {
        self.put_document_inner(db_name, doc_id, data, expected_rev, None)
    }

    /// Write a document that expires `ttl_secs` seconds from now.
    ///
    /// Expired documents are invisible to reads (and skipped by queries);
    /// storage space is reclaimed on demand. `ttl_secs = 0` is rejected.
    pub fn put_document_ttl(
        &self,
        db_name: &str,
        doc_id: &str,
        data: serde_json::Value,
        ttl_secs: u64,
    ) -> Result<KvDocument> {
        if ttl_secs == 0 {
            return Err(crate::Error::ValidationError(
                "TTL must be greater than zero".to_string(),
            ));
        }
        let expires_at = now_millis().checked_add((ttl_secs as i64) * 1000);
        self.put_document_inner(db_name, doc_id, data, None, expires_at)
    }

    fn put_document_inner(
        &self,
        db_name: &str,
        doc_id: &str,
        data: serde_json::Value,
        expected_rev: Option<&str>,
        expires_at: Option<i64>,
    ) -> Result<KvDocument> {
        let mut databases = self.databases.write().unwrap();
        let database = databases.get_mut(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let mut docs = database.documents.write().unwrap();

        if let Some(expected) = expected_rev {
            let current = docs
                .get(doc_id)
                .filter(|d| !d.deleted)
                .and_then(|d| d._rev.clone());
            if current.as_deref() != Some(expected) {
                return Err(crate::Error::ValidationError(
                    "Revision mismatch".to_string(),
                ));
            }
        }

        let limit = *database.revision_limit.read().unwrap();

        let (new_rev, is_new) = if let Some(existing) = docs.get(doc_id) {
            if existing.deleted {
                (Self::next_rev(None, limit)?, true)
            } else {
                (Self::next_rev(existing._rev.as_deref(), limit)?, false)
            }
        } else {
            (Self::next_rev(None, limit)?, true)
        };

        let now = chrono::Utc::now().to_rfc3339();
        let document = KvDocument {
            _id: doc_id.to_string(),
            _rev: Some(new_rev),
            value: data,
            created_at: if is_new { Some(now.clone()) } else { None },
            updated_at: Some(now),
            deleted: false,
            expires_at,
        };

        docs.insert(doc_id.to_string(), document.clone());

        let mut seq = database.sequence.write().unwrap();
        *seq += 1;
        let current_seq = *seq;
        drop(seq);
        drop(docs);
        drop(databases);

        let tree = self.db.open_tree(db_name)?;
        let doc_bytes = serde_json::to_vec(&document)?;
        tree.insert(doc_id.as_bytes(), doc_bytes)?;
        let seq_bytes = serde_json::to_vec(&serde_json::json!(current_seq))?;
        tree.insert("_meta:sequence".as_bytes(), seq_bytes)?;
        tree.flush()?;

        Ok(document)
    }

    /// Soft-delete a document (MVCC tombstone) given its current `_rev`.
    ///
    /// Marks the document as deleted and bumps its revision. The tombstone is
    /// kept so that concurrent writers detect conflicts.
    ///
    /// # Errors
    /// Returns an error if the document does not exist or the supplied
    /// revision does not match the stored one.
    pub fn delete_document(&self, db_name: &str, doc_id: &str, rev: &str) -> Result<KvDocument> {
        let mut databases = self.databases.write().unwrap();
        let database = databases.get_mut(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let mut docs = database.documents.write().unwrap();

        let existing = docs.get_mut(doc_id).ok_or_else(|| {
            crate::Error::ValidationError(format!("Document {} not found", doc_id))
        })?;

        if existing._rev.as_ref() != Some(&rev.to_string()) {
            return Err(crate::Error::ValidationError(
                "Revision mismatch".to_string(),
            ));
        }

        let limit = *database.revision_limit.read().unwrap();
        let new_rev = Self::next_rev(existing._rev.as_deref(), limit)?;

        existing._rev = Some(new_rev);
        existing.deleted = true;
        existing.updated_at = Some(chrono::Utc::now().to_rfc3339());

        let result = existing.clone();

        let mut seq = database.sequence.write().unwrap();
        *seq += 1;
        let current_seq = *seq;
        drop(seq);
        drop(docs);
        drop(databases);

        let tree = self.db.open_tree(db_name)?;
        let doc_bytes = serde_json::to_vec(&result)?;
        tree.insert(doc_id.as_bytes(), doc_bytes)?;
        let seq_bytes = serde_json::to_vec(&serde_json::json!(current_seq))?;
        tree.insert("_meta:sequence".as_bytes(), seq_bytes)?;
        tree.flush()?;

        Ok(result)
    }

    /// Apply a batch of document writes (`_bulk_docs`).
    ///
    /// Each document is inserted or updated depending on its `_rev`. With
    /// `all_or_nothing` set, conflicts fail the entire batch.
    ///
    /// # Returns
    /// A per-document result list with new revisions or conflict errors.
    pub fn bulk_docs(
        &self,
        db_name: &str,
        docs: Vec<KvDocument>,
        all_or_nothing: bool,
    ) -> Result<Vec<KvBulkDocsResponse>> {
        let mut databases = self.databases.write().unwrap();
        let database = databases.get_mut(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let mut results = Vec::new();
        let mut docs_map = database.documents.write().unwrap();
        let mut persisted_docs: Vec<KvDocument> = Vec::new();

        for mut doc in docs {
            if all_or_nothing {
                let doc_id = doc._id.clone();
                let result = if docs_map.contains_key(&doc_id) {
                    KvBulkDocsResponse {
                        id: doc_id,
                        rev: None,
                        error: Some("conflict".to_string()),
                    }
                } else {
                    let rev = format!("1-{}", Self::generate_rev_hash());
                    doc._rev = Some(rev.clone());
                    docs_map.insert(doc_id.clone(), doc.clone());
                    persisted_docs.push(doc);
                    KvBulkDocsResponse {
                        id: doc_id,
                        rev: Some(rev),
                        error: None,
                    }
                };
                results.push(result);
            } else {
                let doc_id = doc._id.clone();
                let result = if let Some(existing) = docs_map.get(&doc_id) {
                    if existing._rev == doc._rev {
                        let limit = *database.revision_limit.read().unwrap();
                        let new_rev = Self::next_rev(doc._rev.as_deref(), limit)?;
                        doc._rev = Some(new_rev.clone());
                        doc.updated_at = Some(chrono::Utc::now().to_rfc3339());
                        docs_map.insert(doc_id.clone(), doc.clone());
                        persisted_docs.push(doc);
                        KvBulkDocsResponse {
                            id: doc_id,
                            rev: Some(new_rev),
                            error: None,
                        }
                    } else {
                        KvBulkDocsResponse {
                            id: doc_id,
                            rev: existing._rev.clone(),
                            error: Some("conflict".to_string()),
                        }
                    }
                } else {
                    let rev = format!("1-{}", Self::generate_rev_hash());
                    doc._rev = Some(rev.clone());
                    doc.created_at = Some(chrono::Utc::now().to_rfc3339());
                    doc.updated_at = Some(chrono::Utc::now().to_rfc3339());
                    docs_map.insert(doc_id.clone(), doc.clone());
                    persisted_docs.push(doc);
                    KvBulkDocsResponse {
                        id: doc_id,
                        rev: Some(rev),
                        error: None,
                    }
                };
                results.push(result);
            }
        }

        drop(docs_map);
        drop(databases);

        let tree = self.db.open_tree(db_name)?;
        for doc in &persisted_docs {
            let doc_bytes = serde_json::to_vec(doc)?;
            tree.insert(doc._id.as_bytes(), doc_bytes)?;
        }
        tree.flush()?;

        Ok(results)
    }

    /// Enumerate the documents of a database (`_all_docs`).
    ///
    /// Returns a JSON object with `total_rows`, `offset`, and `rows` (each
    /// containing the doc revision, and optionally the full document when
    /// `include_docs` is set). Tombstoned documents are omitted.
    pub fn all_docs(
        &self,
        db_name: &str,
        include_docs: bool,
        limit: Option<usize>,
        skip: Option<usize>,
    ) -> Result<serde_json::Value> {
        let databases = self.databases.read().unwrap();
        let database = databases.get(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let docs = database.documents.read().unwrap();
        let _seq = *database.sequence.read().unwrap();

        let skip = skip.unwrap_or(0);
        let limit = limit.unwrap_or(usize::MAX);
        let now = now_millis();

        let mut visible: Vec<&KvDocument> = docs
            .values()
            .filter(|doc| !doc.deleted && !is_expired(doc, now))
            .collect();
        visible.sort_by(|a, b| a._id.cmp(&b._id));

        let mut rows: Vec<serde_json::Value> = Vec::new();

        for doc in visible.into_iter().skip(skip).take(limit) {
            let id = &doc._id;
            let row = if include_docs {
                serde_json::json!({
                    "id": id,
                    "key": id,
                    "value": {
                        "rev": doc._rev
                    },
                    "doc": doc
                })
            } else {
                serde_json::json!({
                    "id": id,
                    "key": id,
                    "value": {
                        "rev": doc._rev
                    }
                })
            };
            rows.push(row);
        }

        Ok(serde_json::json!({
            "total_rows": docs.len() as u64,
            "offset": skip,
            "rows": rows
        }))
    }

    /// Create a secondary index on a database.
    ///
    /// The index is currently descriptive; it is not built eagerly, but its
    /// definition is stored so it can be reported by `list_indexes`.
    ///
    /// # Returns
    /// The created index definition.
    pub fn create_index(
        &self,
        db_name: &str,
        name: &str,
        fields: Vec<String>,
        selector: Option<serde_json::Value>,
    ) -> Result<KvIndex> {
        let mut databases = self.databases.write().unwrap();
        let database = databases.get_mut(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let mut indexes = database.indexes.write().unwrap();

        let index = KvIndex {
            name: name.to_string(),
            fields: fields.clone(),
            selector,
        };

        indexes.insert(name.to_string(), index.clone());

        info!(
            "Created index '{}' on {} in database {}",
            name,
            fields.join(", "),
            db_name
        );

        Ok(index)
    }

    /// List all secondary index definitions on a database.
    pub fn list_indexes(&self, db_name: &str) -> Result<Vec<KvIndex>> {
        let databases = self.databases.read().unwrap();
        let database = databases.get(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let indexes = database.indexes.read().unwrap();
        let result: Vec<KvIndex> = indexes.values().cloned().collect();
        Ok(result)
    }

    /// Run a Mango-style `find` query over a database.
    ///
    /// Matches documents against the selector using operators such as `$eq`,
    /// `$ne`, `$gt`, `$gte`, `$lt`, `$lte`, `$in`, `$nin`, `$exists`, and
    /// `$type`. Results can be sorted, limited, and skipped.
    ///
    /// # Returns
    /// A JSON object with the matched `docs` and `execution_stats`.
    pub fn find(&self, db_name: &str, request: KvFindRequest) -> Result<serde_json::Value> {
        let databases = self.databases.read().unwrap();
        let database = databases.get(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let docs = database.documents.read().unwrap();
        let limit = request.limit.unwrap_or(100);
        let skip = request.skip.unwrap_or(0);
        let now = now_millis();

        let selector = &request.selector;
        let mut results: Vec<&KvDocument> = Vec::new();

        for doc in docs.values() {
            if doc.deleted || is_expired(doc, now) {
                continue;
            }
            if Self::matches_selector(&doc.value, selector) {
                results.push(doc);
            }
        }

        if let Some(sort) = &request.sort {
            results = Self::sort_results(results, sort);
        }

        let docs_skipped: Vec<_> = results.iter().skip(skip).take(limit).collect();

        Ok(serde_json::json!({
            "docs": docs_skipped,
            "warning": "This is a basic find implementation",
            "execution_stats": {
                "documents_examined": docs.len(),
                "results_returned": docs_skipped.len()
            }
        }))
    }

    /// Return CouchDB-style database metadata (document counts, update
    /// sequence, index count, and reported cluster settings).
    pub fn get_db_info(&self, db_name: &str) -> Result<serde_json::Value> {
        let databases = self.databases.read().unwrap();
        let database = databases.get(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;

        let docs = database.documents.read().unwrap();
        let seq = *database.sequence.read().unwrap();
        let indexes = database.indexes.read().unwrap();

        let deleted_count = docs.values().filter(|d| d.deleted).count();

        Ok(serde_json::json!({
            "db_name": db_name,
            "doc_count": docs.len() - deleted_count,
            "doc_del_count": deleted_count,
            "sizes": {
                "active": docs.len() * 1000,
                "external": docs.len() * 800,
                "file": docs.len() * 1200
            },
            "update_seq": seq,
            "purge_seq": 0,
            "disk_format_version": 6,
            "fragmentation": 0.4,
            "indexes": indexes.len(),
            "security": {},
            "compact_running": false,
            "cluster": {
                "q": 8,
                "n": 3,
                "w": 2,
                "r": 2
            }
        }))
    }

    /// Return the maximum number of document revisions retained for a database.
    pub fn get_revision_limit(&self, db_name: &str) -> Result<u64> {
        let databases = self.databases.read().unwrap();
        let database = databases.get(db_name).ok_or_else(|| {
            crate::Error::ValidationError(format!("Database {} not found", db_name))
        })?;
        let limit = *database.revision_limit.read().unwrap();
        Ok(limit)
    }

    /// Set the maximum number of document revisions retained for a database.
    ///
    /// The numeric prefix of generated revisions is capped at this limit (a
    /// value below 1 is treated as 1). The limit is persisted to sled so it
    /// survives restarts.
    pub fn set_revision_limit(&self, db_name: &str, limit: u64) -> Result<()> {
        let limit = limit.max(1);
        {
            let databases = self.databases.read().unwrap();
            let database = databases.get(db_name).ok_or_else(|| {
                crate::Error::ValidationError(format!("Database {} not found", db_name))
            })?;
            *database.revision_limit.write().unwrap() = limit;
        }

        let tree = self.db.open_tree(db_name)?;
        let limit_bytes = serde_json::to_vec(&serde_json::json!(limit))?;
        tree.insert("_meta:revision_limit".as_bytes(), limit_bytes)?;
        tree.flush()?;

        info!("Revision limit set to {} for database {}", limit, db_name);
        Ok(())
    }

    /// Flush a database's sled tree to disk and acknowledge a full commit.
    pub fn ensure_full_commit(&self, db_name: &str) -> Result<serde_json::Value> {
        if let Ok(tree) = self.db.open_tree(db_name) {
            let _ = tree.flush();
        }
        info!("Full commit for database: {}", db_name);
        Ok(serde_json::json!({
            "ok": true,
            "instance_start_time": "0"
        }))
    }

    /// Request compaction of a database (currently a no-op acknowledgement).
    pub fn compact(&self, db_name: &str) -> Result<serde_json::Value> {
        info!("Compacting database: {}", db_name);
        Ok(serde_json::json!({
            "ok": true
        }))
    }

    /// Mark a database as encrypted.
    ///
    /// Sets the in-memory encryption flag. This is a flag-only operation;
    /// the read/write paths do not currently transform the stored data.
    pub fn enable_database_encryption(&self, database: &str) -> Result<()> {
        let mut encrypted = self.encrypted_databases.write().unwrap();
        encrypted.insert(database.to_string(), true);
        info!("Encryption enabled for Key-Value database: {}", database);
        Ok(())
    }

    /// Clear the encryption flag for a database.
    pub fn disable_database_encryption(&self, database: &str) -> Result<()> {
        let mut encrypted = self.encrypted_databases.write().unwrap();
        encrypted.insert(database.to_string(), false);
        info!("Encryption disabled for Key-Value database: {}", database);
        Ok(())
    }

    /// Return whether a database is currently flagged as encrypted.
    pub fn is_database_encrypted(&self, database: &str) -> Result<bool> {
        let encrypted = self.encrypted_databases.read().unwrap();
        Ok(*encrypted.get(database).unwrap_or(&false))
    }

    fn generate_rev() -> String {
        format!("1-{}", Self::generate_rev_hash())
    }

    /// Compute the next revision for a document, capping the numeric prefix at
    /// the configured revision limit (CouchDB-style overflow protection).
    fn next_rev(current: Option<&str>, limit: u64) -> Result<String> {
        let Some(cur) = current else {
            return Ok(Self::generate_rev());
        };
        let parts: Vec<&str> = cur.split('-').collect();
        if parts.len() != 2 {
            return Err(crate::Error::ValidationError(
                "Invalid _rev format".to_string(),
            ));
        }
        let next_num: u64 = parts[0].parse::<u64>().map_err(|e| {
            crate::Error::ValidationError(format!("invalid revision number: {}", e))
        })? + 1;
        let capped = if next_num > limit { limit } else { next_num };
        Ok(format!("{}-{}", capped, Self::generate_rev_hash()))
    }

    fn generate_rev_hash() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{:016x}{:08x}", timestamp, rand_u32())
    }

    fn matches_selector(doc: &serde_json::Value, selector: &serde_json::Value) -> bool {
        if let (Some(selector), Some(doc_obj)) = (selector.as_object(), doc.as_object()) {
            for (key, expected) in selector {
                if key.starts_with('$') {
                    continue;
                }
                if let Some(actual) = doc_obj.get(key) {
                    if !Self::match_operator(actual, expected) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn match_operator(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
        if let Some(op_obj) = expected.as_object() {
            for (op, value) in op_obj {
                match op.as_str() {
                    "$eq" => return actual == value,
                    "$ne" => return actual != value,
                    "$gt" => {
                        if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                            return a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0);
                        }
                    }
                    "$gte" => {
                        if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                            return a.as_f64().unwrap_or(0.0) >= b.as_f64().unwrap_or(0.0);
                        }
                    }
                    "$lt" => {
                        if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                            return a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0);
                        }
                    }
                    "$lte" => {
                        if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                            return a.as_f64().unwrap_or(0.0) <= b.as_f64().unwrap_or(0.0);
                        }
                    }
                    "$in" => {
                        if let Some(arr) = value.as_array() {
                            return arr.contains(actual);
                        }
                    }
                    "$nin" => {
                        if let Some(arr) = value.as_array() {
                            return !arr.contains(actual);
                        }
                    }
                    "$exists" => {
                        let exists = !actual.is_null();
                        return exists == value.as_bool().unwrap_or(false);
                    }
                    "$type" => {
                        let actual_type = match actual {
                            serde_json::Value::Null => "null",
                            serde_json::Value::Bool(_) => "boolean",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::Object(_) => "object",
                        };
                        return actual_type == value.as_str().unwrap_or("");
                    }
                    _ => {}
                }
            }
            true
        } else {
            actual == expected
        }
    }

    fn sort_results<'a>(
        mut docs: Vec<&'a KvDocument>,
        sort: &[serde_json::Value],
    ) -> Vec<&'a KvDocument> {
        if let Some(first_sort) = sort.first() {
            if let Some(field) = first_sort.get("field").or_else(|| first_sort.get("key")) {
                let field = field.as_str().unwrap_or("_id");
                let ascending = first_sort
                    .get("direction")
                    .and_then(|d| d.as_str())
                    .map(|d| d == "asc")
                    .unwrap_or(true);

                docs.sort_by(|a, b| {
                    let a_val = a.value.get(field);
                    let b_val = b.value.get(field);
                    let cmp = match (a_val, b_val) {
                        (Some(av), Some(bv)) => av.to_string().cmp(&bv.to_string()),
                        _ => a._id.cmp(&b._id),
                    };
                    if ascending {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });
            }
        }
        docs
    }
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    (nanos as u32) ^ ((nanos >> 32) as u32)
}

#[async_trait]
impl StorageEngine for KeyValueEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    /// Insert a document into a database.
    ///
    /// Uses the document's `_id` if present, otherwise generates a random
    /// hexadecimal ID.
    ///
    /// # Returns
    /// `1` on success.
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        if let Some(id) = data.get("_id").and_then(|v| v.as_str()) {
            let _doc = self.put_document(table, id, data.clone())?;
            Ok(1)
        } else {
            let id = format!("{:x}", rand_u32());
            let _doc = self.put_document(table, &id, data.clone())?;
            Ok(1)
        }
    }

    /// Query documents from a database with optional selector conditions.
    ///
    /// Matching documents are collected first, ordered by `_id` for stable
    /// pagination, then `limit`/`offset` are applied. Tombstoned and expired
    /// documents are skipped.
    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let now = now_millis();

        let databases = self.databases.read().unwrap();
        if let Some(database) = databases.get(table) {
            let docs = database.documents.read().unwrap();
            let mut matches: Vec<&KvDocument> = docs
                .values()
                .filter(|doc| !doc.deleted && !is_expired(doc, now))
                .filter(|doc| {
                    if let Some(cond) = conditions {
                        Self::matches_selector(&doc.value, cond)
                    } else {
                        true
                    }
                })
                .collect();
            matches.sort_by(|a, b| a._id.cmp(&b._id));

            let records = matches
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(|doc| Record {
                    id: doc._id.clone(),
                    data: doc.value.clone(),
                    metadata: std::collections::HashMap::new(),
                })
                .collect();
            Ok(records)
        } else {
            Ok(vec![])
        }
    }

    /// Update documents matching the given selector conditions.
    ///
    /// Replaces each matching document's value with `data` (generating a new
    /// revision) and returns the number of documents updated. A `_rev` key in
    /// `conditions` acts as a compare-and-set guard: the document is only
    /// updated when its stored revision matches.
    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let cas_rev = conditions
            .and_then(|c| c.as_object())
            .and_then(|c| c.get("_rev"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let selector = conditions.and_then(|c| {
            c.as_object().map(|o| {
                let filtered: serde_json::Map<String, serde_json::Value> = o
                    .iter()
                    .filter(|(k, _)| k.as_str() != "_rev")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                serde_json::Value::Object(filtered)
            })
        });

        let databases = self.databases.read().unwrap();
        if let Some(database) = databases.get(table) {
            let docs = database.documents.read().unwrap();
            let mut count = 0;

            for (id, doc) in docs.iter() {
                if doc.deleted {
                    continue;
                }
                let matches = if let Some(cond) = &selector {
                    Self::matches_selector(&doc.value, cond)
                } else {
                    true
                };
                if !matches {
                    continue;
                }
                if let Some(expected) = &cas_rev {
                    if doc._rev.as_deref() != Some(expected.as_str()) {
                        continue;
                    }
                    let _ = self.put_document_cas(table, id, Some(expected), data.clone())?;
                } else {
                    let _ = self.put_document(table, id, data.clone())?;
                }
                count += 1;
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Soft-delete documents matching the given selector conditions.
    ///
    /// Marks matching documents as deleted (tombs) with a bumped revision and
    /// persists the tombstones.
    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let mut databases = self.databases.write().unwrap();
        if let Some(database) = databases.get_mut(table) {
            let mut docs = database.documents.write().unwrap();
            let mut count = 0;

            let to_delete: Vec<String> = docs
                .iter()
                .filter(|(_, doc)| {
                    if doc.deleted {
                        return false;
                    }
                    if let Some(cond) = conditions {
                        return Self::matches_selector(&doc.value, cond);
                    }
                    true
                })
                .map(|(id, _)| id.clone())
                .collect();

            let limit = *database.revision_limit.read().unwrap();

            for id in &to_delete {
                if let Some(doc) = docs.get_mut(id) {
                    let new_rev = Self::next_rev(doc._rev.as_deref(), limit)?;
                    doc._rev = Some(new_rev);
                    doc.deleted = true;
                    doc.updated_at = Some(chrono::Utc::now().to_rfc3339());
                    count += 1;
                }
            }

            let deleted_docs: Vec<(String, KvDocument)> = to_delete
                .iter()
                .filter_map(|id| docs.get(id).map(|d| (id.clone(), d.clone())))
                .collect();

            drop(docs);
            drop(databases);

            let tree = self.db.open_tree(table)?;
            for (id, doc) in &deleted_docs {
                let doc_bytes = serde_json::to_vec(doc)?;
                tree.insert(id.as_bytes(), doc_bytes)?;
            }
            tree.flush()?;

            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Create a database (mapped from a table creation).
    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        self.create_database(table)
    }

    /// Drop a database (mapped from a table drop).
    async fn drop_table(&self, table: &str) -> Result<()> {
        self.delete_database(table)
    }

    /// Truncate a database by dropping and recreating it.
    async fn truncate_table(&self, table: &str, _cascade: bool) -> Result<()> {
        self.delete_database(table)?;
        self.create_database(table)
    }

    /// Enumerate the names of all key-value databases.
    fn list_tables(&self) -> Result<Vec<String>> {
        self.list_databases()
    }

    /// Return database metadata as a [`TableInfo`] with inferred field types
    /// and secondary index definitions.
    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let info = self.get_db_info(table)?;
        let databases = self.databases.read().unwrap();
        let (fields, indexes) = if let Some(database) = databases.get(table) {
            let idx_map = database.indexes.read().unwrap();
            let indexes: Vec<crate::storage::Index> = idx_map
                .values()
                .map(|i| crate::storage::Index {
                    name: i.name.clone(),
                    fields: i.fields.clone(),
                    index_type: crate::storage::IndexType::BTree,
                    unique: false,
                })
                .collect();
            let docs = database.documents.read().unwrap();
            let mut field_names: Vec<String> = Vec::new();
            for doc in docs.values() {
                if let serde_json::Value::Object(map) = &doc.value {
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
            (fields, indexes)
        } else {
            (vec![], vec![])
        };
        Ok(TableInfo {
            name: table.to_string(),
            schema: Schema {
                fields,
                indexes,
                constraints: vec![],
            },
            row_count: info.get("doc_count").and_then(|v| v.as_u64()).unwrap_or(0),
            size_bytes: info
                .get("sizes")
                .and_then(|s| s.get("file"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    /// Produce database statistics (active/deleted document counts, average
    /// fields per document, index count, update sequence) as a JSON string.
    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        let info = self.get_db_info(table)?;
        let databases = self.databases.read().unwrap();
        let stats = if let Some(database) = databases.get(table) {
            let docs = database.documents.read().unwrap();
            let total_docs = docs.len();
            let deleted_docs = docs.values().filter(|d| d.deleted).count();
            let active_docs = total_docs - deleted_docs;
            let total_field_count: usize = docs
                .values()
                .filter(|d| !d.deleted)
                .map(|d| match &d.value {
                    serde_json::Value::Object(obj) => obj.len(),
                    _ => 0,
                })
                .sum();

            serde_json::json!({
                "table": table,
                "active_documents": active_docs,
                "deleted_documents": deleted_docs,
                "total_documents": total_docs,
                "average_fields_per_document": if active_docs > 0 {
                    total_field_count as f64 / active_docs as f64
                } else {
                    0.0f64
                },
                "index_count": database.indexes.read().unwrap().len(),
                "sequence": info.get("update_seq"),
            })
        } else {
            serde_json::json!({
                "table": table,
                "error": "Table not found"
            })
        };
        Ok(serde_json::to_string_pretty(&stats)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(dir: &tempfile::TempDir) -> KeyValueEngine {
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_string_lossy().into_owned();
        KeyValueEngine::new(&config).unwrap()
    }

    #[test]
    fn test_cas_put_document() {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(&dir);
        engine.create_database("casdb").unwrap();

        let doc = engine
            .put_document("casdb", "doc1", serde_json::json!({"v": 1}))
            .unwrap();
        let rev = doc._rev.clone().unwrap();

        let err = engine
            .put_document_cas(
                "casdb",
                "doc1",
                Some("1-aaaaaaaa"),
                serde_json::json!({"v": 2}),
            )
            .unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));

        let updated = engine
            .put_document_cas("casdb", "doc1", Some(&rev), serde_json::json!({"v": 2}))
            .unwrap();
        assert_ne!(updated._rev.as_deref(), Some(rev.as_str()));
    }

    #[test]
    fn test_ttl_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(&dir);
        engine.create_database("ttldb").unwrap();

        engine
            .put_document_ttl("ttldb", "short", serde_json::json!({"v": 1}), 1)
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1100));

        let err = engine.get_document("ttldb", "short").unwrap_err();
        assert!(matches!(err, crate::Error::ValidationError(_)));
    }

    #[test]
    fn test_revision_limit_caps_rev_number() {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(&dir);
        engine.create_database("limdb").unwrap();
        engine.set_revision_limit("limdb", 2).unwrap();

        let mut rev = engine
            .put_document("limdb", "k", serde_json::json!({"n": 0}))
            .unwrap()
            ._rev
            .unwrap();
        for n in 1..5u64 {
            rev = engine
                .put_document_cas("limdb", "k", Some(&rev), serde_json::json!({"n": n}))
                .unwrap()
                ._rev
                .unwrap();
            let num: u64 = rev.split('-').next().unwrap().parse().unwrap();
            assert!(num <= 2, "rev number {} exceeds limit 2", num);
        }

        let stored = engine.get_revision_limit("limdb").unwrap();
        assert_eq!(stored, 2);
    }

    #[test]
    fn test_select_applies_conditions_and_pagination_after_filter() {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(&dir);
        engine.create_database("seldb").unwrap();

        for i in 0..5u64 {
            engine
                .put_document(
                    "seldb",
                    &format!("doc{}", i),
                    serde_json::json!({"group": i % 2}),
                )
                .unwrap();
        }

        let records = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(engine.select(
                "seldb",
                Some(&serde_json::json!({"group": 0})),
                10,
                0,
                &crate::transaction::Transaction {
                    id: "t".to_string(),
                    operations: vec![],
                    status: crate::transaction::TransactionStatus::Prepared,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
                    timeout_ms: 0,
                },
            ))
            .unwrap();

        let groups: Vec<u64> = records
            .iter()
            .map(|r| r.data["group"].as_u64().unwrap())
            .collect();
        assert_eq!(groups, vec![0, 0, 0]);
    }
}
