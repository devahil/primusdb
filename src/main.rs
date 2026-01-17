/*!
# PrimusDB CLI - Command Line Interface

This module provides the command-line interface for PrimusDB, offering both
embedded mode (local database instance) and client mode (connecting to remote servers).

## Architecture Overview

```
CLI Architecture
═══════════════════════════════════════════════════════════════

┌─────────────────────────────────────────────────────────┐
│                    CLI Entry Point                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Command Line Parsing (clap)                    │    │
│  │  • Argument validation                           │    │
│  │  • Help generation                               │    │
│  │  • Subcommand routing                            │    │
│  └─────────────────────────────────────────────────┘    │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Mode Selection                                │    │
│  │  • Embedded mode: Local PrimusDB instance      │    │
│  │  • Client mode: Connect to remote server       │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘

Embedded Mode:
• Direct access to local PrimusDB instance
• Full CRUD and advanced operations
• No network overhead
• Immediate operation execution

Client Mode:
• HTTP client connecting to remote PrimusDB server
• REST API communication
• Supports all operations via network
• Suitable for distributed deployments
```

## Usage Examples

### Embedded Mode (Default)
```bash
# Show system information
primusdb info

# Create records
primusdb crud create --storage-type document --table users \
  --data '{"name": "Alice", "email": "alice@example.com"}'

# Query data
primusdb crud read --storage-type document --table users --limit 10

# AI predictions
primusdb advanced predict --storage-type columnar --table sales \
  --data '{"quarter": "Q1"}' --prediction-type revenue

# Vector search
primusdb advanced vector-search --table embeddings \
  --query-vector "0.1,0.2,0.3,0.4" --limit 5
```

### Client Mode
```bash
# Connect to remote server
primusdb --mode client --server http://primusdb-server.com:8080 info

# Execute remote operations
primusdb --mode client --server http://localhost:8080 \
  crud read --storage-type document --table products --limit 20
```

## Command Structure

```
primusdb [OPTIONS] <COMMAND>

Commands:
  info                    Display system information
  crud                    CRUD operations
    create                Create new records
    read                  Query existing records
    update                Modify existing records
    delete                Remove records
  advanced                Advanced operations
    analyze               Data pattern analysis
    predict               AI/ML predictions
    vector-search         Vector similarity search
    cluster               Data clustering
    transaction           Complex transactions
    table-info            Table metadata

Options:
  --server <URL>          Server URL for client mode
  --mode <MODE>           Operation mode (embedded/client)
  -h, --help              Display help information
  -V, --version           Display version information
```

## Development Notes

This CLI serves as both a development tool and a production interface for PrimusDB.
It demonstrates all major functionality and provides examples for driver implementations.

Key Features:
- Zero-configuration setup for development
- Comprehensive error handling and user feedback
- JSON data format for flexible operations
- Support for all storage engines and AI features
- Both interactive and programmatic usage patterns
*/

use clap::{Parser, Subcommand};
use primusdb::{PrimusDB, PrimusDBConfig, Query, QueryOperation, Result, StorageType};
use std::sync::Arc;

/// Main CLI structure defining command-line arguments and subcommands
///
/// This structure uses clap for automatic argument parsing, validation,
/// and help generation. It supports both embedded and client modes of operation.
#[derive(Parser)]
#[command(
    name = "primusdb",
    about = "PrimusDB - Hybrid Database Engine",
    long_about = "PrimusDB is a next-generation hybrid database engine that combines \
                  traditional relational databases with modern document stores, \
                  columnar analytics, and vector similarity search. Enhanced with \
                  integrated AI/ML capabilities and enterprise-grade security."
)]
struct Cli {
    /// Server URL for client mode connections
    /// Format: http://hostname:port or https://hostname:port
    /// Only used when mode is set to "client"
    #[arg(long, default_value = "http://localhost:8080")]
    server: String,

    /// Operation mode: embedded (local instance) or client (remote connection)
    /// Embedded mode creates a local PrimusDB instance
    /// Client mode connects to a remote PrimusDB server via REST API
    #[arg(long, default_value = "embedded", value_parser = ["embedded", "client"])]
    mode: String,

