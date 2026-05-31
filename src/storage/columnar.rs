/*
 * PrimusDB Columnar Storage Engine
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 */

use crate::{
    storage::{Schema, StorageEngine, TableInfo},
    PrimusDBConfig, Record, Result,
};
use async_trait::async_trait;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use tracing::info;

pub struct ColumnarEngine {
    #[allow(dead_code)]
    config: PrimusDBConfig,
    db: sled::Db,
}

impl ColumnarEngine {
    pub fn new(config: &PrimusDBConfig) -> Result<Self> {
        let db_path = format!("{}/columnar", config.storage.data_dir);
        let db = sled::open(&db_path)?;

        Ok(ColumnarEngine {
            config: config.clone(),
            db,
        })
    }

    #[allow(dead_code)]
    fn extract_fields(data: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
        match data.as_object() {
            Some(obj) => obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            None => vec![],
        }
    }

    fn matches_filter(data: &serde_json::Value, conditions: &serde_json::Value) -> bool {
        if conditions.is_null() || conditions.as_object().map_or(true, |o| o.is_empty()) {
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
impl StorageEngine for ColumnarEngine {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let table_owned = table.to_string();
        let data = data.clone();

        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<u64> {
                let meta_tree = db.open_tree(format!("meta:{}", table_owned))?;
                let id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64;
                let id_key = id.to_be_bytes();

                // Store row data
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let row_value = serde_json::to_vec(&data)?;
                row_tree.insert(id_key, row_value)?;

                // Store column-wise indices for queryable fields
                if let Some(obj) = data.as_object() {
                    for (field_name, field_val) in obj {
                        let col_tree = db.open_tree(format!("col:{}:{}", table_owned, field_name))?;
                        let col_key = {
                            let mut k = Vec::with_capacity(8 + 1);
                            k.extend_from_slice(&id_key);
                            k
                        };
                        let col_value = serde_json::to_vec(field_val)?;
                        col_tree.insert(col_key, col_value)?;
                    }
                }

                meta_tree.insert(b"row_count", &(meta_tree.len() as u64).to_be_bytes())?;
                row_tree.flush()?;
                meta_tree.flush()?;

                Ok(id)
            }
        })
        .await??;

        Ok(result)
    }

    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>> {
        let table_owned = table.to_string();
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);

