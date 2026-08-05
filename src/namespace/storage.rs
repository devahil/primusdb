//! # NamespacedStorageEngine — Namespace-Scoped Storage Adapter
//!
//! Wraps any [`StorageEngine`] and transparently rewrites every table name to
//! its namespace-scoped physical name before delegating, so a single
//! underlying engine can serve many isolated namespaces.
//!
//! ```text
//! NamespacedStorageEngine { inner, namespace_path }
//!   |   logical "users"
//!   v
//! physical_name = compute_physical_name(namespace_path, "users")
//!                 -> "ns_<hash6>__users"
//!   v
//! inner.insert(physical_name, ...)   (all trait methods)
//! ```

use crate::storage::{Schema, StorageEngine, TableInfo};
use crate::Record;
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

/// Wraps a `StorageEngine` to transparently prefix table names with a namespace path.
pub struct NamespacedStorageEngine {
    inner: Arc<dyn StorageEngine>,
    namespace_path: String,
}

impl NamespacedStorageEngine {
    /// Wraps `inner` so all operations on `namespace_path` are scoped to it.
    pub fn new(inner: Arc<dyn StorageEngine>, namespace_path: String) -> Self {
        Self {
            inner,
            namespace_path,
        }
    }

    fn physical_name(&self, table: &str) -> String {
        super::compute_physical_name(&self.namespace_path, table)
    }

    /// Returns the namespace path this adapter is scoped to.
    pub fn namespace_path(&self) -> &str {
        &self.namespace_path
    }
}

#[async_trait]
impl StorageEngine for NamespacedStorageEngine {
    fn as_any(&self) -> &dyn Any {
        self.inner.as_any()
    }

    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        transaction: &crate::transaction::Transaction,
    ) -> crate::Result<u64> {
        let phys = self.physical_name(table);
        self.inner.insert(&phys, data, transaction).await
    }

    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        transaction: &crate::transaction::Transaction,
    ) -> crate::Result<Vec<Record>> {
        let phys = self.physical_name(table);
        self.inner
            .select(&phys, conditions, limit, offset, transaction)
            .await
    }

    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        transaction: &crate::transaction::Transaction,
    ) -> crate::Result<u64> {
        let phys = self.physical_name(table);
        self.inner
            .update(&phys, conditions, data, transaction)
            .await
    }

    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        transaction: &crate::transaction::Transaction,
    ) -> crate::Result<u64> {
        let phys = self.physical_name(table);
        self.inner.delete(&phys, conditions, transaction).await
    }

    async fn analyze(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        transaction: &crate::transaction::Transaction,
    ) -> crate::Result<String> {
        let phys = self.physical_name(table);
        self.inner.analyze(&phys, conditions, transaction).await
    }

    async fn create_table(&self, table: &str, schema: &Schema) -> crate::Result<()> {
        let phys = self.physical_name(table);
        self.inner.create_table(&phys, schema).await
    }

    async fn drop_table(&self, table: &str) -> crate::Result<()> {
        let phys = self.physical_name(table);
        self.inner.drop_table(&phys).await
    }

    async fn truncate_table(&self, table: &str, cascade: bool) -> crate::Result<()> {
        let phys = self.physical_name(table);
        self.inner.truncate_table(&phys, cascade).await
    }

    async fn table_info(&self, table: &str) -> crate::Result<TableInfo> {
        let phys = self.physical_name(table);
        self.inner.table_info(&phys).await
    }
}