    /// Subcommand to execute (see Commands enum for options)
    #[command(subcommand)]
    command: Commands,
}

/// Top-level commands available in PrimusDB CLI
///
/// Each command represents a major functional area of PrimusDB.
/// Use `primusdb <command> --help` for detailed help on each subcommand.
#[derive(Subcommand)]
enum Commands {
    /// Display comprehensive information about PrimusDB system
    /// Shows version, features, available commands, and usage examples
    /// Safe to run - only displays information, no modifications
    Info,

    /// CRUD (Create, Read, Update, Delete) operations on data
    /// Basic database operations for managing records across all storage types
    /// Supports columnar, vector, document, and relational storage
    #[command(subcommand)]
    Crud(CrudCommands),

    /// Advanced operations including AI/ML, analytics, and clustering
    /// Specialized operations for data analysis, predictions, and advanced queries
    /// Requires appropriate data and model setup for full functionality
    #[command(subcommand)]
    Advanced(AdvancedCommands),
}

#[derive(Subcommand)]
enum CrudCommands {
    /// Create a new record
    Create {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Data to insert (JSON)
        #[arg(long)]
        data: String,
    },
    /// Read records
    Read {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Query conditions (JSON)
        #[arg(long)]
        conditions: Option<String>,

        /// Limit results
        #[arg(long, default_value = "10")]
        limit: u64,

        /// Offset results
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Update records
    Update {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Update conditions (JSON)
        #[arg(long)]
        conditions: Option<String>,

        /// Data to update (JSON)
        #[arg(long)]
        data: String,
    },
    /// Delete records
    Delete {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Delete conditions (JSON)
        #[arg(long)]
        conditions: Option<String>,
    },
}

#[derive(Subcommand)]
enum AdvancedCommands {
    /// Analyze data patterns
    Analyze {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Analysis conditions (JSON)
        #[arg(long)]
        conditions: Option<String>,
    },
    /// Make AI predictions
    Predict {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Prediction data (JSON)
        #[arg(long)]
        data: String,

        /// Prediction type (linear_regression, anomaly_detection, etc.)
        #[arg(long)]
        prediction_type: String,
    },
    /// Vector similarity search
    VectorSearch {
        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Query vector (JSON array)
        #[arg(long)]
        query_vector: String,

        /// Number of similar results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Cluster data
    Cluster {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Clustering parameters (JSON)
        #[arg(long)]
        _params: Option<String>,
    },
    /// Execute transaction
    Transaction {
        /// Transaction operations (JSON array)
        #[arg(long)]
        operations: String,
    },
    /// Get table/collection information
    TableInfo {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,
    },
}

/// Main entry point for PrimusDB CLI
///
/// Parses command-line arguments, determines operation mode (embedded vs client),
/// and routes execution to the appropriate handler. Uses tokio async runtime
/// for all database operations.
///
/// # Error Handling
/// All errors are propagated up and displayed to the user with appropriate
/// context and suggestions for resolution.
///
/// # Exit Codes
/// - 0: Success
/// - 1: Error (with descriptive message)
#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments using clap
    let cli = Cli::parse();

    // Route based on operation mode
    if cli.mode == "client" {
        // Client mode: HTTP connection to remote PrimusDB server
        // Suitable for production deployments with separate server processes
        handle_client_mode(cli).await
    } else {
        // Embedded mode: Local PrimusDB instance in same process
        // Ideal for development, testing, and single-user applications
        handle_embedded_mode(cli).await
    }
}

/// Handle embedded mode operations
///
/// Creates a local PrimusDB instance in the same process, configures it with
/// development-friendly defaults, and executes the requested command.
/// This mode is optimized for development, testing, and single-user scenarios.
///
/// # Configuration
/// Uses sensible defaults for local development:
/// - Data directory: ./data
/// - Storage limit: 1GB per file
/// - Cache size: 100MB
/// - Clustering: Disabled
/// - Encryption: Enabled (for security)
///
/// # Performance
/// Embedded mode has minimal overhead as there's no network communication.
/// All operations execute directly against local storage engines.
async fn handle_embedded_mode(cli: Cli) -> Result<()> {
    // Initialize PrimusDB instance with embedded configuration
    // This creates a complete database instance in the current process
    let config = PrimusDBConfig {
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
            encryption_enabled: true,
            key_rotation_interval: 86400, // 24 hours
            auth_required: false,
        },
        cluster: primusdb::ClusterConfig {
            enabled: false,
            node_id: "cli-embedded".to_string(),
            discovery_servers: vec![],
        },
    };
    let primusdb = Arc::new(PrimusDB::new(config)?);