        let result: Vec<Record> = tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<Vec<Record>> {
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let limit = if limit == 0 { u64::MAX } else { limit };
                let mut records = Vec::new();

                // Collect candidate keys with optional column pruning
                let cond_fields: HashSet<String> = match &conditions {
                    serde_json::Value::Object(obj) => obj.keys().cloned().collect(),
                    _ => HashSet::new(),
                };

                // Determine candidate row IDs using column indices
                let candidate_ids: Option<HashSet<u64>> = if !cond_fields.is_empty() {
                    let mut sets: Vec<HashSet<u64>> = Vec::new();
                    for field in &cond_fields {
                        if let Some(cond_val) = conditions.get(field) {
                            if let Ok(col_tree) = db.open_tree(format!("col:{}:{}", table_owned, field)) {
                                let mut ids = HashSet::new();
                                for item in col_tree.iter() {
                                    let (key, value) = item?;
                                    let row_id = u64::from_be_bytes(key.as_ref()[..8].try_into().unwrap());
                                    let stored_val: serde_json::Value = serde_json::from_slice(&value)?;
                                    if &stored_val == cond_val {
                                        ids.insert(row_id);
                                    }
                                }
                                sets.push(ids);
                            }
                        }
                    }
                    if !sets.is_empty() {
                        let mut intersection = sets.remove(0);
                        for s in sets {
                            intersection.retain(|id| s.contains(id));
                        }
                        Some(intersection)
                    } else {
                        None
                    }
                } else {
                    None
                };

                let mut skipped = 0u64;
                for item in row_tree.iter() {
                    let (key, value) = item?;
                    let row_id = u64::from_be_bytes(key.as_ref().try_into().unwrap());

                    // Skip if not a candidate
                    if let Some(ref candidates) = candidate_ids {
                        if !candidates.contains(&row_id) {
                            continue;
                        }
                    }

                    let data: serde_json::Value = serde_json::from_slice(&value)?;

                    // Apply filter conditions
                    if !Self::matches_filter(&data, &conditions) {
                        continue;
                    }

                    if skipped < offset {
                        skipped += 1;
                        continue;
                    }

                    records.push(Record {
                        id: row_id.to_string(),
                        data,
                        metadata: HashMap::new(),
                    });

                    if records.len() as u64 >= limit {
                        break;
                    }
                }

                Ok(records)
            }
        })
        .await??;

        Ok(result)
    }

    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let table_owned = table.to_string();
        let data = data.clone();
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);

        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<u64> {
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let mut updated = 0u64;

                for item in row_tree.iter() {
                    let (key, value) = item?;
                    let stored: serde_json::Value = serde_json::from_slice(&value)?;

                    if !Self::matches_filter(&stored, &conditions) {
                        continue;
                    }

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
                            row_tree.insert(key.clone(), new_value)?;

                            // Update column indices
                            if let Some(obj) = data.as_object() {
                                for (field_name, field_val) in obj {
                                    if let Ok(col_tree) = db.open_tree(format!("col:{}:{}", table_owned, field_name)) {
                                        let col_value = serde_json::to_vec(field_val)?;
                                        col_tree.insert(key.to_vec(), col_value)?;
                                    }
                                }
                            }

                    updated += 1;
                }

                if updated > 0 {
                    row_tree.flush()?;
                }

                info!("Columnar update in {}: {} records updated", table_owned, updated);
                Ok(updated)
            }
        })
        .await??;

        Ok(result)
    }

    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<u64> {
        let table_owned = table.to_string();
        let conditions = conditions.cloned().unwrap_or(serde_json::Value::Null);

        let result: u64 = tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<u64> {
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let mut deleted = 0u64;
                let mut to_remove = Vec::new();
                let mut fields_to_clean: Vec<String> = Vec::new();

                // Detect affected fields from first matching record
                for item in row_tree.iter() {
                    let (key, value) = item?;
                    let stored: serde_json::Value = serde_json::from_slice(&value)?;

                    if !Self::matches_filter(&stored, &conditions) {
                        continue;
                    }

                    if fields_to_clean.is_empty() {
                        if let Some(obj) = stored.as_object() {
                            fields_to_clean = obj.keys().cloned().collect();
                        }
                    }

                    to_remove.push(key.to_vec());
                    deleted += 1;
                }

                for key in &to_remove {
                    row_tree.remove(key)?;
                    for field in &fields_to_clean {
                        if let Ok(col_tree) = db.open_tree(format!("col:{}:{}", table_owned, field)) {
                            let _ = col_tree.remove(key);
                        }
                    }
                }

                if deleted > 0 {
                    row_tree.flush()?;
                }

                info!("Columnar delete from {}: {} records deleted", table_owned, deleted);
                Ok(deleted)
            }
        })
        .await??;

        Ok(result)
    }

    async fn analyze(
        &self,
        table: &str,
        _conditions: Option<&serde_json::Value>,
        _transaction: &crate::transaction::Transaction,
    ) -> Result<String> {
        let table_owned = table.to_string();
        let result: serde_json::Value = tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<serde_json::Value> {
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let mut total_records = 0u64;
                let mut field_counts: HashMap<String, u64> = HashMap::new();
                let mut field_types: HashMap<String, String> = HashMap::new();
                let mut column_stats: HashMap<String, serde_json::Value> = HashMap::new();

                for item in row_tree.iter() {
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

                            // Compute per-column stats for numeric fields
                            if let Some(num) = val.as_f64() {
                                let entry = column_stats.entry(key.clone())
                                    .or_insert_with(|| serde_json::json!({"min": num, "max": num, "sum": 0.0, "count": 0}));
                                if let Some(min) = entry.get("min").and_then(|v| v.as_f64()) {
                                    entry["min"] = serde_json::json!(min.min(num));
                                }
                                if let Some(max) = entry.get("max").and_then(|v| v.as_f64()) {
                                    entry["max"] = serde_json::json!(max.max(num));
                                }
                                if let Some(sum) = entry.get("sum").and_then(|v| v.as_f64()) {
                                    entry["sum"] = serde_json::json!(sum + num);
                                }
                                if let Some(count) = entry.get("count").and_then(|v| v.as_u64()) {
                                    entry["count"] = serde_json::json!(count + 1);
                                }
                            }
                        }
                    }
                }

                // Compute averages
                for (_field, stats) in column_stats.iter_mut() {
                    if let (Some(sum), Some(count)) = (
                        stats.get("sum").and_then(|v| v.as_f64()),
                        stats.get("count").and_then(|v| v.as_u64()),
                    ) {
                        if count > 0 {
                            stats["avg"] = serde_json::json!(sum / count as f64);
                        }
                    }
                    stats.as_object_mut().map(|o| o.remove("count"));
                }

                Ok(serde_json::json!({
                    "table": table_owned,
                    "total_records": total_records,
                    "fields": field_counts,
                    "field_types": field_types,
                    "column_stats": column_stats,
                    "engine": "columnar"
                }))
            }
        })
        .await??;

        Ok(serde_json::to_string(&result)?)
    }

    async fn create_table(&self, table: &str, _schema: &Schema) -> Result<()> {
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            let table_owned = table.to_string();
            move || -> crate::Result<()> {
                db.open_tree(format!("meta:{}", table_owned))?;
                db.open_tree(format!("row:{}", table_owned))?;
                Ok(())
            }
        })
        .await??;

        info!("Columnar table created: {}", table);
        Ok(())
    }

    async fn drop_table(&self, table: &str) -> Result<()> {
        let table_owned = table.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<()> {
                db.drop_tree(format!("meta:{}", table_owned))?;
                db.drop_tree(format!("row:{}", table_owned))?;
                // Drop all column trees
                let prefix = format!("col:{}:", table_owned);
                let trees: Vec<String> = db.tree_names().iter()
                    .filter_map(|t| {
                        let s = String::from_utf8_lossy(t).to_string();
                        if s.starts_with(&prefix) { Some(s) } else { None }
                    })
                    .collect();
                for t in trees {
                    let _ = db.drop_tree(t);
                }
                Ok(())
            }
        })
        .await??;

        info!("Columnar table dropped: {}", table);
        Ok(())
    }

    async fn truncate_table(&self, table: &str, _cascade: bool) -> Result<()> {
        let table_owned = table.to_string();
        tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<()> {
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let mut iter = row_tree.iter();
                while let Some(Ok((key, _))) = iter.next() {
                    row_tree.remove(key)?;
                }
                row_tree.flush()?;

                let prefix = format!("col:{}:", table_owned);
                let trees: Vec<String> = db.tree_names().iter()
                    .filter_map(|t| {
                        let s = String::from_utf8_lossy(t).to_string();
                        if s.starts_with(&prefix) { Some(s) } else { None }
                    })
                    .collect();
                for t in trees {
                    let col_tree = db.open_tree(t)?;
                    let mut citer = col_tree.iter();
                    while let Some(Ok((key, _))) = citer.next() {
                        col_tree.remove(key)?;
                    }
                }

                Ok(())
            }
        })
        .await??;

        info!("Columnar table truncated: {}", table);
        Ok(())
    }

    async fn table_info(&self, table: &str) -> Result<TableInfo> {
        let table_owned = table.to_string();
        let (count, size): (usize, u64) = tokio::task::spawn_blocking({
            let db = self.db.clone();
            move || -> crate::Result<(usize, u64)> {
                let row_tree = db.open_tree(format!("row:{}", table_owned))?;
                let count = row_tree.len();
                let size = row_tree.iter().filter_map(|item| {
                    item.ok().map(|(_, v)| v.len() as u64)
                }).sum();
                Ok((count, size))
            }
        })
        .await??;

        Ok(TableInfo {
            name: table.to_string(),
            schema: Schema {
                fields: vec![],
                indexes: vec![],
                constraints: vec![],
            },
            row_count: count as u64,
            size_bytes: size,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::Transaction;

    fn test_tx() -> Transaction {
        Transaction {
            id: "test".to_string(),
            operations: vec![],
            created_at: chrono::Utc::now(),
            status: crate::transaction::TransactionStatus::Active,
            updated_at: chrono::Utc::now(),
            isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
            timeout_ms: 30000,
            ..Default::default()
        }
    }

    #[test]
    fn test_extract_fields() {
        let data = serde_json::json!({"a": 1, "b": "hello"});
        let fields = ColumnarEngine::extract_fields(&data);
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn test_matches_filter() {
        let data = serde_json::json!({"name": "alice", "age": 30});
        assert!(ColumnarEngine::matches_filter(&data, &serde_json::json!({"name": "alice"})));
        assert!(!ColumnarEngine::matches_filter(&data, &serde_json::json!({"name": "bob"})));
    }

    #[tokio::test]
    async fn test_columnar_insert_and_select() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = ColumnarEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_col", &schema).await.unwrap();

        let tx = test_tx();
        let data = serde_json::json!({"name": "alice", "age": 30, "city": "NYC"});
        let id = engine.insert("test_col", &data, &tx).await.unwrap();
        assert!(id > 0);

        let records = engine.select("test_col", None, 10, 0, &tx).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].data["name"], "alice");

        engine.drop_table("test_col").await.unwrap();
    }

    #[tokio::test]
    async fn test_columnar_filtered_select() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = ColumnarEngine::new(&config).unwrap();
        let schema = Schema { fields: vec![], indexes: vec![], constraints: vec![] };
        engine.create_table("test_filt", &schema).await.unwrap();

        let tx = test_tx();
        for i in 0..10 {
            let data = serde_json::json!({"value": i, "parity": i % 2});
            engine.insert("test_filt", &data, &tx).await.unwrap();
        }

        let cond = serde_json::json!({"parity": 0});
        let records = engine.select("test_filt", Some(&cond), 10, 0, &tx).await.unwrap();
        assert_eq!(records.len(), 5);

        engine.drop_table("test_filt").await.unwrap();
    }
}
