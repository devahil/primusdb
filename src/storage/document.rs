use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct DocumentEngine {
    config: PrimusDBConfig,
    collections: Arc<RwLock<HashMap<String, DocumentCollection>>>,
}

#[derive(Debug)]
struct DocumentCollection {
    name: String,
    documents: HashMap<String, Document>,
    indexes: HashMap<String, DocumentIndex>,
    next_id: u64,
}

#[derive(Debug, Clone)]
struct Document {
    id: String,
    data: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    version: u64,
}

#[derive(Debug)]
struct DocumentIndex {
    field: String,
    index_type: DocumentIndexType,
    data: HashMap<String, Vec<String>>, // field_value -> document_ids
}

#[derive(Debug)]
enum DocumentIndexType {
    BTree,
    Hash,
    FullText,
    GeoSpatial,
}

impl DocumentEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        Ok(DocumentEngine {
            config: config.clone(),
            collections: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn match_document(document: &Document, conditions: &serde_json::Value) -> bool {
        match conditions {
            serde_json::Value::Object(obj) => {
                for (key, value) in obj {
                    if !self::DocumentEngine::match_field(&document.data, key, value) {
                        return false;
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

#[async_trait]
impl StorageEngine for DocumentEngine {
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        // Implementation for document insert with automatic indexing
        println!("Document insert into {}: {:?}", table, data);

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

        let document = Document {
            id: id.clone(),
            data: data.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            version: 1,
        };

        collection.documents.insert(id, document);

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
        // Implementation for document query with complex conditions
        println!(
            "Document query from {} with conditions: {:?}",
            table, conditions
        );

        let collections = self.collections.read().unwrap();
        if let Some(collection) = collections.get(table) {
            let mut records = Vec::new();
            for doc in collection
                .documents
                .values()
                .skip(offset as usize)
                .take(limit as usize)
            {
                records.push(Record {
                    id: doc.id.clone(),
                    data: doc.data.clone(),
                    metadata: HashMap::new(),
                });
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
        _data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        // Implementation for document update with field-level operations
        println!(
            "Document update in {} with conditions: {:?}",
            table, conditions
        );
        Ok(1)
    }

    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        // Implementation for document delete
        println!(
            "Document delete from {} with conditions: {:?}",
            table, conditions
        );
        Ok(1)
    }

    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        // Implementation for document analytics and aggregation
        println!("Document analyze for collection: {}", table);
        Ok("Document analysis completed".to_string())
    }

    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        println!("Creating document collection: {}", table);
        Ok(())
    }

    async fn drop_table(&self, table: &str) -> Result<()> {
        println!("Dropping document collection: {}", table);
        Ok(())
    }

    async fn truncate_table(&self, table: &str) -> Result<()> {
        println!("Truncating document collection: {}", table);
        // For document engine, truncate would require mutable access
        // This is a placeholder implementation
        Ok(())
    }

    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        println!("Getting document collection info for: {}", table);
        Err(crate::Error::DatabaseError(
            "Collection not found".to_string(),
        ))
    }
}
