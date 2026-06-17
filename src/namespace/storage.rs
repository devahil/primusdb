use crate::namespace::NamespaceController;
use crate::storage::{Schema, StorageEngine, TableInfo};
use crate::{Record, StorageType};
use async_trait::async_trait;
use std::any::Any;
use std::sync::Arc;

#[allow(dead_code)]
pub struct NamespacedStorageEngine {
    inner: Arc<dyn StorageEngine>,
    controller: Arc<NamespaceController>,
    namespace_path: String,
    storage_type: StorageType,
}

impl NamespacedStorageEngine {
    pub fn new(
        inner: Arc<dyn StorageEngine>,
        controller: Arc<NamespaceController>,
        namespace_path: String,
        storage_type: StorageType,
    ) -> Self {
        Self {
            inner,
            controller,
            namespace_path,
            storage_type,
        }
    }

    fn physical_name(&self, table: &str) -> String {
        super::compute_physical_name(&self.namespace_path, table)
    }

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
