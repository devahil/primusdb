/*
 * PrimusDB Rust Native Driver
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Builder pattern, Collection abstraction
 */

use primusdb::cluster::ClusterStatus;
use primusdb::{PrimusDB, PrimusDBConfig, Query, QueryOperation, Result, StorageType};
use std::sync::Arc;

/// Native Rust driver for PrimusDB
pub struct NativeDriver {
    db: Arc<PrimusDB>,
}

impl NativeDriver {
    /// Create a new native driver instance
    pub fn new(config: PrimusDBConfig) -> Result<Self> {
        let db = Arc::new(PrimusDB::new(config)?);
        Ok(Self { db })
    }

    /// Get reference to underlying database
    pub fn db(&self) -> &Arc<PrimusDB> {
        &self.db
    }

    /// Execute a raw query
    pub async fn execute_query(&self, query: Query) -> Result<serde_json::Value> {
        let result = self.db.execute_query(query).await?;
        Ok(serde_json::to_value(result)?)
    }

    /// Create a table/collection
    pub async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<()> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Create,
            table: table.to_string(),
            conditions: None,
            data: Some(schema),
            limit: None,
            offset: None,
        };
        self.db.execute_query(query).await?;
        Ok(())
    }

    /// Insert data
    pub async fn insert(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Create,
            table: table.to_string(),
            conditions: None,
            data: Some(data),
            limit: None,
            offset: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Insert(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Select data
    pub async fn select(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Read,
            table: table.to_string(),
            conditions,
            data: None,
            limit,
            offset,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Select(records) => {
                Ok(records.into_iter().map(|r| r.data).collect())
            }
            _ => Ok(vec![]),
        }
    }

    /// Update data
    pub async fn update(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Update,
            table: table.to_string(),
            conditions,
            data: Some(data),
            limit: None,
            offset: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Update(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Delete data
    pub async fn delete(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Delete,
            table: table.to_string(),
            conditions,
            data: None,
            limit: None,
            offset: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Delete(count) => Ok(count),
            _ => Ok(0),
        }
    }

    /// Analyze data patterns
    pub async fn analyze(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let query = Query {
            storage_type,
            operation: QueryOperation::Analyze,
            table: table.to_string(),
            conditions,
            data: None,
            limit: None,
            offset: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Explain(analysis) => Ok(serde_json::Value::String(analysis)),
            _ => Ok(serde_json::Value::Null),
        }
    }

    /// Make AI predictions
    pub async fn predict(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value> {
        let predict_data = serde_json::json!({
            "data": data,
            "prediction_type": prediction_type
        });

        let query = Query {
            storage_type,
            operation: QueryOperation::Predict,
            table: table.to_string(),
            conditions: None,
            data: Some(predict_data),
            limit: None,
            offset: None,
        };

        match self.db.execute_query(query).await? {
            primusdb::QueryResult::Select(predictions) => Ok(serde_json::to_value(predictions)?),
            _ => Ok(serde_json::Value::Null),
        }
    }

    /// Perform vector similarity search
    pub async fn vector_search(
        &self,
        table: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        // Vector search would be implemented as a special query operation
        // For now, return empty result
        Ok(vec![])
    }

    /// Perform data clustering
    pub async fn cluster(
        &self,
        storage_type: StorageType,
        table: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Clustering would be implemented as a special query operation
        // For now, return empty result
        Ok(serde_json::Value::Null)
    }

    /// Get cluster status
    pub fn get_cluster_status(&self) -> Result<ClusterStatus> {
        self.db.get_cluster_status()
    }
}

/// Builder pattern for NativeDriver configuration
pub struct NativeDriverBuilder {
    config: PrimusDBConfig,
}

impl NativeDriverBuilder {
    pub fn new() -> Self {
        Self {
            config: PrimusDBConfig {
                storage: primusdb::StorageConfig {
                    data_dir: "./data".to_string(),
                    max_file_size: 1024 * 1024 * 1024, // 1GB
                    compression: primusdb::CompressionType::Lz4,
                    cache_size: 100 * 1024 * 1024, // 100MB
                },
                network: primusdb::NetworkConfig {
                    bind_address: "127.0.0.1".to_string(),
                    port: 8080,
                    max_connections: 1000,
                },
                security: primusdb::SecurityConfig {
                    encryption_enabled: false,
                    key_rotation_interval: 86400,
                    auth_required: false,
                },
                cluster: primusdb::ClusterConfig {
                    enabled: false,
                    node_id: "native-driver".to_string(),
                    discovery_servers: vec![],
                },
            },
        }
    }

    pub fn data_dir(mut self, path: &str) -> Self {
        self.config.storage.data_dir = path.to_string();
        self
    }

    pub fn max_file_size(mut self, size: u64) -> Self {
        self.config.storage.max_file_size = size;
        self
    }

    pub fn compression(mut self, compression: primusdb::CompressionType) -> Self {
        self.config.storage.compression = compression;
        self
    }

    pub fn cache_size(mut self, size: u64) -> Self {
        self.config.storage.cache_size = size as usize;
        self
    }

    pub fn bind_address(mut self, address: &str) -> Self {
        self.config.network.bind_address = address.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.network.port = port;
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.config.network.max_connections = max as usize;
        self
    }

    pub fn encryption_enabled(mut self, enabled: bool) -> Self {
        self.config.security.encryption_enabled = enabled;
        self
    }

    pub fn auth_required(mut self, required: bool) -> Self {
        self.config.security.auth_required = required;
        self
    }

    pub fn cluster_enabled(mut self, enabled: bool) -> Self {
        self.config.cluster.enabled = enabled;
        self
    }

    pub fn node_id(mut self, id: &str) -> Self {
        self.config.cluster.node_id = id.to_string();
        self
    }

    pub fn discovery_servers(mut self, servers: Vec<String>) -> Self {
        self.config.cluster.discovery_servers = servers;
        self
    }

    pub fn build(self) -> Result<NativeDriver> {
        NativeDriver::new(self.config)
    }
}

impl Default for NativeDriverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level collection abstraction
pub struct Collection {
    driver: NativeDriver,
    storage_type: StorageType,
    name: String,
}

impl Collection {
    pub fn new(driver: NativeDriver, storage_type: StorageType, name: &str) -> Self {
        Self {
            driver,
            storage_type,
            name: name.to_string(),
        }
    }

    pub async fn insert_one(&self, data: serde_json::Value) -> Result<u64> {
        self.driver
            .insert(self.storage_type, &self.name, data)
            .await
    }

    pub async fn find(
        &self,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        self.driver
            .select(self.storage_type, &self.name, conditions, limit, offset)
            .await
    }

    pub async fn update_one(
        &self,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64> {
        self.driver
            .update(self.storage_type, &self.name, conditions, data)
            .await
    }

    pub async fn delete_one(&self, conditions: Option<serde_json::Value>) -> Result<u64> {
        self.driver
            .delete(self.storage_type, &self.name, conditions)
            .await
    }

    pub async fn count(&self, conditions: Option<serde_json::Value>) -> Result<u64> {
        let results = self.find(conditions, Some(1000000), None).await?;
        Ok(results.len() as u64)
    }

    pub async fn analyze(
        &self,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.driver
            .analyze(self.storage_type, &self.name, conditions)
            .await
    }

    pub async fn predict(
        &self,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value> {
        self.driver
            .predict(self.storage_type, &self.name, data, prediction_type)
            .await
    }
}

/// Database abstraction
pub struct Database {
    driver: NativeDriver,
}

impl Database {
    pub fn new(driver: NativeDriver) -> Self {
        Self { driver }
    }

    pub fn collection(&self, storage_type: StorageType, name: &str) -> Collection {
        Collection::new(self.driver.clone(), storage_type, name)
    }

    pub async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<()> {
        self.driver.create_table(storage_type, table, schema).await
    }

    pub fn get_cluster_status(&self) -> Result<ClusterStatus> {
        self.driver.get_cluster_status()
    }
}

impl Clone for NativeDriver {
    fn clone(&self) -> Self {
        // Since we have Arc<PrimusDB>, we can share the reference
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (NativeDriver, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = PrimusDBConfig {
            storage: primusdb::StorageConfig {
                data_dir: temp_dir.path().to_string_lossy().to_string(),
                max_file_size: 1024 * 1024 * 1024,
                compression: primusdb::CompressionType::Lz4,
                cache_size: 10 * 1024 * 1024,
            },
            network: primusdb::NetworkConfig {
                bind_address: "127.0.0.1".to_string(),
                port: 8080,
                max_connections: 100,
            },
            security: primusdb::SecurityConfig {
                encryption_enabled: false,
                key_rotation_interval: 86400,
                auth_required: false,
            },
            cluster: primusdb::ClusterConfig {
                enabled: false,
                node_id: "test-driver".to_string(),
                discovery_servers: vec![],
            },
        };

        let driver = NativeDriver::new(config).unwrap();
        (driver, temp_dir)
    }

    #[tokio::test]
    async fn test_native_driver_crud() {
        let (driver, _temp_dir) = setup_test_db().await;

        // Insert data
        let data = serde_json::json!({"name": "Test Item", "value": 42});
        let count = driver
            .insert(StorageType::Document, "test_collection", data)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Select data
        let results = driver
            .select(
                StorageType::Document,
                "test_collection",
                None,
                Some(10),
                Some(0),
            )
            .await
            .unwrap();
        assert!(!results.is_empty());

        println!("✓ Native driver CRUD test passed");
    }

    #[tokio::test]
    async fn test_driver_builder() {
        let driver = NativeDriverBuilder::new()
            .data_dir("/tmp/test")
            .port(9090)
            .build()
            .unwrap();

        // Just test that it builds successfully
        assert!(driver.db().get_cluster_status().is_ok());
    }
}
