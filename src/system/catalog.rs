//! # SystemCatalog — Key-Value Metadata Store
//!
//! Stores system metadata (server version, engine registry, system
//! timestamps) organised by category.
//!
//! ## Architecture
//!
//! ```text
//! SystemCatalog
//!   +-> sys_catalog tree (sled)
//!   |     Key: arbitrary string key
//!   |     Value: CatalogEntry { key, value, category, updated_at }
//!   |
//!   +-> Seeded defaults on first init:
//!         server.version, server.status,
//!         engine.registry, system.version, system.created_at
//!   |
//!   +-> Query: get(), set(), delete(),
//!              list_by_category(), list_all(), to_map()
//! ```

use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::sync::Arc;

const CATALOG_TREE: &str = "sys_catalog";

/// Key-value metadata store backed by the `sys_catalog` sled tree, with
/// category-based listing and JSON values.
pub struct SystemCatalog {
    db: Arc<Db>,
}

/// A single catalog entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// Unique entry key.
    pub key: String,
    /// Arbitrary JSON value.
    pub value: serde_json::Value,
    /// Grouping category, e.g. `server`, `engine`, `system`.
    pub category: String,
    /// Time of the last update.
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn serialize_entry(entry: &CatalogEntry) -> crate::Result<Vec<u8>> {
    serde_json::to_vec(entry).map_err(|e| crate::Error::ConfigurationError(e.to_string()))
}

fn deserialize_entry(bytes: &[u8]) -> crate::Result<CatalogEntry> {
    serde_json::from_slice(bytes).map_err(|e| crate::Error::ConfigurationError(e.to_string()))
}

impl SystemCatalog {
    /// Creates a catalog backed by the given sled database.
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    /// Marks the catalog initialised and seeds default entries on first run.
    pub fn init(&self) -> crate::Result<()> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        if tree.get("initialized")?.is_none() {
            tree.insert(
                "initialized",
                serialize_entry(&CatalogEntry {
                    key: "initialized".into(),
                    value: serde_json::Value::Bool(true),
                    category: "system".into(),
                    updated_at: chrono::Utc::now(),
                })?,
            )?;
            self.seed_defaults()?;
        }
        Ok(())
    }

    fn seed_defaults(&self) -> crate::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let entries: Vec<(&str, &str, &str)> = vec![
            ("server.version", env!("CARGO_PKG_VERSION"), "server"),
            ("server.status", "initialized", "server"),
            ("engine.registry", "[]", "engine"),
            ("system.version", "1", "system"),
            ("system.created_at", &now, "system"),
        ];
        let tree = self.db.open_tree(CATALOG_TREE)?;
        for (key, value, category) in entries {
            let entry = CatalogEntry {
                key: key.to_string(),
                value: serde_json::Value::String(value.to_string()),
                category: category.to_string(),
                updated_at: chrono::Utc::now(),
            };
            tree.insert(key.as_bytes(), serialize_entry(&entry)?)?;
        }
        tree.flush()?;
        Ok(())
    }

    /// Reads a single entry by key, or `None` if absent.
    pub fn get(&self, key: &str) -> crate::Result<Option<CatalogEntry>> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        if let Some(bytes) = tree.get(key.as_bytes())? {
            Ok(Some(deserialize_entry(&bytes)?))
        } else {
            Ok(None)
        }
    }

    /// Upserts an entry under `key` with the given category and value.
    pub fn set(&self, key: &str, value: serde_json::Value, category: &str) -> crate::Result<()> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        let entry = CatalogEntry {
            key: key.to_string(),
            value,
            category: category.to_string(),
            updated_at: chrono::Utc::now(),
        };
        tree.insert(key.as_bytes(), serialize_entry(&entry)?)?;
        tree.flush()?;
        Ok(())
    }

    /// Lists all entries belonging to a category.
    pub fn list_by_category(&self, category: &str) -> crate::Result<Vec<CatalogEntry>> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        let mut entries = Vec::new();
        for result in &tree {
            let (_, value) = result?;
            if let Ok(entry) = deserialize_entry(&value) {
                if entry.category == category {
                    entries.push(entry);
                }
            }
        }
        Ok(entries)
    }

    /// Lists every catalog entry.
    pub fn list_all(&self) -> crate::Result<Vec<CatalogEntry>> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        let mut entries = Vec::new();
        for result in &tree {
            let (_, value) = result?;
            if let Ok(entry) = deserialize_entry(&value) {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// Removes an entry by key.
    pub fn delete(&self, key: &str) -> crate::Result<()> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        tree.remove(key.as_bytes())?;
        tree.flush()?;
        Ok(())
    }

    /// Flattens the catalog into a `key -> value` map.
    pub fn to_map(&self) -> crate::Result<HashMap<String, serde_json::Value>> {
        let tree = self.db.open_tree(CATALOG_TREE)?;
        let mut map = HashMap::new();
        for result in &tree {
            let (_, value) = result?;
            if let Ok(entry) = deserialize_entry(&value) {
                map.insert(entry.key.clone(), entry.value);
            }
        }
        Ok(map)
    }
}
