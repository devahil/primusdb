/*
 * PrimusDB Python Driver
 * Copyright (c) 2024-2026 PrimusDB Team <devahil@gmail.com>
 * License: GPL-3.0 - See LICENSE file for details
 * Version: 1.2.0-alpha - Added: Auth, Token, Encryption, Key-Value functions
 */

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use reqwest::Client;

use tokio::runtime::Runtime;

/// Python bindings for PrimusDB - Hybrid Database Engine
/// 
/// This module provides Python bindings for PrimusDB, supporting:
/// - Multiple storage engines: Columnar, Vector, Document, Relational
/// - CRUD operations with async/await support
/// - Authentication and token management
/// - Collection encryption
/// - Transactions
/// 
/// # Quick Start
/// 
/// ```python
/// from primusdb import PrimusDB, StorageType
/// 
/// db = PrimusDB()
/// db.connect("localhost", 8080)
/// 
/// # Create table
/// db.create_table(StorageType.DOCUMENT, "users", '{"name": "str", "age": "int"}')
/// 
/// # Insert data
/// db.insert(StorageType.DOCUMENT, "users", '{"name": "John", "age": 30}')
/// 
/// # Query data
/// result = db.select(StorageType.DOCUMENT, "users", limit=10)
/// print(result)
/// ```
#[pymodule]
fn primusdb_native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyPrimusDBDriver>()?;
    m.add_class::<PyStorageType>()?;
    m.add_function(wrap_pyfunction!(login, m)?)?;
    m.add_function(wrap_pyfunction!(register_user, m)?)?;
    m.add_function(wrap_pyfunction!(create_token, m)?)?;
    m.add_function(wrap_pyfunction!(enable_collection_encryption, m)?)?;
    m.add_function(wrap_pyfunction!(disable_collection_encryption, m)?)?;
    // Key-Value functions
    m.add_function(wrap_pyfunction!(kv_get_db_info, m)?)?;
    m.add_function(wrap_pyfunction!(kv_create_database, m)?)?;
    m.add_function(wrap_pyfunction!(kv_delete_database, m)?)?;
    m.add_function(wrap_pyfunction!(kv_all_docs, m)?)?;
    m.add_function(wrap_pyfunction!(kv_get_document, m)?)?;
    m.add_function(wrap_pyfunction!(kv_put_document, m)?)?;
    m.add_function(wrap_pyfunction!(kv_delete_document, m)?)?;
    m.add_function(wrap_pyfunction!(kv_bulk_docs, m)?)?;
    m.add_function(wrap_pyfunction!(kv_find, m)?)?;
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

// ==================== Key-Value (CouchDB-compatible) Functions ====================

/// Get database information
#[pyfunction]
fn kv_get_db_info(host: &str, port: u16, database: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}", host, port, database);
        
        let mut request = client.get(&url);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Create a Key-Value database
#[pyfunction]
fn kv_create_database(host: &str, port: u16, database: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}", host, port, database);
        
        let mut request = client.put(&url);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Delete a Key-Value database
#[pyfunction]
fn kv_delete_database(host: &str, port: u16, database: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}", host, port, database);
        
        let mut request = client.delete(&url);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Get all documents from database
#[pyfunction]
fn kv_all_docs(host: &str, port: u16, database: &str, token: &str, include_docs: bool, limit: u32, skip: u32) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}/_all_docs?include_docs={}&limit={}&skip={}", 
            host, port, database, include_docs, limit, skip);
        
        let mut request = client.get(&url);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Get a document by ID
#[pyfunction]
fn kv_get_document(host: &str, port: u16, database: &str, doc_id: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}/{}", host, port, database, doc_id);
        
        let mut request = client.get(&url);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Create or update a document
#[pyfunction]
fn kv_put_document(host: &str, port: u16, database: &str, doc_id: &str, data: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}/{}", host, port, database, doc_id);
        
        let data_value: serde_json::Value = serde_json::from_str(data)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON data: {}", e)))?;
        
        let mut request = client.put(&url).json(&data_value);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Delete a document
#[pyfunction]
fn kv_delete_document(host: &str, port: u16, database: &str, doc_id: &str, rev: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}/{}?rev={}", host, port, database, doc_id, rev);
        
        let mut request = client.delete(&url);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Bulk document operations
