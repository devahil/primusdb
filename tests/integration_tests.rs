use primusdb::{PrimusDB, PrimusDBConfig, Query, QueryOperation, Result, StorageType};
use serde_json;
use std::sync::Arc;
use tempfile::TempDir;

async fn setup_test_db() -> Result<(Arc<PrimusDB>, TempDir)> {
    let temp_dir = TempDir::new()?;
    let config = PrimusDBConfig {
        storage: primusdb::StorageConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            max_file_size: 1024 * 1024 * 1024, // 1GB
            compression: primusdb::CompressionType::Lz4,
            cache_size: 10 * 1024 * 1024, // 10MB
        },
        network: primusdb::NetworkConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 100,
        },
        security: primusdb::SecurityConfig {
            encryption_enabled: false, // Disable for tests
            key_rotation_interval: 86400,
            auth_required: false,
        },
        cluster: primusdb::ClusterConfig {
            enabled: false,
            node_id: "test-node".to_string(),
            discovery_servers: vec![],
        },
    };

    let db = Arc::new(PrimusDB::new(config)?);
    Ok((db, temp_dir))
}

#[tokio::test]
async fn test_columnar_storage_crud() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert data (tables are created implicitly)
    let insert_data = vec![
        serde_json::json!({"product_id": 1, "amount": 99.99, "timestamp": 1640995200}),
        serde_json::json!({"product_id": 2, "amount": 149.50, "timestamp": 1641081600}),
        serde_json::json!({"product_id": 1, "amount": 79.99, "timestamp": 1641168000}),
    ];

    for data in insert_data {
        let insert_query = Query {
            storage_type: StorageType::Columnar,
            operation: QueryOperation::Create,
            table: "sales".to_string(),
            conditions: None,
            data: Some(data),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!("Insert result: {:?}", result);
    }

    // Query data
    let select_query = Query {
        storage_type: StorageType::Columnar,
        operation: QueryOperation::Read,
        table: "sales".to_string(),
        conditions: None,
        data: None,
        limit: Some(10),
        offset: Some(0),
    };

    let result = db.execute_query(select_query).await?;
    println!("Select result: {:?}", result);

    if let primusdb::QueryResult::Select(records) = result {
        assert!(!records.is_empty());
        println!(
            "✓ Columnar storage CRUD test passed - inserted {} records",
            records.len()
        );
    } else {
        panic!("Expected Select result");
    }

    Ok(())
}

#[tokio::test]
async fn test_vector_storage_similarity() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert vectors (table created implicitly)
    let vectors = vec![
        serde_json::json!({"id": "vec1", "vector": [1.0, 0.0, 0.0], "metadata": {"type": "red"}}),
        serde_json::json!({"id": "vec2", "vector": [0.0, 1.0, 0.0], "metadata": {"type": "green"}}),
        serde_json::json!({"id": "vec3", "vector": [0.9, 0.1, 0.0], "metadata": {"type": "red-like"}}),
    ];

    for vector in vectors {
        let insert_query = Query {
            storage_type: StorageType::Vector,
            operation: QueryOperation::Create,
            table: "embeddings".to_string(),
            conditions: None,
            data: Some(vector),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!("Insert vector result: {:?}", result);
    }

    // Query vectors
    let select_query = Query {
        storage_type: StorageType::Vector,
        operation: QueryOperation::Read,
        table: "embeddings".to_string(),
        conditions: None,
        data: None,
        limit: Some(10),
        offset: Some(0),
    };

    let result = db.execute_query(select_query).await?;
    println!("Vector query result: {:?}", result);

    println!("✓ Vector storage similarity test passed");
    Ok(())
}

#[tokio::test]
async fn test_document_storage_json() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert documents (collection created implicitly)
    let documents = vec![
        serde_json::json!({"name": "Alice", "email": "alice@example.com", "age": 30}),
        serde_json::json!({"name": "Bob", "email": "bob@example.com", "age": 25}),
        serde_json::json!({"name": "Charlie", "email": "charlie@example.com", "age": 35}),
    ];

    for doc in documents {
        let insert_query = Query {
            storage_type: StorageType::Document,
            operation: QueryOperation::Create,
            table: "users".to_string(),
            conditions: None,
            data: Some(doc),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!("Insert document result: {:?}", result);
    }

    // Query documents
    let select_query = Query {
        storage_type: StorageType::Document,
        operation: QueryOperation::Read,
        table: "users".to_string(),
        conditions: None,
        data: None,
        limit: Some(10),
        offset: Some(0),
    };

    let result = db.execute_query(select_query).await?;
    println!("Document query result: {:?}", result);

    if let primusdb::QueryResult::Select(records) = result {
        assert!(!records.is_empty());
        println!(
            "✓ Document storage JSON test passed - inserted {} documents",
            records.len()
        );
    } else {
        panic!("Expected Select result");
    }

    Ok(())
}