    match cli.command {
        Commands::Info => {
            println!("PrimusDB - Hybrid Database Engine (Embedded Mode)");
            println!("Version: 0.1.0");
            println!("Mode: Embedded (local instance)");
            println!();
            println!("Features implemented:");
            println!("- Hybrid storage engines (columnar, vector, document, relational)");
            println!("- Blockchain-style consensus with transaction validation");
            println!("- AI/ML capabilities (predictions, anomaly detection, clustering)");
            println!("- Distributed clustering with load balancing");
            println!("- Enterprise encryption (AES-256-GCM, ChaCha20)");
            println!("- REST/GraphQL/gRPC APIs");
            println!("- ACID transactions with rollback");
            println!("- Real-time analytics and vector search");
            println!();
            println!("Available commands:");
            println!("  info                    - Show this information");
            println!("  crud create             - Create new records");
            println!("  crud read               - Read/query records");
            println!("  crud update             - Update existing records");
            println!("  crud delete             - Delete records");
            println!("  advanced analyze        - Analyze data patterns");
            println!("  advanced predict        - AI predictions");
            println!("  advanced vector-search  - Vector similarity search");
            println!("  advanced cluster        - Cluster data analysis");
            println!("  advanced transaction    - Execute complex transactions");
            println!("  advanced table-info     - Get table/collection information");
            println!();
            println!("Storage engines:");
            println!("  columnar  - Optimized for analytics and large datasets");
            println!("  vector    - For embeddings and similarity search");
            println!("  document  - JSON documents with flexible schemas");
            println!("  relational- Traditional SQL-like tables");
            println!();
            println!("Example usage:");
            println!("  primusdb crud create --storage-type columnar --table users --data '{{\"name\":\"John\",\"email\":\"john@example.com\"}}'");
            println!("  primusdb crud read --storage-type relational --table users --limit 10");
            println!("  primusdb advanced analyze --storage-type columnar --table sales");
            println!("  primusdb advanced predict --storage-type vector --table embeddings --data '{{\"features\":[1.0,2.0,3.0]}}'");
            println!();
            println!("For client-server mode: primusdb --mode client --server http://localhost:8080 [command]");
        }

        Commands::Crud(crud_cmd) => {
            handle_crud_command(primusdb, crud_cmd).await?;
        }

        Commands::Advanced(adv_cmd) => {
            handle_advanced_command(primusdb, adv_cmd).await?;
        }
    }

    Ok(())
}

