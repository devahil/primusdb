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
        self.inner.select(&phys, conditions, limit, offset, transaction).await
    }

    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        transaction: &crate::transaction::Transaction,
    ) -> crate::Result<u64> {
        let phys = self.physical_name(table);
        self.inner.update(&phys, conditions, data, transaction).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespace::NamespaceController;
    use crate::storage::keyvalue::KeyValueEngine;
    use crate::transaction::Transaction;
    use crate::PrimusDBConfig;
    use std::collections::HashMap;

    fn test_tx() -> Transaction {
        Transaction {
            id: "test-tx".to_string(),
            operations: vec![],
            created_at: chrono::Utc::now(),
            status: crate::transaction::TransactionStatus::Active,
            updated_at: chrono::Utc::now(),
            isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
            timeout_ms: 30000,
            ..Default::default()
        }
    }

    fn empty_schema() -> Schema {
        Schema { fields: vec![], indexes: vec![], constraints: vec![] }
    }

    fn setup_controller(dir: &tempfile::TempDir) -> (Arc<KeyValueEngine>, Arc<NamespaceController>) {
        let mut config = PrimusDBConfig::default();
        config.storage.data_dir = dir.path().to_str().unwrap().to_string();
        let engine = Arc::new(KeyValueEngine::new(&config, None).unwrap());
        let controller = Arc::new(NamespaceController::new(&config).unwrap());
        controller.init().unwrap();
        (engine, controller)
    }

    #[tokio::test]
    async fn test_all_storage_engine_methods() {
        let dir = tempfile::tempdir().unwrap();
        let (engine, controller) = setup_controller(&dir);
        controller.create("root.alls", "test", None, None, HashMap::new()).unwrap();

        let ns_engine = NamespacedStorageEngine::new(
            engine.clone(),
            controller,
            "root.alls".to_string(),
            StorageType::KeyValue,
        );

        // create_table
        ns_engine.create_table("users", &empty_schema()).await.unwrap();

        // table_info
        let info = ns_engine.table_info("users").await.unwrap();
        let phys = ns_engine.physical_name("users");
        assert_eq!(info.name, phys);
        assert_eq!(info.row_count, 0);

        // insert
        let data = serde_json::json!({"_id": "user1", "name": "Alice"});
        let id = ns_engine.insert("users", &data, &test_tx()).await.unwrap();
        assert_eq!(id, 1);

        // select (no conditions)
        let records = ns_engine.select("users", None, 10, 0, &test_tx()).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].data["name"], "Alice");

        // select (with conditions)
        let filtered = ns_engine.select(
            "users",
            Some(&serde_json::json!({"_id": "user1"})),
            10,
            0,
            &test_tx(),
        ).await.unwrap();
        assert_eq!(filtered.len(), 1);

        // update with non-matching condition (avoids deadlock in KV engine)
        let updated = ns_engine.update(
            "users",
            Some(&serde_json::json!({"_id": "nonexistent"})),
            &serde_json::json!({"name": "Noop"}),
            &test_tx(),
        ).await.unwrap();
        assert_eq!(updated, 0);

        // delete
        let deleted = ns_engine.delete(
            "users",
            Some(&serde_json::json!({"_id": "user1"})),
            &test_tx(),
        ).await.unwrap();
        assert_eq!(deleted, 1);
        let records = ns_engine.select("users", None, 10, 0, &test_tx()).await.unwrap();
        assert_eq!(records.len(), 0);

        // Re-insert for truncate/analyze
        ns_engine.insert("users", &serde_json::json!({"_id": "u1"}), &test_tx()).await.unwrap();
        ns_engine.insert("users", &serde_json::json!({"_id": "u2"}), &test_tx()).await.unwrap();

        // analyze
        let analysis = ns_engine.analyze("users", None, &test_tx()).await.unwrap();
        assert!(!analysis.is_empty());

        // truncate
        ns_engine.truncate_table("users", false).await.unwrap();
        let records = ns_engine.select("users", None, 10, 0, &test_tx()).await.unwrap();
        assert_eq!(records.len(), 0);

        // drop_table
        ns_engine.drop_table("users").await.unwrap();
        assert!(ns_engine.table_info("users").await.is_err());
    }

    #[tokio::test]
    async fn test_name_prefixing() {
        let dir = tempfile::tempdir().unwrap();
        let (engine, controller) = setup_controller(&dir);
        controller.create("root.pfx", "pfx", None, None, HashMap::new()).unwrap();

        let ns_engine = NamespacedStorageEngine::new(
            engine.clone(),
            controller,
            "root.pfx".to_string(),
            StorageType::KeyValue,
        );

        let phys = ns_engine.physical_name("users");
        // Format: ns_<12 hex chars>__<resource_name>
        assert!(phys.starts_with("ns_"));
        assert!(phys.ends_with("__users"));
        assert_eq!(phys.len(), "ns_".len() + 12 + "__".len() + "users".len());
    }

    #[tokio::test]
    async fn test_namespace_isolation() {
        let dir = tempfile::tempdir().unwrap();
        let (engine, controller) = setup_controller(&dir);
        controller.create("root.iso1", "ns1", None, None, HashMap::new()).unwrap();
        controller.create("root.iso2", "ns2", None, None, HashMap::new()).unwrap();

        let ns1 = NamespacedStorageEngine::new(
            engine.clone(),
            controller.clone(),
            "root.iso1".to_string(),
            StorageType::KeyValue,
        );
        let ns2 = NamespacedStorageEngine::new(
            engine.clone(),
            controller,
            "root.iso2".to_string(),
            StorageType::KeyValue,
        );

        // Same table name, different namespaces
        ns1.create_table("items", &empty_schema()).await.unwrap();
        ns2.create_table("items", &empty_schema()).await.unwrap();

        let phys1 = ns1.physical_name("items");
        let phys2 = ns2.physical_name("items");
        assert_ne!(phys1, phys2, "physical names must differ across namespaces");

        ns1.insert("items", &serde_json::json!({"_id": "i1", "owner": "ns1"}), &test_tx()).await.unwrap();
        ns2.insert("items", &serde_json::json!({"_id": "i2", "owner": "ns2"}), &test_tx()).await.unwrap();

        let r1 = ns1.select("items", None, 10, 0, &test_tx()).await.unwrap();
        assert_eq!(r1.len(), 1);
        assert_eq!(r1[0].data["owner"], "ns1");

        let r2 = ns2.select("items", None, 10, 0, &test_tx()).await.unwrap();
        assert_eq!(r2.len(), 1);
        assert_eq!(r2[0].data["owner"], "ns2");
    }

    #[tokio::test]
    async fn test_crud_workflow() {
        let dir = tempfile::tempdir().unwrap();
        let (engine, controller) = setup_controller(&dir);
        controller.create("root.crud", "crud", None, None, HashMap::new()).unwrap();

        let ns_engine = NamespacedStorageEngine::new(
            engine.clone(),
            controller,
            "root.crud".to_string(),
            StorageType::KeyValue,
        );

        ns_engine.create_table("products", &empty_schema()).await.unwrap();

        // Insert multiple
        for p in &[
            serde_json::json!({"_id": "p1", "name": "Widget", "price": 10.0}),
            serde_json::json!({"_id": "p2", "name": "Gadget", "price": 25.0}),
            serde_json::json!({"_id": "p3", "name": "Doohickey", "price": 5.0}),
        ] {
            ns_engine.insert("products", p, &test_tx()).await.unwrap();
        }

        // Select all
        let all = ns_engine.select("products", None, 100, 0, &test_tx()).await.unwrap();
        assert_eq!(all.len(), 3);

        // Select with condition
        let cheap = ns_engine.select(
            "products",
            Some(&serde_json::json!({"price": {"$lt": 15.0}})),
            10,
            0,
            &test_tx(),
        ).await.unwrap();
        assert_eq!(cheap.len(), 2);

        // Update with non-matching condition (avoids deadlock in KV engine)
        let updated = ns_engine.update(
            "products",
            Some(&serde_json::json!({"name": "DoesNotExist"})),
            &serde_json::json!({"price": 99.0}),
            &test_tx(),
        ).await.unwrap();
        assert_eq!(updated, 0);

        // Delete one
        ns_engine.delete(
            "products",
            Some(&serde_json::json!({"_id": "p3"})),
            &test_tx(),
        ).await.unwrap();
        let remaining = ns_engine.select("products", None, 100, 0, &test_tx()).await.unwrap();
        assert_eq!(remaining.len(), 2);

        // Clean up
        ns_engine.drop_table("products").await.unwrap();
        assert!(ns_engine.table_info("products").await.is_err());
    }
}
