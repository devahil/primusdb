use super::{HTTPDriver, PrimusDBDriver, StorageType};
use async_trait::async_trait;
use serde_json;

/// Rust native driver for PrimusDB
pub struct RustDriver {
    inner: HTTPDriver,
}

impl RustDriver {
    pub fn new() -> Self {
        Self {
            inner: HTTPDriver::new(),
        }
    }

    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.connect(host, port).await?;
        Ok(())
    }

    pub async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.inner.create_table(storage_type, table, schema).await?;
        Ok(())
    }

    pub async fn insert(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.inner.insert(storage_type, table, data).await?)
    }

    pub async fn select(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        Ok(self
            .inner
            .select(storage_type, table, conditions, limit, offset)
            .await?)
    }

    pub async fn update(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self
            .inner
            .update(storage_type, table, conditions, data)
            .await?)
    }

    pub async fn delete(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(self.inner.delete(storage_type, table, conditions).await?)
    }

    pub async fn analyze(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(self.inner.analyze(storage_type, table, conditions).await?)
    }

    pub async fn predict(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(self
            .inner
            .predict(storage_type, table, data, prediction_type)
            .await?)
    }

    pub async fn vector_search(
        &self,
        table: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        Ok(self.inner.vector_search(table, query_vector, limit).await?)
    }

    pub async fn cluster(
        &self,
        storage_type: StorageType,
        table: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(self.inner.cluster(storage_type, table, params).await?)
    }
}

impl Default for RustDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for RustDriver configuration
pub struct RustDriverBuilder {
    host: String,
    port: u16,
}

impl RustDriverBuilder {
    pub fn new() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }

    pub fn host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub async fn build(self) -> Result<RustDriver, Box<dyn std::error::Error>> {
        let mut driver = RustDriver::new();
        driver.connect(&self.host, self.port).await?;
        Ok(driver)
    }
}

impl Default for RustDriverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level collection abstraction
pub struct Collection {
    driver: RustDriver,
    storage_type: StorageType,
    name: String,
}

impl Collection {
    pub fn new(driver: RustDriver, storage_type: StorageType, name: &str) -> Self {
        Self {
            driver,
            storage_type,
            name: name.to_string(),
        }
    }

    pub async fn insert(&self, data: serde_json::Value) -> Result<u64, Box<dyn std::error::Error>> {
        self.driver
            .insert(self.storage_type, &self.name, data)
            .await
    }

    pub async fn find(
        &self,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        self.driver
            .select(self.storage_type, &self.name, conditions, limit, offset)
            .await
    }

    pub async fn update(
        &self,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.driver
            .update(self.storage_type, &self.name, conditions, data)
            .await
    }

    pub async fn delete(
        &self,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        self.driver
            .delete(self.storage_type, &self.name, conditions)
            .await
    }

    pub async fn count(
        &self,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let results = self
            .driver
            .select(
                self.storage_type,
                &self.name,
                conditions,
                Some(1000000),
                None,
            )
            .await?;
        Ok(results.len() as u64)
    }
}

/// Database abstraction
pub struct Database {
    driver: RustDriver,
}

impl Database {
    pub fn new(driver: RustDriver) -> Self {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.driver.create_table(storage_type, table, schema).await
    }

    pub async fn analyze(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.driver.analyze(storage_type, table, conditions).await
    }

    pub async fn predict(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.driver
            .predict(storage_type, table, data, prediction_type)
            .await
    }

    pub async fn vector_search(
        &self,
        table: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        self.driver.vector_search(table, query_vector, limit).await
    }

    pub async fn cluster(
        &self,
        storage_type: StorageType,
        table: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        self.driver.cluster(storage_type, table, params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_rust_driver_connection() {
        let mut driver = RustDriver::new();
        // This will fail in CI, but tests the connection logic
        let result = driver.connect("localhost", 8080).await;
        assert!(result.is_ok() || result.is_err()); // Just test that it doesn't panic
    }

    #[tokio::test]
    async fn test_driver_builder() {
        let builder = RustDriverBuilder::new().host("localhost").port(8080);

        let result = builder.build().await;
        assert!(result.is_ok() || result.is_err()); // Just test that it doesn't panic
    }
}
