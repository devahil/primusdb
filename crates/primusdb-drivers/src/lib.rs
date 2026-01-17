use async_trait::async_trait;
use primusdb_core::{PrimusDB, PrimusDBConfig, Query, QueryOperation, Result, StorageType};
use primusdb_error::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Common interface for all PrimusDB drivers
#[async_trait]
pub trait PrimusDBDriver {
    /// Connect to PrimusDB server
    async fn connect(&mut self, host: &str, port: u16) -> Result<()>;

    /// Execute a query
    async fn execute_query(&self, query: Query) -> Result<serde_json::Value>;

    /// Create a table/collection
    async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<()>;

    /// Insert data
    async fn insert(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
    ) -> Result<u64>;

    /// Select data
    async fn select(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>>;

    /// Update data
    async fn update(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64>;

    /// Delete data
    async fn delete(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64>;

    /// Analyze data
    async fn analyze(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value>;

    /// Make predictions
    async fn predict(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
        prediction_type: &str,
    ) -> Result<serde_json::Value>;

    /// Vector search
    async fn vector_search(
        &self,
        table: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>>;

    /// Cluster data
    async fn cluster(
        &self,
        storage_type: StorageType,
        table: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value>;
}

/// HTTP-based driver for client-server mode
pub struct HTTPDriver {
    client: Client,
    base_url: String,
    connected: bool,
}

impl HTTPDriver {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: String::new(),
            connected: false,
        }
    }

    fn build_query(
        &self,
        storage_type: StorageType,
        operation: QueryOperation,
        table: &str,
        conditions: Option<serde_json::Value>,
        data: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Query {
        Query {
            storage_type,
            operation,
            table: table.to_string(),
            conditions,
            data,
            limit,
            offset,
        }
    }
}

#[async_trait]
impl PrimusDBDriver for HTTPDriver {
    async fn connect(&mut self, host: &str, port: u16) -> Result<()> {
        self.base_url = format!("http://{}:{}", host, port);
        self.connected = true;
        Ok(())
    }

    async fn execute_query(&self, query: Query) -> Result<serde_json::Value> {
        if !self.connected {
            return Err(Error::NetworkError("Not connected to server".to_string()));
        }

        let url = format!("{}/api/v1/query", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&query)
            .send()
            .await
            .map_err(|e| Error::RequestError(e))?;

        let api_response: APIResponse<serde_json::Value> =
            response.json().await.map_err(|e| Error::RequestError(e))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::InvalidRequest("No data in response".to_string()))
        } else {
            Err(Error::InvalidRequest(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    async fn create_table(
        &self,
        storage_type: StorageType,
        table: &str,
        schema: serde_json::Value,
    ) -> Result<()> {
        let query = self.build_query(
            storage_type,
            QueryOperation::Create,
            table,
            None,
            Some(schema),
            None,
            None,
        );
        self.execute_query(query).await?;
        Ok(())
    }

    async fn insert(
        &self,
        storage_type: StorageType,
        table: &str,
        data: serde_json::Value,
    ) -> Result<u64> {
        let query = self.build_query(
            storage_type,
            QueryOperation::Create,
            table,
            None,
            Some(data),
            None,
            None,
        );
        let result = self.execute_query(query).await?;

        // Parse Insert result
        if let serde_json::Value::Object(map) = result {
            if let Some(serde_json::Value::Number(count)) = map.get("count") {
                return Ok(count.as_u64().unwrap_or(0));
            }
        }
        Ok(0)
    }

    async fn select(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        let query = self.build_query(
            storage_type,
            QueryOperation::Read,
            table,
            conditions,
            None,
            limit,
            offset,
        );
        let result = self.execute_query(query).await?;

        // Parse Select result
        if let serde_json::Value::Array(records) = result {
            Ok(records)
        } else {
            Ok(vec![])
        }
    }

    async fn update(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
        data: serde_json::Value,
    ) -> Result<u64> {
        let query = self.build_query(
            storage_type,
            QueryOperation::Update,
            table,
            conditions,
            Some(data),
            None,
            None,
        );
        let result = self.execute_query(query).await?;

        // Parse Update result
        if let serde_json::Value::Object(map) = result {
            if let Some(serde_json::Value::Number(count)) = map.get("count") {
                return Ok(count.as_u64().unwrap_or(0));
            }
        }
        Ok(0)
    }

    async fn delete(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<u64> {
        let query = self.build_query(
            storage_type,
            QueryOperation::Delete,
            table,
            conditions,
            None,
            None,
            None,
        );
        let result = self.execute_query(query).await?;

        // Parse Delete result
        if let serde_json::Value::Object(map) = result {
            if let Some(serde_json::Value::Number(count)) = map.get("count") {
                return Ok(count.as_u64().unwrap_or(0));
            }
        }
        Ok(0)
    }

    async fn analyze(
        &self,
        storage_type: StorageType,
        table: &str,
        conditions: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let query = self.build_query(
            storage_type,
            QueryOperation::Analyze,
            table,
            conditions,
            None,
            None,
            None,
        );
        self.execute_query(query).await
    }

    async fn predict(
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
        let query = self.build_query(
            storage_type,
            QueryOperation::Predict,
            table,
            None,
            Some(predict_data),
            None,
            None,
        );
        self.execute_query(query).await
    }

    async fn vector_search(
        &self,
        table: &str,
        query_vector: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/v1/advanced/vector-search/{}", self.base_url, table);
        let body = serde_json::json!({
            "query_vector": query_vector,
            "limit": limit
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::RequestError(e))?;

        let api_response: APIResponse<Vec<serde_json::Value>> =
            response.json().await.map_err(|e| Error::RequestError(e))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::InvalidRequest("No data in response".to_string()))
        } else {
            Err(Error::InvalidRequest(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    async fn cluster(
        &self,
        storage_type: StorageType,
        table: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let url = format!(
            "{}/api/v1/advanced/cluster/{}/{}",
            self.base_url,
            storage_type.to_string().to_lowercase(),
            table
        );
        let body =
            params.unwrap_or_else(|| serde_json::json!({"algorithm": "kmeans", "clusters": 5}));

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::RequestError(e))?;

        let api_response: APIResponse<serde_json::Value> =
            response.json().await.map_err(|e| Error::RequestError(e))?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| Error::InvalidRequest("No data in response".to_string()))
        } else {
            Err(Error::InvalidRequest(
                api_response
                    .error
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

/// API Response structure for HTTP communication
#[derive(Debug, Serialize, Deserialize)]
pub struct APIResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> APIResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(error_msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error_msg),
            timestamp: chrono::Utc::now(),
        }
    }
}

// Driver modules
#[cfg(feature = "java")]
pub mod java_driver;
#[cfg(feature = "python")]
pub mod python;
#[cfg(feature = "ruby")]
pub mod ruby_driver;
pub mod rust_driver;
