/*!
# PrimusDB Storage Engine Architecture

This module defines the storage layer of PrimusDB, providing a unified interface
for multiple storage engines optimized for different data access patterns.

## Storage Engine Types

```
Storage Engine Matrix
═══════════════════════════════════════════════════════════════

Feature Comparison:
┌─────────────────┬───────────┬──────────┬──────────┬───────────┐
│ Feature         │ Columnar  │ Vector   │ Document │ Relational│
├─────────────────┼───────────┼──────────┼──────────┼───────────┤
│ Primary Use     │ Analytics │ Search   │ Content  │ Business  │
│ Data Structure  │ Columns   │ Vectors  │ JSON     │ Tables    │
│ Query Pattern   │ OLAP      │ KNN      │ Document │ SQL       │
│ Compression     │ High      │ Medium   │ Low      │ Medium    │
│ Indexes         │ Bitmap    │ FAISS    │ B-Tree   │ B-Tree    │
│ Transactions    │ Snapshot  │ None     │ MVCC     │ ACID      │
│ Scaling         │ Partition │ Shard    │ Replica  │ Shard     │
└─────────────────┴───────────┴──────────┴──────────┴───────────┘

Storage Engine Interfaces:
• StorageEngine trait - Core operations (CRUD + analytics)
• Schema management - Table creation with constraints
• Index management - Automatic and custom indexes
• Transaction support - Varies by engine type
• Performance optimization - Engine-specific tuning
```

## Usage Examples

### Creating Tables
```rust
use primusdb::storage::{Schema, Field, FieldType, Index};

// Define a document collection schema
let user_schema = Schema {
    fields: vec![
        Field {
            name: "id".to_string(),
            field_type: FieldType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "email".to_string(),
            field_type: FieldType::String,
            nullable: false,
            default_value: None,
        },
        Field {
            name: "age".to_string(),
            field_type: FieldType::Integer,
            nullable: true,
            default_value: Some(serde_json::json!(25)),
        },
    ],
    indexes: vec![
        Index {
            name: "email_idx".to_string(),
            fields: vec!["email".to_string()],
            index_type: IndexType::Unique,
        },
    ],
    constraints: vec![],
};

// Create the table
storage_engine.create_table("users", &user_schema).await?;
```

### Data Operations
```rust
// Insert data
let user_data = serde_json::json!({
    "id": "user123",
    "email": "user@example.com",
    "age": 30
});
storage_engine.insert("users", &user_data, &transaction).await?;

// Query with conditions
let conditions = serde_json::json!({"age": {"$gt": 25}});
let users = storage_engine.select("users", Some(&conditions), Some(10), Some(0), &transaction).await?;

// Update records
let update_data = serde_json::json!({"age": 31});
let updated_count = storage_engine.update("users", Some(&conditions), &update_data, &transaction).await?;
```

## Performance Characteristics

### Columnar Engine
- **Best for**: Analytical queries, aggregations, reporting
- **Compression**: LZ4 with bitmap indexes for fast filtering
- **Query Speed**: 1M+ rows/second for aggregations
- **Storage**: 20-50% smaller than row-based storage
- **Limitations**: Slower single-row updates

### Vector Engine
- **Best for**: Similarity search, ML embeddings, recommendations
- **Indexing**: FAISS-style with SIMD acceleration
- **Query Speed**: Sub-millisecond similarity search
- **Scalability**: Efficient sharding for large datasets
- **Limitations**: No complex filtering beyond vector distance

### Document Engine
- **Best for**: Flexible schemas, nested data, content management
- **Indexing**: Dynamic B-Tree indexes on any field
- **Query Speed**: Fast document retrieval with path queries
- **Flexibility**: Schema-less with optional validation
- **Limitations**: No complex joins or aggregations

### Relational Engine
- **Best for**: Traditional applications, complex relationships
- **Features**: Full SQL support, foreign keys, ACID transactions
- **Query Speed**: Optimized for OLTP workloads
- **Consistency**: Strong consistency with serializable isolation
- **Limitations**: Fixed schema, less flexible than document storage
*/