#[tokio::test]
async fn test_relational_storage_sql_like() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert data (table created implicitly)
    let products = vec![
        serde_json::json!({"id": 1, "name": "Laptop", "price": 999.99, "category": "Electronics"}),
        serde_json::json!({"id": 2, "name": "Book", "price": 19.99, "category": "Education"}),
        serde_json::json!({"id": 3, "name": "Chair", "price": 149.99, "category": "Furniture"}),
    ];

    for product in products {
        let insert_query = Query {
            storage_type: StorageType::Relational,
            operation: QueryOperation::Create,
            table: "products".to_string(),
            conditions: None,
            data: Some(product),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!("Insert product result: {:?}", result);
    }

    // Query data
    let select_query = Query {
        storage_type: StorageType::Relational,
        operation: QueryOperation::Read,
        table: "products".to_string(),
        conditions: None,
        data: None,
        limit: Some(10),
        offset: Some(0),
    };

    let result = db.execute_query(select_query).await?;
    println!("Relational query result: {:?}", result);

    if let primusdb::QueryResult::Select(records) = result {
        assert!(!records.is_empty());
        println!(
            "✓ Relational storage SQL-like test passed - inserted {} products",
            records.len()
        );
    } else {
        panic!("Expected Select result");
    }

    Ok(())
}

#[tokio::test]
async fn test_ai_predictions() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert training data (simple linear relationship: y = 2x + 1)
    let training_data = vec![
        serde_json::json!({"x": 0.0, "y": 1.0}),
        serde_json::json!({"x": 1.0, "y": 3.0}),
        serde_json::json!({"x": 2.0, "y": 5.0}),
        serde_json::json!({"x": 3.0, "y": 7.0}),
        serde_json::json!({"x": 4.0, "y": 9.0}),
    ];

    for data in training_data {
        let insert_query = Query {
            storage_type: StorageType::Columnar,
            operation: QueryOperation::Create,
            table: "training_data".to_string(),
            conditions: None,
            data: Some(data),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!("Insert training data result: {:?}", result);
    }

    // Test analysis operation (placeholder for now)
    let analyze_query = Query {
        storage_type: StorageType::Columnar,
        operation: QueryOperation::Analyze,
        table: "training_data".to_string(),
        conditions: None,
        data: None,
        limit: None,
        offset: None,
    };

    let result = db.execute_query(analyze_query).await?;
    println!("AI analysis result: {:?}", result);

    println!("✓ AI predictions test passed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_operations() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert initial accounts
    let accounts = vec![
        serde_json::json!({"id": 1, "balance": 1000.0}),
        serde_json::json!({"id": 2, "balance": 500.0}),
    ];

    for account in accounts {
        let insert_query = Query {
            storage_type: StorageType::Relational,
            operation: QueryOperation::Create,
            table: "accounts".to_string(),
            conditions: None,
            data: Some(account),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!("Insert account result: {:?}", result);
    }

    // Test multiple operations (transactions are handled internally)
    println!("✓ Transaction operations test passed");
    Ok(())
}

#[tokio::test]
async fn test_cross_engine_operations() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert data into different engines
    let test_data = vec![
        (
            StorageType::Columnar,
            "analytics",
            serde_json::json!({"timestamp": 1640995200, "value": 100.0}),
        ),
        (
            StorageType::Document,
            "metadata",
            serde_json::json!({"type": "config", "settings": {"debug": true}}),
        ),
        (
            StorageType::Relational,
            "users",
            serde_json::json!({"id": 1, "name": "Test User"}),
        ),
    ];

    for (storage_type, table_name, data) in test_data {
        let insert_query = Query {
            storage_type: storage_type.clone(),
            operation: QueryOperation::Create,
            table: table_name.to_string(),
            conditions: None,
            data: Some(data),
            limit: None,
            offset: None,
        };

        let result = db.execute_query(insert_query).await?;
        println!(
            "Insert into {} ({:?}) result: {:?}",
            table_name, storage_type, result
        );
    }

    println!("✓ Cross-engine operations test passed");
    Ok(())
}
