use crate::{HTTPDriver, PrimusDBDriver};
use primusdb_core::{Query, QueryOperation, StorageType};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Python bindings for PrimusDB
#[pymodule]
fn _native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPrimusDBDriver>()?;
    m.add_class::<PyStorageType>()?;
    Ok(())
}

#[pyclass]
pub struct PyPrimusDBDriver {
    driver: Arc<dyn PrimusDBDriver>,
    runtime: Runtime,
}

#[pymethods]
impl PyPrimusDBDriver {
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = Runtime::new()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;

        let driver: Arc<dyn PrimusDBDriver> = Arc::new(HTTPDriver::new());

        Ok(Self { driver, runtime })
    }

    fn connect(&mut self, host: &str, port: u16) -> PyResult<()> {
        self.runtime.block_on(async {
            self.driver
                .connect(host, port)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Connection failed: {:?}", e)))
        })
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
            self.driver
                .create_table(storage_type.0, table, schema_value)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Create table failed: {:?}", e)))
        })
    }

    fn insert(&self, storage_type: &PyStorageType, table: &str, data: &str) -> PyResult<u64> {
        let data_value: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON data: {}", e)))?;

        self.runtime.block_on(async {
            self.driver
                .insert(storage_type.0, table, data_value)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Insert failed: {:?}", e)))
        })
    }

    fn select(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        conditions: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> PyResult<String> {
        let conditions_value =
            if let Some(cond) = conditions {
                Some(serde_json::from_str(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?)
            } else {
                None
            };

        let result = self.runtime.block_on(async {
            self.driver
                .select(storage_type.0, table, conditions_value, limit, offset)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Select failed: {:?}", e)))
        })?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }

    fn update(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        conditions: Option<&str>,
        data: &str,
    ) -> PyResult<u64> {
        let conditions_value =
            if let Some(cond) = conditions {
                Some(serde_json::from_str(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?)
            } else {
                None
            };

        let data_value: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON data: {}", e)))?;

        self.runtime.block_on(async {
            self.driver
                .update(storage_type.0, table, conditions_value, data_value)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Update failed: {:?}", e)))
        })
    }

    fn delete(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        conditions: Option<&str>,
    ) -> PyResult<u64> {
        let conditions_value =
            if let Some(cond) = conditions {
                Some(serde_json::from_str(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?)
            } else {
                None
            };

        self.runtime.block_on(async {
            self.driver
                .delete(storage_type.0, table, conditions_value)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {:?}", e)))
        })
    }

    fn analyze(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        conditions: Option<&str>,
    ) -> PyResult<String> {
        let conditions_value =
            if let Some(cond) = conditions {
                Some(serde_json::from_str(cond).map_err(|e| {
                    PyRuntimeError::new_err(format!("Invalid JSON conditions: {}", e))
                })?)
            } else {
                None
            };

        let result = self.runtime.block_on(async {
            self.driver
                .analyze(storage_type.0, table, conditions_value)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Analyze failed: {:?}", e)))
        })?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }

    fn predict(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        data: &str,
        prediction_type: &str,
    ) -> PyResult<String> {
        let data_value: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON data: {}", e)))?;

        let result = self.runtime.block_on(async {
            self.driver
                .predict(storage_type.0, table, data_value, prediction_type)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Predict failed: {:?}", e)))
        })?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }

    fn vector_search(&self, table: &str, query_vector: Vec<f32>, limit: usize) -> PyResult<String> {
        let result = self.runtime.block_on(async {
            self.driver
                .vector_search(table, query_vector, limit)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Vector search failed: {:?}", e)))
        })?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }

    fn cluster(
        &self,
        storage_type: &PyStorageType,
        table: &str,
        params: Option<&str>,
    ) -> PyResult<String> {
        let params_value = if let Some(p) = params {
            Some(
                serde_json::from_str(p)
                    .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON params: {}", e)))?,
            )
        } else {
            None
        };

        let result = self.runtime.block_on(async {
            self.driver
                .cluster(storage_type.0, table, params_value)
                .await
                .map_err(|e| PyRuntimeError::new_err(format!("Cluster failed: {:?}", e)))
        })?;

        serde_json::to_string(&result)
            .map_err(|e| PyRuntimeError::new_err(format!("Serialization failed: {}", e)))
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyStorageType(StorageType);

#[pymethods]
impl PyStorageType {
    #[classattr]
    const COLUMNAR: Self = Self(StorageType::Columnar);

    #[classattr]
    const VECTOR: Self = Self(StorageType::Vector);

    #[classattr]
    const DOCUMENT: Self = Self(StorageType::Document);

    #[classattr]
    const RELATIONAL: Self = Self(StorageType::Relational);

    #[new]
    fn new(storage_type: &str) -> PyResult<Self> {
        match storage_type.to_lowercase().as_str() {
            "columnar" => Ok(Self(StorageType::Columnar)),
            "vector" => Ok(Self(StorageType::Vector)),
            "document" => Ok(Self(StorageType::Document)),
            "relational" => Ok(Self(StorageType::Relational)),
            _ => Err(PyRuntimeError::new_err(format!(
                "Unknown storage type: {}",
                storage_type
            ))),
        }
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.0)
    }
}
