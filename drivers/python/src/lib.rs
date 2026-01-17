use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use reqwest::Client;

use tokio::runtime::Runtime;

/// Python bindings for PrimusDB
#[pymodule]
fn primusdb_native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPrimusDBDriver>()?;
    m.add_class::<PyStorageType>()?;
    Ok(())
}

#[pyclass]
pub struct PyPrimusDBDriver {
    client: Client,
    base_url: String,
    runtime: Runtime,
}

#[pymethods]
impl PyPrimusDBDriver {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        let client = Client::new();

        Ok(Self {
            client,
            base_url: String::new(),
            runtime,
        })
    }

    fn connect(&mut self, host: &str, port: u16) -> PyResult<()> {
        self.base_url = format!("http://{}:{}", host, port);
        Ok(())
    }

    fn create_table(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        schema: &str,
    ) -> PyResult<()> {
        let schema_value: serde_json::Value = serde_json::from_str(schema)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON schema: {}", e)))?;

        self.runtime.block_on(async {
            let url = format!("{}/api/v1/tables", self.base_url);
            let body = serde_json::json!({
                "storage_type": storage_type.0,
                "table": table,
                "schema": schema_value
            });

            self.client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Request failed: {}", e)))?;

            Ok(())
        })
    }

    fn insert(&self, storage_type: &PyStorageType, table: &str, data: &str) -> PyResult<u64> {
        let data_value: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON data: {}", e)))?;

        self.runtime.block_on(async {
            let url = format!("{}/api/v1/query", self.base_url);
            let body = serde_json::json!({
                "storage_type": storage_type.0,
                "operation": "Create",
                "table": table,
                "data": data_value
            });

            let response = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Request failed: {}", e)))?;

            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Response parse failed: {}", e)))?;

            if let Some(count) = result.get("count").and_then(|c| c.as_u64()) {
                Ok(count)
            } else {
                Ok(0)
            }
        })
    }

    #[pyo3(signature = (storage_type, table, conditions=None, limit=None, offset=None))]
    fn select(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        conditions: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> PyResult<String> {
        let conditions_value = if let Some(cond) = conditions {
            Some(
                serde_json::from_str::<serde_json::Value>(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?,
            )
        } else {
            None
        };

        self.runtime.block_on(async {
            let url = format!("{}/api/v1/query", self.base_url);
            let body = serde_json::json!({
                "storage_type": storage_type.0,
                "operation": "Read",
                "table": table,
                "conditions": conditions_value,
                "limit": limit,
                "offset": offset
            });

            let response = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Request failed: {}", e)))?;

            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Response parse failed: {}", e)))?;

            serde_json::to_string(&result)
                .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
        })
    }

    #[pyo3(signature = (storage_type, table, data, conditions=None))]
    fn update(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        data: &str,
        conditions: Option<&str>,
    ) -> PyResult<u64> {
        let conditions_value = if let Some(cond) = conditions {
            Some(
                serde_json::from_str::<serde_json::Value>(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?,
            )
        } else {
            None
        };

        let data_value: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON data: {}", e)))?;

        self.runtime.block_on(async {
            let url = format!("{}/api/v1/query", self.base_url);
            let body = serde_json::json!({
                "storage_type": storage_type.0,
                "operation": "Update",
                "table": table,
                "conditions": conditions_value,
                "data": data_value
            });

            let response = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Request failed: {}", e)))?;

            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Response parse failed: {}", e)))?;

            if let Some(count) = result.get("count").and_then(|c| c.as_u64()) {
                Ok(count)
            } else {
                Ok(0)
            }
        })
    }

    #[pyo3(signature = (storage_type, table, conditions=None))]
    fn delete(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        conditions: Option<&str>,
    ) -> PyResult<u64> {
        let conditions_value = if let Some(cond) = conditions {
            Some(
                serde_json::from_str::<serde_json::Value>(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?,
            )
        } else {
            None
        };

        self.runtime.block_on(async {
            let url = format!("{}/api/v1/query", self.base_url);
            let body = serde_json::json!({
                "storage_type": storage_type.0,
                "operation": "Delete",
                "table": table,
                "conditions": conditions_value
            });

            let response = self
                .client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Request failed: {}", e)))?;

            let result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Response parse failed: {}", e)))?;

            if let Some(count) = result.get("count").and_then(|c| c.as_u64()) {
                Ok(count)
            } else {
                Ok(0)
            }
        })
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyStorageType(&'static str);

#[pymethods]
impl PyStorageType {
    #[classattr]
    const COLUMNAR: Self = Self("columnar");

    #[classattr]
    const VECTOR: Self = Self("vector");

    #[classattr]
    const DOCUMENT: Self = Self("document");

    #[classattr]
    const RELATIONAL: Self = Self("relational");

    #[new]
    fn new(storage_type: &str) -> PyResult<Self> {
        let lower = storage_type.to_lowercase();
        match lower.as_str() {
            "columnar" => Ok(Self("columnar")),
            "vector" => Ok(Self("vector")),
            "document" => Ok(Self("document")),
            "relational" => Ok(Self("relational")),
            _ => Err(PyRuntimeError::new_err(format!(
                "Unknown storage type: {}",
                storage_type
            ))),
        }
    }

    fn __str__(&self) -> &str {
        self.0
    }
}