use crate::{Record, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Core trait defining the interface for all storage engines
///
/// Each storage engine implementation must provide these fundamental operations.
/// The trait is async to support high-performance concurrent operations.
///
/// # Transaction Support
/// All operations accept a transaction parameter for atomicity and isolation.
/// The transaction context determines visibility and durability guarantees.
///
/// # Error Handling
/// All methods return `Result<T>` where errors include I/O failures, constraint
/// violations, and transaction conflicts.
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Insert a new record into the specified table
    ///
    /// # Arguments
    /// * `table` - Target table/collection name
    /// * `data` - JSON data to insert
    /// * `transaction` - Transaction context for atomicity
    ///
    /// # Returns
    /// Number of records inserted (usually 1)
    async fn insert(
        &self,
        table: &str,
        data: &serde_json::Value,
        transaction: &crate::transaction::Transaction,
    ) -> Result<u64>;

    /// Query records from the specified table with optional filtering
    ///
    /// # Arguments
    /// * `table` - Target table/collection name
    /// * `conditions` - Optional JSON filter conditions
    /// * `limit` - Maximum number of records to return
    /// * `offset` - Number of records to skip (for pagination)
    /// * `transaction` - Transaction context for consistency
    ///
    /// # Returns
    /// Vector of matching records
    async fn select(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        limit: u64,
        offset: u64,
        transaction: &crate::transaction::Transaction,
    ) -> Result<Vec<Record>>;

    /// Update existing records matching the conditions
    ///
    /// # Arguments
    /// * `table` - Target table/collection name
    /// * `conditions` - JSON conditions to match records for update
    /// * `data` - JSON data to update (partial update supported)
    /// * `transaction` - Transaction context for atomicity
    ///
    /// # Returns
    /// Number of records updated
    async fn update(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        data: &serde_json::Value,
        transaction: &crate::transaction::Transaction,
    ) -> Result<u64>;

    /// Delete records matching the conditions
    ///
    /// # Arguments
    /// * `table` - Target table/collection name
    /// * `conditions` - JSON conditions to match records for deletion
    /// * `transaction` - Transaction context for atomicity
    ///
    /// # Returns
    /// Number of records deleted
    async fn delete(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        transaction: &crate::transaction::Transaction,
    ) -> Result<u64>;

    /// Perform analytical operations on table data
    ///
    /// # Arguments
    /// * `table` - Target table/collection for analysis
    /// * `conditions` - Optional filter for subset analysis
    /// * `transaction` - Transaction context for consistency
    ///
    /// # Returns
    /// JSON string with analysis results (statistics, distributions, etc.)
    async fn analyze(
        &self,
        table: &str,
        conditions: Option<&serde_json::Value>,
        transaction: &crate::transaction::Transaction,
    ) -> Result<String>;

    /// Create a new table/collection with the specified schema
    ///
    /// # Arguments
    /// * `table` - Name of the new table/collection
    /// * `schema` - Schema definition including fields, indexes, constraints
    async fn create_table(&self, table: &str, schema: &Schema) -> Result<()>;

    /// Drop (delete) an existing table/collection
    ///
    /// # Arguments
    /// * `table` - Name of the table/collection to drop
    async fn drop_table(&self, table: &str) -> Result<()>;

    /// Truncate (empty) an existing table/collection
    ///
    /// # Arguments
    /// * `table` - Name of the table/collection to truncate
    async fn truncate_table(&self, table: &str) -> Result<()>;

    /// Get metadata information about a table/collection
    ///
    /// # Arguments
    /// * `table` - Name of the table/collection to inspect
    ///
    /// # Returns
    /// TableInfo structure with schema, size, and performance statistics
    async fn table_info(&self, table: &str) -> Result<TableInfo>;
}

/// Schema definition for tables/collections
///
/// Defines the structure, constraints, and indexes for a table or collection.
/// The schema is used for data validation, query optimization, and storage layout.
///
/// # Schema Components
/// ```
/// Schema
/// ├── Fields        - Column definitions with types and constraints
/// ├── Indexes       - Performance optimization structures
/// └── Constraints   - Data integrity rules
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Field definitions for the table/collection
    /// Each field specifies name, type, nullability, and default values
    pub fields: Vec<Field>,
    /// Index definitions for query performance optimization
    /// Includes primary keys, unique constraints, and secondary indexes
    pub indexes: Vec<Index>,
    /// Data integrity constraints
    /// Foreign keys, check constraints, and custom validation rules
    pub constraints: Vec<Constraint>,
}

/// Field definition within a table/collection schema
///
/// Defines a single column/field in a table, including its data type,
/// nullability, default values, and validation constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    /// Field name (column name)
    /// Must be unique within the table schema
    pub name: String,
    /// Data type of the field
    /// Determines storage format and allowed operations
    pub field_type: FieldType,
    /// Whether the field can contain null values
    /// False means the field is required (NOT NULL)
    pub nullable: bool,
    /// Default value for the field when not specified
    /// Used during INSERT operations when field is omitted
    pub default_value: Option<serde_json::Value>,
    /// Additional constraints on the field
    /// Examples: "PRIMARY KEY", "UNIQUE", "CHECK(value > 0)"
    pub constraints: Vec<String>,
}

