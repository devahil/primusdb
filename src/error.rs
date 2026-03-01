/*!
# PrimusDB Error Types

This module defines all error types used throughout the PrimusDB system.
Errors are categorized by subsystem and provide detailed context for debugging.

## Error Categories

```
Error Hierarchy:
══════════════════

Error (Root)
├── Storage Errors     - Data persistence and retrieval
├── Transaction Errors - ACID transaction failures
├── Consensus Errors   - Distributed consensus failures
├── Crypto Errors      - Encryption/decryption failures
├── AI/ML Errors       - Machine learning operation failures
├── Cluster Errors     - Distributed system coordination
├── Network Errors     - Communication and connectivity
├── Validation Errors  - Data integrity and constraint violations
├── Configuration Errors - System setup and parameter issues
└── I/O Errors         - File system and hardware failures
```

## Error Handling Strategy

PrimusDB uses a comprehensive error handling strategy:

1. **Typed Errors**: Each subsystem has specific error types
2. **Context Preservation**: Errors carry detailed context information
3. **Recovery Guidance**: Errors suggest recovery actions where possible
4. **Logging Integration**: All errors are logged with appropriate levels
5. **Metrics Integration**: Error rates are tracked for monitoring

## Usage Examples

```rust
use primusdb::Result;

// Function that may fail
fn process_data(data: &serde_json::Value) -> Result<()> {
    // Validate input
    if !data.is_object() {
        return Err(Error::ValidationError(
            "Input must be a JSON object".to_string()
        ));
    }

    // Attempt storage operation
    storage_engine.insert("table", data, &transaction)
        .map_err(|e| Error::StorageError(format!("Insert failed: {}", e)))?;

    Ok(())
}

// Error propagation with context
fn complex_operation() -> Result<()> {
    process_data(&data).map_err(|e| {
        Error::DatabaseError(format!("Complex operation failed: {}", e))
    })
}
```

## Best Practices

1. **Wrap External Errors**: Convert third-party errors to PrimusDB errors
2. **Add Context**: Include relevant context in error messages
3. **Use Appropriate Types**: Choose the most specific error type available
4. **Handle Recovery**: Provide recovery suggestions where applicable
5. **Log Errors**: Ensure errors are logged at appropriate levels
*/

use thiserror::Error;

/// Comprehensive error type for all PrimusDB operations
///
/// This enum covers all possible error conditions that can occur within
/// the PrimusDB system, organized by subsystem for easier debugging and handling.
///
/// # Error Recovery
///
/// Some errors are recoverable (e.g., network timeouts), while others indicate
/// serious system issues (e.g., data corruption). Check the error variant to
/// determine appropriate recovery actions.
#[derive(Error, Debug)]
pub enum Error {
    /// Storage engine requested is not available or not found
    /// This occurs when requesting a storage type that hasn't been registered
    /// Recovery: Check storage configuration and available engines
    #[error("Storage engine not found: {0:?}")]
    StorageEngineNotFound(crate::StorageType),

    /// Transaction operation failed due to concurrency, deadlock, or constraint violation
    /// This includes ACID transaction failures and rollback scenarios
    /// Recovery: Retry transaction or check constraint violations
    #[error("Transaction error: {0}")]
    TransactionError(String),

    /// Consensus algorithm failure in distributed operations
    /// Occurs during block validation, voting, or leader election failures
    /// Recovery: Check network connectivity and node health
    #[error("Consensus error: {0}")]
    ConsensusError(String),

    /// Cryptographic operation failure (encryption/decryption/key management)
    /// Includes key generation, encryption, and integrity verification failures
    /// Recovery: Check key validity and cryptographic configuration
    #[error("Crypto error: {0}")]
    CryptoError(String),

    /// Machine learning or AI operation failure
    /// Includes model training failures, prediction errors, and invalid inputs
    /// Recovery: Validate input data and model state
    #[error("AI/ML error: {0}")]
    AIError(String),

    /// Distributed cluster coordination failure
    /// Includes node discovery, load balancing, and failover operation failures
    /// Recovery: Check cluster configuration and node connectivity
    #[error("Cluster error: {0}")]
    ClusterError(String),

    /// Data validation or integrity constraint violation
    /// Occurs when data doesn't meet schema requirements or business rules
    /// Recovery: Validate and correct input data
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Data corruption detected in storage or transmission
    /// Indicates serious data integrity issues that require immediate attention
    /// Recovery: Restore from backup or repair corrupted data
    #[error("Data corruption detected: {0}")]
    DataCorruption(String),

    /// Network communication failure
    /// Includes connection timeouts, DNS failures, and protocol errors
    /// Recovery: Check network configuration and retry with backoff
    #[error("Network error: {0}")]
    NetworkError(String),

    /// System configuration error or invalid parameters
    /// Occurs during system startup or configuration changes
    /// Recovery: Validate configuration files and parameters
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// File system or hardware I/O operation failure
    /// Includes disk full, permission denied, file not found, etc.
    /// Recovery: Check file system permissions, disk space, and hardware health
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),

    /// Data serialization/deserialization failure
    /// Occurs when converting between Rust types and JSON/network formats
    /// Recovery: Validate data formats and schema compatibility
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// General database operation failure not covered by specific error types
    /// Used for complex operations that may fail for multiple reasons
    /// Recovery: Check system logs for detailed failure information
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Client request contains invalid parameters or violates API constraints
    /// Includes malformed queries, invalid field names, type mismatches
    /// Recovery: Validate request format and parameters before retrying
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Embedded database (sled) operation failure
    /// Sled is used for metadata storage and some storage engines
    /// Recovery: Check sled database integrity and available disk space
    #[error("Sled error: {0}")]
    SledError(#[from] sled::Error),

    /// Asynchronous task execution failure
    /// Occurs when tokio tasks panic or fail to complete
    /// Recovery: Check task implementation and error handling
    #[error("Task join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    /// Integer parsing failure
    /// Occurs when converting strings to numeric types
    /// Recovery: Validate input data before parsing
    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    /// HTTP client operation failure
    /// Includes network issues, SSL errors, and HTTP protocol failures
    /// Recovery: Check network connectivity and endpoint availability
    #[error("HTTP client error: {0}")]
    HttpError(String),

    /// External HTTP request failure (reqwest library)
    /// Used for REST API calls and external service communication
    /// Recovery: Check network configuration and service availability
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    /// Authentication failed due to invalid credentials or token
    /// Includes invalid passwords, expired tokens, and unauthorized access
    /// Recovery: Verify credentials and obtain valid token
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Authorization failed due to insufficient permissions
    /// User is authenticated but lacks required privileges
    /// Recovery: Request elevated privileges from administrator
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
}

/// Convenient type alias for Results containing PrimusDB errors
///
/// This alias is used throughout the PrimusDB codebase to provide
/// consistent error handling with the custom Error enum.
///
/// # Usage
/// ```rust
/// use primusdb::Result;
///
/// fn my_function() -> Result<String> {
///     // Function that may return an error
///     Ok("success".to_string())
/// }
/// ```
pub type Result<T> = std::result::Result<T, Error>;
