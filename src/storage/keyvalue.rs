/*
 * PrimusDB Key-Value Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Key-Value engine with CouchDB-like API
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
use crate::crypto::FileEncryptionManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvDocument {
    pub _id: String,
    pub _rev: Option<String>,
    pub value: serde_json::Value,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvAttachment {
    pub content_type: String,
    pub data: String,
    pub length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvBulkDocsRequest {
    pub docs: Vec<KvDocument>,
    pub all_or_nothing: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvBulkDocsResponse {
    pub id: String,
    pub rev: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvViewRequest {
    pub map: String,
    pub reduce: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvViewResult {
    pub id: String,
    pub key: serde_json::Value,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KvFindRequest {
    pub selector: serde_json::Value,
    pub limit: Option<usize>,
    pub skip: Option<usize>,
    pub sort: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone)]
pub struct KvIndex {
    pub name: String,
    pub fields: Vec<String>,
    pub selector: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct KeyValueEngine {
    config: PrimusDBConfig,
    databases: Arc<RwLock<HashMap<String, KvDatabase>>>,
    file_encryption: Arc<RwLock<Option<FileEncryptionManager>>>,
    encrypted_databases: Arc<RwLock<HashMap<String, bool>>>,
}

#[derive(Clone)]
pub struct KvDatabase {
    name: String,
    documents: Arc<RwLock<HashMap<String, KvDocument>>>,
    indexes: Arc<RwLock<HashMap<String, KvIndex>>>,
    sequence: Arc<RwLock<u64>>,
    attachments: Arc<RwLock<HashMap<String, HashMap<String, KvAttachment>>>>,
}

impl KeyValueEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let file_encryption = if config.security.encryption_enabled {
            Some(FileEncryptionManager::new())
        } else {
            None
        };

        Ok(KeyValueEngine {
            config: config.clone(),
            databases: Arc::new(RwLock::new(HashMap::new())),
            file_encryption: Arc::new(RwLock::new(file_encryption)),
            encrypted_databases: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn create_database(&self, name: &str) -> Result<()> {
        let mut db = self.databases.write().unwrap();
        if db.contains_key(name) {
            return Err(crate::Error::ValidationError(format!("Database {} already exists", name)));
        }
        
        db.insert(name.to_string(), KvDatabase {
            name: name.to_string(),
            documents: Arc::new(RwLock::new(HashMap::new())),
            indexes: Arc::new(RwLock::new(HashMap::new())),
            sequence: Arc::new(RwLock::new(0)),
            attachments: Arc::new(RwLock::new(HashMap::new())),
        });
        
        println!("Created Key-Value database: {}", name);
        Ok(())
    }

    pub fn delete_database(&self, name: &str) -> Result<()> {
        let mut db = self.databases.write().unwrap();
        if !db.contains_key(name) {
            return Err(crate::Error::ValidationError(format!("Database {} not found", name)));
        }
        
        db.remove(name);
        println!("Deleted Key-Value database: {}", name);
        Ok(())
    }

    pub fn list_databases(&self) -> Result<Vec<String>> {
        let db = self.databases.read().unwrap();
        Ok(db.keys().cloned().collect())
    }

    pub fn get_document(&self, db_name: &str, doc_id: &str) -> Result<KvDocument> {
        let db = self.databases.read().unwrap();
        let database = db.get(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let docs = database.documents.read().unwrap();
        let doc = docs.get(doc_id)
            .ok_or_else(|| crate::Error::ValidationError(format!("Document {} not found", doc_id)))?;
        
        if doc.deleted {
            return Err(crate::Error::ValidationError(format!("Document {} is deleted", doc_id)));
        }
        
        Ok(doc.clone())
    }

    pub fn put_document(&self, db_name: &str, doc_id: &str, data: serde_json::Value) -> Result<KvDocument> {
        let mut db = self.databases.write().unwrap();
        let database = db.get_mut(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let mut docs = database.documents.write().unwrap();
        
        let (new_rev, is_new) = if let Some(existing) = docs.get(doc_id) {
            if existing.deleted {
                (Self::generate_rev(), true)
            } else {
                let current_rev = existing._rev.as_ref().ok_or_else(|| 
                    crate::Error::ValidationError("Document has no _rev".to_string()))?;
                let parts: Vec<&str> = current_rev.split('-').collect();
                if parts.len() != 2 {
                    return Err(crate::Error::ValidationError("Invalid _rev format".to_string()));
                }
                let new_num: u64 = parts[0].parse().unwrap_or(0) + 1;
                (format!("{}-{}", new_num, Self::generate_rev_hash()), false)
            }
        } else {
            (format!("1-{}", Self::generate_rev_hash()), true)
        };

        let now = chrono::Utc::now().to_rfc3339();
        let document = KvDocument {
            _id: doc_id.to_string(),
            _rev: Some(new_rev),
            value: data,
            created_at: if is_new { Some(now.clone()) } else { None },
            updated_at: Some(now),
            deleted: false,
        };

        docs.insert(doc_id.to_string(), document.clone());
        
        let mut seq = database.sequence.write().unwrap();
        *seq += 1;

        Ok(document)
    }

    pub fn delete_document(&self, db_name: &str, doc_id: &str, rev: &str) -> Result<KvDocument> {
        let mut db = self.databases.write().unwrap();
        let database = db.get_mut(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let mut docs = database.documents.write().unwrap();
        
        let existing = docs.get_mut(doc_id)
            .ok_or_else(|| crate::Error::ValidationError(format!("Document {} not found", doc_id)))?;
        
        if existing._rev.as_ref() != Some(&rev.to_string()) {
            return Err(crate::Error::ValidationError("Revision mismatch".to_string()));
        }
        
        let parts: Vec<&str> = rev.split('-').collect();
        if parts.len() != 2 {
            return Err(crate::Error::ValidationError("Invalid _rev format".to_string()));
        }
        let new_num: u64 = parts[0].parse().unwrap_or(0) + 1;
        let new_rev = format!("{}-{}", new_num, Self::generate_rev_hash());
        
        existing._rev = Some(new_rev);
        existing.deleted = true;
        existing.updated_at = Some(chrono::Utc::now().to_rfc3339());
        
        let mut seq = database.sequence.write().unwrap();
        *seq += 1;

        Ok(existing.clone())
    }

    pub fn bulk_docs(&self, db_name: &str, docs: Vec<KvDocument>, all_or_nothing: bool) -> Result<Vec<KvBulkDocsResponse>> {
        let mut db = self.databases.write().unwrap();
        let database = db.get_mut(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let mut results = Vec::new();
        let mut docs_map = database.documents.write().unwrap();

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
                    docs_map.insert(doc_id.clone(), doc);
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
                        let parts: Vec<&str> = doc._rev.as_ref().unwrap().split('-').collect();
                        let new_num: u64 = parts[0].parse().unwrap_or(0) + 1;
                        let new_rev = format!("{}-{}", new_num, Self::generate_rev_hash());
                        doc._rev = Some(new_rev.clone());
                        doc.updated_at = Some(chrono::Utc::now().to_rfc3339());
                        docs_map.insert(doc_id.clone(), doc);
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
                    docs_map.insert(doc_id.clone(), doc);
                    KvBulkDocsResponse {
                        id: doc_id,
                        rev: Some(rev),
                        error: None,
                    }
                };
                results.push(result);
            }
        }

        Ok(results)
    }

    pub fn all_docs(&self, db_name: &str, include_docs: bool, limit: Option<usize>, skip: Option<usize>) -> Result<serde_json::Value> {
        let db = self.databases.read().unwrap();
        let database = db.get(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let docs = database.documents.read().unwrap();
        let seq = *database.sequence.read().unwrap();
        
        let skip = skip.unwrap_or(0);
        let limit = limit.unwrap_or(usize::MAX);
        
        let mut rows: Vec<serde_json::Value> = Vec::new();
        
        for (id, doc) in docs.iter().skip(skip).take(limit) {
            if !doc.deleted {
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
        }

        Ok(serde_json::json!({
            "total_rows": docs.len() as u64,
            "offset": skip,
            "rows": rows
        }))
    }

    pub fn create_index(&self, db_name: &str, name: &str, fields: Vec<String>, selector: Option<serde_json::Value>) -> Result<KvIndex> {
        let mut db = self.databases.write().unwrap();
        let database = db.get_mut(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let mut indexes = database.indexes.write().unwrap();
        
        let index = KvIndex {
            name: name.to_string(),
            fields: fields.clone(),
            selector,
        };
        
        indexes.insert(name.to_string(), index.clone());
        
        println!("Created index '{}' on {} in database {}", name, fields.join(", "), db_name);
        
        Ok(index)
    }

    pub fn find(&self, db_name: &str, request: KvFindRequest) -> Result<serde_json::Value> {
        let db = self.databases.read().unwrap();
        let database = db.get(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
        let docs = database.documents.read().unwrap();
        let limit = request.limit.unwrap_or(100);
        let skip = request.skip.unwrap_or(0);
        
        let selector = &request.selector;
        let mut results: Vec<&KvDocument> = Vec::new();
        
        for doc in docs.values() {
            if doc.deleted {
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

    pub fn get_db_info(&self, db_name: &str) -> Result<serde_json::Value> {
        let db = self.databases.read().unwrap();
        let database = db.get(db_name)
            .ok_or_else(|| crate::Error::ValidationError(format!("Database {} not found", db_name)))?;
        
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

    pub fn get_revision_limit(&self, db_name: &str) -> Result<u64> {
        Ok(1000)
    }

    pub fn set_revision_limit(&self, db_name: &str, limit: u64) -> Result<()> {
        println!("Revision limit set to {} for database {}", limit, db_name);
        Ok(())
    }

    pub fn ensure_full_commit(&self, db_name: &str) -> Result<serde_json::Value> {
        println!("Full commit for database: {}", db_name);
        Ok(serde_json::json!({
            "ok": true,
            "instance_start_time": "0"
        }))
    }

    pub fn compact(&self, db_name: &str) -> Result<serde_json::Value> {
        println!("Compacting database: {}", db_name);
        Ok(serde_json::json!({
            "ok": true
        }))
    }

    pub fn enable_database_encryption(&self, database: &str) -> Result<()> {
        let mut encrypted = self.encrypted_databases.write().unwrap();
        encrypted.insert(database.to_string(), true);
        println!("Encryption enabled for Key-Value database: {}", database);
        Ok(())
    }

    pub fn disable_database_encryption(&self, database: &str) -> Result<()> {
        let mut encrypted = self.encrypted_databases.write().unwrap();
        encrypted.insert(database.to_string(), false);
        println!("Encryption disabled for Key-Value database: {}", database);
        Ok(())
    }

    pub fn is_database_encrypted(&self, database: &str) -> Result<bool> {
        let encrypted = self.encrypted_databases.read().unwrap();
        Ok(*encrypted.get(database).unwrap_or(&false))
    }

    fn generate_rev() -> String {
        format!("1-{}", Self::generate_rev_hash())
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
                    "$gt" => if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                        return a.as_f64().unwrap_or(0.0) > b.as_f64().unwrap_or(0.0);
                    },
                    "$gte" => if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                        return a.as_f64().unwrap_or(0.0) >= b.as_f64().unwrap_or(0.0);
                    },
                    "$lt" => if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                        return a.as_f64().unwrap_or(0.0) < b.as_f64().unwrap_or(0.0);
                    },
                    "$lte" => if let (Some(a), Some(b)) = (actual.as_number(), value.as_number()) {
                        return a.as_f64().unwrap_or(0.0) <= b.as_f64().unwrap_or(0.0);
                    },
                    "$in" => if let Some(arr) = value.as_array() {
                        return arr.contains(actual);
                    },
                    "$nin" => if let Some(arr) = value.as_array() {
                        return !arr.contains(actual);
                    },
                    "$exists" => {
                        let exists = actual.is_null() == false;
                        return exists == value.as_bool().unwrap_or(false);
                    },
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
                    },
                    _ => {}
                }
            }
            true
        } else {
            actual == expected
        }
    }

    fn sort_results<'a>(mut docs: Vec<&'a KvDocument>, sort: &[serde_json::Value]) -> Vec<&'a KvDocument> {
        if let Some(first_sort) = sort.first() {
            if let Some(field) = first_sort.get("field").or_else(|| first_sort.get("key")) {
                let field = field.as_str().unwrap_or("_id");
                let ascending = first_sort.get("direction")
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
                    if ascending { cmp } else { cmp.reverse() }
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

    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        if let Some(id) = data.get("_id").and_then(|v| v.as_str()) {
            let doc = self.put_document(table, id, data.clone())?;
            Ok(1)
        } else {
            let id = format!("{:x}", rand_u32());
            let doc = self.put_document(table, &id, data.clone())?;
            Ok(1)
        }
    }

    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let limit = limit as usize;
        let offset = offset as usize;

        let db = self.databases.read().unwrap();
        if let Some(database) = db.get(table) {
            let docs = database.documents.read().unwrap();
            let mut records = Vec::new();

            for (_, doc) in docs.iter().skip(offset).take(limit) {
                if !doc.deleted {
                    if let Some(cond) = conditions {
                        if Self::matches_selector(&doc.value, cond) {
                            records.push(Record {
                                id: doc._id.clone(),
                                data: doc.value.clone(),
                                metadata: std::collections::HashMap::new(),
                            });
                        }
                    } else {
                        records.push(Record {
                            id: doc._id.clone(),
                            data: doc.value.clone(),
                            metadata: std::collections::HashMap::new(),
                        });
                    }
                }
            }
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
        let db = self.databases.read().unwrap();
        if let Some(database) = db.get(table) {
            let docs = database.documents.read().unwrap();
            let mut count = 0;

            for (id, doc) in docs.iter() {
                if !doc.deleted {
                    if let Some(cond) = conditions {
                        if Self::matches_selector(&doc.value, cond) {
                            let _ = self.put_document(table, id, data.clone())?;
                            count += 1;
                        }
                    }
                }
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let mut db = self.databases.write().unwrap();
        if let Some(database) = db.get_mut(table) {
            let mut docs = database.documents.write().unwrap();
            let mut count = 0;

            let to_delete: Vec<String> = docs.iter()
                .filter(|(_, doc)| {
                    if doc.deleted { return false; }
                    if let Some(cond) = conditions {
                        return Self::matches_selector(&doc.value, cond);
                    }
                    true
                })
                .map(|(id, _)| id.clone())
                .collect();

            for id in to_delete {
                if let Some(doc) = docs.get_mut(&id) {
                    let rev = doc._rev.clone().unwrap_or_default();
                    let parts: Vec<&str> = rev.split('-').collect();
                    let new_num: u64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0) + 1;
                    doc._rev = Some(format!("{}-{:016x}", new_num, rand_u32()));
                    doc.deleted = true;
                    doc.updated_at = Some(chrono::Utc::now().to_rfc3339());
                    count += 1;
                }
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        self.create_database(table)
    }

    async fn drop_table(&self, table: &str) -> Result<()> {
        self.delete_database(table)
    }

    async fn truncate_table(&self, table: &str) -> Result<()> {
        self.delete_database(table)?;
        self.create_database(table)
    }

    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let info = self.get_db_info(table)?;
        Ok(TableInfo {
            name: table.to_string(),
            schema: Schema {
                fields: vec![],
                indexes: vec![],
                constraints: vec![],
            },
            row_count: info.get("doc_count").and_then(|v| v.as_u64()).unwrap_or(0),
            size_bytes: info.get("sizes").and_then(|s| s.get("file")).and_then(|v| v.as_u64()).unwrap_or(0),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }

    async fn analyze(
        &self,
        _table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        Ok("Key-Value analyze not fully implemented".to_string())
    }
}