/// Data types supported by PrimusDB storage engines
///
/// Each field type is optimized for specific use cases and may have
/// different storage characteristics across different engines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    /// 64-bit signed integer (-2^63 to 2^63-1)
    /// Best for: Counts, IDs, integer mathematics
    /// Storage: 8 bytes, efficient for arithmetic operations
    Integer,

    /// 64-bit IEEE 754 floating point number
    /// Best for: Measurements, calculations, statistical data
    /// Storage: 8 bytes, supports NaN and infinity
    Float,

    /// UTF-8 encoded string with configurable maximum length
    /// Best for: Names, descriptions, categorical data
    /// Storage: Variable length with length prefix
    String,

    /// Boolean value (true/false)
    /// Best for: Flags, binary states, yes/no decisions
    /// Storage: 1 byte, highly compressible
    Boolean,

    /// Date without time component (YYYY-MM-DD)
    /// Best for: Birth dates, event dates, business dates
    /// Storage: 4 bytes, optimized date arithmetic
    Date,

    /// Full timestamp with date and time (ISO 8601)
    /// Best for: Event timestamps, audit trails, temporal data
    /// Storage: 8 bytes, nanosecond precision
    DateTime,

    /// Raw binary data (BLOB)
    /// Best for: Images, documents, serialized objects
    /// Storage: Variable length with compression options
    Binary,

    /// Large text content
    /// Best for: Articles, comments, unstructured text
    /// Storage: Compressed with text-specific algorithms
    Text,

    /// Array/collection of values of the same type
    /// Best for: Tags, categories, multiple values per record
    /// Storage: Variable length with type information
    Array(Box<FieldType>),

    /// Vector embeddings for similarity search
    /// Best for: ML embeddings, feature vectors, semantic search
    /// Storage: SIMD-optimized with FAISS-style indexing
    /// The Vec<usize> specifies dimensions [width, height, depth, ...]
    Vector(Vec<usize>),

    /// Flexible JSON document
    /// Best for: Nested data, flexible schemas, document storage
    /// Storage: BSON-like format with path indexing
    Json,
}

/// Index definition for query performance optimization
///
/// Indexes speed up data retrieval operations by providing efficient
/// lookup structures. Different index types are optimized for different query patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Unique name for the index within the table
    pub name: String,
    /// Fields included in this index (composite indexes supported)
    /// Order matters for B-Tree indexes (determines sort order)
    pub fields: Vec<String>,
    /// Type of index structure to use
    /// Different types optimize different query patterns
    pub index_type: IndexType,
    /// Whether this index enforces uniqueness
    /// Unique indexes prevent duplicate values in indexed fields
    pub unique: bool,
}

/// Types of index structures supported by storage engines
///
/// Each index type is optimized for different query patterns and data types.
/// Choose the appropriate type based on your query workload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    /// B-Tree index for range queries and ordered traversal
    /// Best for: Range queries, ORDER BY, prefix matching
    /// Performance: O(log n) for search, insert, delete
    /// Storage: Moderate overhead, good for most workloads
    BTree,

    /// Hash index for exact equality lookups
    /// Best for: Point queries (=), unique constraints
    /// Performance: O(1) for equality lookups
    /// Storage: Low overhead but no range query support
    Hash,

    /// Vector similarity index using specialized distance metrics
    /// Best for: KNN search, similarity matching, ML embeddings
    /// Performance: Sub-linear with FAISS-style algorithms
    /// Storage: High for large vector datasets
    VectorSimilarity { distance: DistanceMetric },

    /// Full-text search index for text content
    /// Best for: Text search, fuzzy matching, relevance ranking
    /// Features: Stemming, stop words, phrase queries
    /// Storage: High for large text corpora
    FullText,
}

/// Distance metrics for vector similarity search
///
/// Defines how similarity between vectors is calculated.
/// Different metrics are appropriate for different types of data and use cases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistanceMetric {
    /// Euclidean distance (L2 norm)
    /// Best for: General purpose, when magnitude matters
    /// Formula: sqrt(sum((a_i - b_i)^2))
    /// Range: [0, ∞), smaller is more similar
    Euclidean,

    /// Cosine similarity (normalized dot product)
    /// Best for: Text embeddings, direction matters more than magnitude
    /// Formula: (a·b) / (||a|| * ||b||)
    /// Range: [-1, 1], higher is more similar
    Cosine,

    /// Dot product similarity
    /// Best for: Normalized vectors, recommendation systems
    /// Formula: sum(a_i * b_i)
    /// Range: (-∞, ∞), higher is more similar
    DotProduct,

    /// Manhattan distance (L1 norm)
    /// Best for: High-dimensional sparse data, robustness to outliers
    /// Formula: sum(|a_i - b_i|)
    /// Range: [0, ∞), smaller is more similar
    Manhattan,
}