#[pyfunction]
fn kv_bulk_docs(host: &str, port: u16, database: &str, docs: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}/_bulk_docs", host, port, database);
        
        let docs_value: serde_json::Value = serde_json::from_str(docs)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON docs: {}", e)))?;
        
        let body = serde_json::json!({ "docs": docs_value });
        
        let mut request = client.post(&url).json(&body);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Find documents using Mango query
#[pyfunction]
fn kv_find(host: &str, port: u16, database: &str, selector: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/kv/{}/_find", host, port, database);
        
        let selector_value: serde_json::Value = serde_json::from_str(selector)
            .map_err(|e| PyRuntimeError::new_err(format!("Invalid JSON selector: {}", e)))?;
        
        let mut request = client.post(&url).json(&selector_value);
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Login to PrimusDB server
/// 
/// # Arguments
/// * `host` - Server hostname
/// * `port` - Server port
/// * `username` - Username
/// * `password` - Password
/// 
/// # Returns
/// Login response with token
#[pyfunction]
fn login(host: &str, port: u16, username: &str, password: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/auth/login", host, port);
        
        let body = serde_json::json!({
            "username": username,
            "password": password
        });
        
        let response = client
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

/// Register a new user
/// 
/// # Arguments
/// * `host` - Server hostname
/// * `port` - Server port
/// * `username` - New username
/// * `password` - Password
/// * `email` - Email (optional, pass empty string if not needed)
/// * `roles` - Roles as JSON string (optional)
/// 
/// # Returns
/// Registration response
#[pyfunction]
fn register_user(host: &str, port: u16, username: &str, password: &str, email: &str, roles: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/auth/register", host, port);
        
        let roles_value: serde_json::Value = if roles.is_empty() {
            serde_json::json!(["readonly"])
        } else {
            serde_json::from_str(roles).unwrap_or(serde_json::json!(["readonly"]))
        };
        
        let body = serde_json::json!({
            "username": username,
            "password": password,
            "email": if email.is_empty() { serde_json::Value::Null } else { serde_json::json!(email) },
            "roles": roles_value
        });
        
        let response = client
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

/// Create an API token
/// 
/// # Arguments
/// * `host` - Server hostname
/// * `port` - Server port
/// * `authorization` - Login token
/// * `name` - Token name
/// * `scopes` - Token scopes (JSON string)
/// * `expires_in_hours` - Expiration hours
/// 
/// # Returns
/// Created token info
#[pyfunction]
#[pyo3(signature = (host, port, authorization, name, scopes, expires_in_hours = 8760))]
fn create_token(host: &str, port: u16, authorization: &str, name: &str, scopes: &str, expires_in_hours: u32) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/auth/token/create", host, port);
        
        let body = serde_json::json!({
            "authorization": authorization,
            "name": name,
            "scopes": serde_json::from_str::<serde_json::Value>(scopes).unwrap_or(serde_json::json!([{"resource": "All", "actions": ["Read", "Write"]}])),
            "expires_in_hours": expires_in_hours
        });
        
        let response = client
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

/// Enable encryption for a document collection
/// 
/// # Arguments
/// * `host` - Server hostname
/// * `port` - Server port
/// * `collection` - Collection name
/// * `token` - API token (pass empty string if not needed)
/// 
/// # Returns
/// Encryption status
#[pyfunction]
fn enable_collection_encryption(host: &str, port: u16, collection: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/collection/{}/encrypt", host, port, collection);
        
        let mut request = client.post(&url);
        
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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

/// Disable encryption for a document collection
/// 
/// # Arguments
/// * `host` - Server hostname
/// * `port` - Server port
/// * `collection` - Collection name
/// * `token` - API token (pass empty string if not needed)
/// 
/// # Returns
/// Encryption status
#[pyfunction]
fn disable_collection_encryption(host: &str, port: u16, collection: &str, token: &str) -> PyResult<String> {
    let runtime = Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("Failed to create runtime: {}", e)))?;
    
    runtime.block_on(async {
        let client = Client::new();
        let url = format!("http://{}:{}/api/v1/collection/{}/decrypt", host, port, collection);
        
        let mut request = client.post(&url);
        
        if !token.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request
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