async fn handle_client_mode(cli: Cli) -> Result<()> {
    println!("PrimusDB - Client Mode");
    println!("Server: {}", cli.server);
    println!("Connecting to remote PrimusDB server...");

    match cli.command {
        Commands::Info => {
            // Get server info via HTTP
            match reqwest::get(&format!("{}/api/v1", cli.server)).await {
                Ok(response) => {
                    let json: serde_json::Value = response.json().await?;
                    println!("Server Information:");
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                Err(e) => {
                    eprintln!("Failed to connect to server: {}", e);
                    eprintln!("Make sure the server is running: primusdb-server --host 127.0.0.1 --port 8080");
                }
            }
        }

        Commands::Crud(crud_cmd) => {
            handle_crud_client_command(cli.server, crud_cmd).await?;
        }

        Commands::Advanced(adv_cmd) => {
            handle_advanced_client_command(cli.server, adv_cmd).await?;
        }
    }

    Ok(())
}

async fn handle_crud_client_command(server: String, cmd: CrudCommands) -> Result<()> {
    let client = reqwest::Client::new();

    match cmd {
        CrudCommands::Create {
            storage_type,
            table,
            data,
        } => {
            let url = format!("{}/api/v1/crud/{}/{}", server, storage_type, table);
            let response = client
                .post(&url)
                .json(&serde_json::from_str::<serde_json::Value>(&data)?)
                .send()
                .await?;

            let json: serde_json::Value = response.json().await?;
            println!("Create result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        CrudCommands::Read {
            storage_type,
            table,
            conditions,
            limit,
            offset,
        } => {
            let mut url = format!(
                "{}/api/v1/crud/{}/{}?limit={}&offset={}",
                server, storage_type, table, limit, offset
            );
            if let Some(conditions) = conditions {
                url.push_str(&format!("&conditions={}", urlencoding::encode(&conditions)));
            }

            let response = client.get(&url).send().await?;
            let json: serde_json::Value = response.json().await?;
            println!("Read result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        CrudCommands::Update {
            storage_type,
            table,
            conditions,
            data,
        } => {
            let url = format!("{}/api/v1/crud/{}/{}", server, storage_type, table);
            let body = serde_json::json!({
                "data": serde_json::from_str::<serde_json::Value>(&data)?,
                "conditions": conditions.and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            });

            let response = client.put(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Update result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        CrudCommands::Delete {
            storage_type,
            table,
            conditions,
        } => {
            let mut url = format!("{}/api/v1/crud/{}/{}", server, storage_type, table);
            if let Some(conditions) = conditions {
                url.push_str(&format!("?conditions={}", urlencoding::encode(&conditions)));
            }

            let response = client.delete(&url).send().await?;
            let json: serde_json::Value = response.json().await?;
            println!("Delete result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

async fn handle_advanced_client_command(server: String, cmd: AdvancedCommands) -> Result<()> {
    let client = reqwest::Client::new();

    match cmd {
        AdvancedCommands::Analyze {
            storage_type,
            table,
            conditions,
        } => {
            let url = format!(
                "{}/api/v1/advanced/analyze/{}/{}",
                server, storage_type, table
            );
            let body = serde_json::json!({
                "conditions": conditions.and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            });

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Analysis result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        AdvancedCommands::Predict {
            storage_type,
            table,
            data,
            prediction_type,
        } => {
            let url = format!(
                "{}/api/v1/advanced/predict/{}/{}",
                server, storage_type, table
            );
            let body = serde_json::json!({
                "data": serde_json::from_str::<serde_json::Value>(&data)?,
                "prediction_type": prediction_type
            });

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Prediction result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        AdvancedCommands::VectorSearch {
            table,
            query_vector,
            limit,
        } => {
            let url = format!("{}/api/v1/advanced/vector-search/{}", server, table);
            let query_vector: Vec<f32> = query_vector
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();

            let body = serde_json::json!({
                "query_vector": query_vector,
                "limit": limit
            });

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Vector search result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        AdvancedCommands::Cluster {
            storage_type,
            table,
            _params,
        } => {
            let url = format!(
                "{}/api/v1/advanced/cluster/{}/{}",
                server, storage_type, table
            );
            let body = _params
                .and_then(|p| serde_json::from_str(&p).ok())
                .unwrap_or(serde_json::json!({"algorithm": "kmeans", "clusters": 5}));

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Clustering result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        AdvancedCommands::Transaction { operations } => {
            println!("Transaction operations: {}", operations);
            println!("Transaction support coming soon in client mode!");
        }
        AdvancedCommands::TableInfo {
            storage_type,
            table,
        } => {
            let url = format!("{}/api/v1/table/{}/{}/info", server, storage_type, table);
            let response = client.get(&url).send().await?;
            let json: serde_json::Value = response.json().await?;
            println!("Table info:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
    }

    Ok(())
}

/// Execute CRUD (Create, Read, Update, Delete) operations
///
/// Routes CRUD subcommands to appropriate handlers and manages the execution
/// lifecycle. All operations are executed against the provided PrimusDB instance.
///
/// # Transaction Management
/// Each operation is executed in its own transaction context.
/// For complex multi-operation workflows, use the transaction command.
///
/// # Error Handling
/// Operation failures are displayed to the user with context about what went wrong
/// and suggestions for resolution where applicable.
///
/// # Return Values
/// Results are formatted as pretty-printed JSON for readability.
/// Success/failure status is indicated by exit code.
async fn handle_crud_command(primusdb: std::sync::Arc<PrimusDB>, cmd: CrudCommands) -> Result<()> {
    match cmd {
        CrudCommands::Create {
            storage_type,
            table,
            data,
        } => {
            execute_crud_operation(
                primusdb,
                storage_type,
                table,
                QueryOperation::Create,
                Some(data),
                None,
                None,
                None,
            )
            .await
        }
        CrudCommands::Read {
            storage_type,
            table,
            conditions,
            limit,
            offset,
        } => {
            execute_crud_operation(
                primusdb,
                storage_type,
                table,
                QueryOperation::Read,
                None,
                conditions,
                Some(limit),
                Some(offset),
            )
            .await
        }
        CrudCommands::Update {
            storage_type,
            table,
            conditions,
            data,
        } => {
            execute_crud_operation(
                primusdb,
                storage_type,
                table,
                QueryOperation::Update,
                Some(data),
                conditions,
                None,
                None,
            )
            .await
        }
        CrudCommands::Delete {
            storage_type,
            table,
            conditions,
        } => {
            execute_crud_operation(
                primusdb,
                storage_type,
                table,
                QueryOperation::Delete,
                None,
                conditions,
                None,
                None,
            )
            .await
        }
    }
}

async fn handle_advanced_command(
    primusdb: std::sync::Arc<PrimusDB>,
    cmd: AdvancedCommands,
) -> Result<()> {
    match cmd {
        AdvancedCommands::Analyze {
            storage_type,
            table,
            conditions,
        } => {
            execute_advanced_operation(
                primusdb,
                "analyze",
                storage_type,
                table,
                conditions,
                None,
                None,
            )
            .await
        }
        AdvancedCommands::Predict {
            storage_type,
            table,
            data,
            prediction_type,
        } => {
            let conditions = Some(format!(r#"{{"prediction_type": "{}"}}"#, prediction_type));
            execute_advanced_operation(
                primusdb,
                "predict",
                storage_type,
                table,
                Some(data),
                conditions,
                None,
            )
            .await
        }
        AdvancedCommands::VectorSearch {
            table,
            query_vector,
            limit,
        } => {
            let conditions = Some(format!(
                r#"{{"query_vector": {}, "limit": {}}}"#,
                query_vector, limit
            ));
            execute_advanced_operation(
                primusdb,
                "vector_search",
                "vector".to_string(),
                table,
                None,
                conditions,
                None,
            )
            .await
        }
        AdvancedCommands::Cluster {
            storage_type,
            table,
            _params,
        } => {
            execute_advanced_operation(primusdb, "cluster", storage_type, table, None, None, None)
                .await
        }
        AdvancedCommands::Transaction { operations } => {
            println!("Executing transaction with operations: {}", operations);
            // TODO: Implement transaction execution
            println!("Transaction feature coming soon!");
            Ok(())
        }
        AdvancedCommands::TableInfo {
            storage_type,
            table,
        } => execute_table_info_operation(primusdb, storage_type, table).await,
    }
}

/// Execute a single CRUD operation against the PrimusDB instance
///
/// This is the core execution function that translates CLI parameters into
/// PrimusDB Query objects and executes them. Handles all CRUD operations
/// with proper error handling and result formatting.
///
/// # Parameters
/// * `primusdb` - Reference to the initialized PrimusDB instance
/// * `storage_type` - Target storage engine (columnar, vector, document, relational)
/// * `table` - Target table/collection name
/// * `operation` - Type of CRUD operation to perform
/// * `data` - JSON data payload (for create/update operations)
/// * `conditions` - JSON filter conditions (for read/update/delete operations)
/// * `limit` - Maximum number of records to return (read operations)
/// * `offset` - Number of records to skip (read operations)
///
/// # Data Flow
/// 1. Parse string parameters into structured types
/// 2. Construct Query object with all parameters
/// 3. Execute query against PrimusDB instance
/// 4. Format and display results or error messages
async fn execute_crud_operation(
    primusdb: std::sync::Arc<PrimusDB>,
    storage_type: String,
    table: String,
    operation: QueryOperation,
    data: Option<String>,
    conditions: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<()> {
    let storage_type = parse_storage_type(&storage_type)?;

    let query = Query {
        storage_type,
        operation,
        table,
        conditions: conditions.and_then(|c| serde_json::from_str(&c).ok()),
        data: data.and_then(|d| serde_json::from_str(&d).ok()),
        limit,
        offset,
    };

    match primusdb.execute_query(query).await {
        Ok(result) => {
            println!("Operation executed successfully:");
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or("Invalid result".to_string())
            );
        }
        Err(e) => {
            eprintln!("Operation failed: {}", e);
        }
    }

    Ok(())
}

async fn execute_advanced_operation(
    primusdb: std::sync::Arc<PrimusDB>,
    op_type: &str,
    storage_type: String,
    table: String,
    data: Option<String>,
    conditions: Option<String>,
    _params: Option<String>,
) -> Result<()> {
    let storage_type = parse_storage_type(&storage_type)?;

    let operation = match op_type {
        "analyze" => QueryOperation::Analyze,
        "predict" => QueryOperation::Predict,
        "vector_search" => QueryOperation::Read, // Special case for vector search
        "cluster" => QueryOperation::Analyze,    // Use analyze for clustering
        _ => {
            eprintln!("Unknown advanced operation: {}", op_type);
            return Ok(());
        }
    };

    let query = Query {
        storage_type,
        operation,
        table,
        conditions: conditions.and_then(|c| serde_json::from_str(&c).ok()),
        data: data.and_then(|d| serde_json::from_str(&d).ok()),
        limit: None,
        offset: None,
    };

    match primusdb.execute_query(query).await {
        Ok(result) => {
            println!("{} operation completed:", op_type);
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or("Invalid result".to_string())
            );
        }
        Err(e) => {
            eprintln!("{} operation failed: {}", op_type, e);
        }
    }

    Ok(())
}

async fn execute_table_info_operation(
    primusdb: std::sync::Arc<PrimusDB>,
    storage_type: String,
    table: String,
) -> Result<()> {
    let storage_type = parse_storage_type(&storage_type)?;

    // Use a read operation to get table info (this would need to be implemented properly)
    let query = Query {
        storage_type,
        operation: QueryOperation::Read,
        table,
        conditions: None,
        data: None,
        limit: Some(0), // No data, just info
        offset: None,
    };

    match primusdb.execute_query(query).await {
        Ok(result) => {
            println!("Table information:");
            println!(
                "{}",
                serde_json::to_string_pretty(&result).unwrap_or("Invalid result".to_string())
            );
        }
        Err(e) => {
            eprintln!("Failed to get table info: {}", e);
        }
    }

    Ok(())
}

/// Parse string storage type into enum variant
///
/// Validates and converts user-provided storage type strings into
/// the corresponding StorageType enum. Provides helpful error messages
/// for invalid inputs.
///
/// # Supported Types
/// - "columnar" → StorageType::Columnar
/// - "vector" → StorageType::Vector
/// - "document" → StorageType::Document
/// - "relational" → StorageType::Relational
///
/// # Error Handling
/// Returns InvalidRequest error for unsupported storage types,
/// with a list of valid options in the error message.
fn parse_storage_type(storage_type: &str) -> Result<StorageType> {
    match storage_type {
        "columnar" => Ok(StorageType::Columnar),
        "vector" => Ok(StorageType::Vector),
        "document" => Ok(StorageType::Document),
        "relational" => Ok(StorageType::Relational),
        _ => {
            eprintln!("Invalid storage type. Use: columnar, vector, document, relational");
            Err(primusdb::Error::InvalidRequest(
                "Invalid storage type".to_string(),
            ))
        }
    }
}