/// Data integrity constraint definition
///
/// Constraints enforce data quality and referential integrity rules.
/// They prevent invalid data from being inserted and maintain consistency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Unique name for the constraint within the table
    pub name: String,
    /// Type of constraint and its specific parameters
    pub constraint_type: ConstraintType,
    /// Fields to which this constraint applies
    /// Can be single field or composite (multiple fields)
    pub fields: Vec<String>,
    /// Additional constraint definition (for CHECK constraints, etc.)
    /// Format depends on constraint type
    pub definition: Option<serde_json::Value>,
}

/// Types of data integrity constraints
///
/// Each constraint type enforces different rules about data validity
/// and relationships between tables/collections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Primary key constraint - uniquely identifies each record
    /// Implies NOT NULL and UNIQUE constraints
    /// Usually implemented with a unique index
    PrimaryKey,

    /// Foreign key constraint - references primary key in another table
    /// Maintains referential integrity between tables
    /// Supports CASCADE, RESTRICT, and SET NULL actions
    ForeignKey {
        /// Table that this foreign key references
        references_table: String,
        /// Field in the referenced table (usually primary key)
        references_field: String,
    },

    /// Unique constraint - prevents duplicate values in specified fields
    /// Allows NULL values (unlike primary keys)
    /// Implemented with unique indexes
    Unique,

    /// Check constraint - validates data against a boolean expression
    /// Examples: "age > 0", "email LIKE '%@%'"
    /// Evaluated for every INSERT/UPDATE operation
    Check {
        /// Boolean expression that must evaluate to true
        /// Supports SQL-like syntax for field references
        expression: String,
    },

    /// Not null constraint - prevents NULL values in specified fields
    /// Ensures required fields always have values
    /// Can be combined with default values
    NotNull,
}

/// Metadata information about a table/collection
///
/// Provides comprehensive statistics and schema information about
/// a table, useful for monitoring, optimization, and introspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table/collection name
    pub name: String,
    /// Complete schema definition
    pub schema: Schema,
    /// Total number of records in the table
    /// May be approximate for very large tables
    pub row_count: u64,
    /// Total storage size in bytes (including indexes)
    /// Includes compression savings where applicable
    pub size_bytes: u64,
    /// Timestamp when the table was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Timestamp when the table was last modified
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Performance and usage metrics for storage operations
///
/// Comprehensive metrics for monitoring storage engine health,
/// performance, and resource utilization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageMetrics {
    /// Total storage size across all tables in bytes
    pub total_size_bytes: u64,
    /// Number of tables/collections in the storage engine
    pub table_count: u32,
    /// Total number of rows across all tables
    pub total_rows: u64,
    /// Cache hit ratio (0.0 to 1.0)
    /// Higher values indicate better cache performance
    pub cache_hit_ratio: f64,
    /// Data compression ratio achieved
    /// Values > 1.0 indicate compression savings
    /// Example: 2.5 means 2.5x compression (60% size reduction)
    pub compression_ratio: f64,
    /// Average read operation latency in milliseconds
    /// Includes disk I/O and decompression time
    pub read_latency_ms: f64,
    /// Average write operation latency in milliseconds
    /// Includes compression and persistence time
    pub write_latency_ms: f64,
}

/// Columnar storage engine implementation
///
/// Optimized for analytical workloads with columnar data layout,
/// LZ4 compression, and bitmap indexes for fast filtering.
pub mod columnar;

/// Document storage engine implementation
///
/// Flexible JSON document storage with dynamic indexing,
/// schema validation, and path-based queries.
pub mod document;

/// Relational storage engine implementation
///
/// Traditional SQL table storage with ACID transactions,
/// foreign key constraints, and relational algebra operations.
pub mod relational;

/// Vector storage engine implementation
///
/// High-performance similarity search with FAISS-style indexing,
/// SIMD operations, and multiple distance metrics.
pub mod vector;

/*
Storage Module Hierarchy:
═══════════════════════════

storage/
├── mod.rs              - Core traits and types
├── columnar/           - Columnar analytics engine
│   ├── mod.rs         - Columnar storage implementation
│   └── index.rs       - Bitmap and compression indexes
├── document/           - Document storage engine
│   ├── mod.rs         - JSON document storage
│   └── index.rs       - Dynamic field indexing
├── relational/         - Relational/SQL engine
│   ├── mod.rs         - Table storage with constraints
│   ├── transaction.rs - ACID transaction support
│   └── query.rs       - SQL parser and optimizer
└── vector/             - Vector similarity engine
    ├── mod.rs         - Vector storage and indexing
    ├── similarity.rs  - Distance metrics and search
    └── quantization.rs- Vector compression techniques
*/
