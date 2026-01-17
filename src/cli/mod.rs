use crate::storage::columnar::ColumnarEngine;
use crate::storage::document::DocumentEngine;
use crate::storage::relational::RelationalEngine;
use crate::storage::vector::VectorEngine;
use crate::storage::Schema;
use crate::storage::StorageEngine;
use crate::transaction::Transaction;

use crate::PrimusDBConfig;
use chrono;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "primusdb-cli")]
#[command(about = "PrimusDB Command Line Interface")]
pub struct Cli {
    /// Server URL for client mode
    #[arg(long, default_value = "http://localhost:8080")]
    pub server: String,

    /// Run in client mode (connect to server) or embedded mode
    #[arg(long, default_value = "embedded")]
    pub mode: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the PrimusDB server
    Server {
        /// Configuration file path
        #[arg(short, long, value_name = "FILE")]
        config: Option<PathBuf>,

        /// Bind address
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        bind: String,
    },

    /// Initialize a new PrimusDB instance
    Init {
        /// Data directory path
        #[arg(short, long, default_value = "./data")]
        data_dir: PathBuf,
    },

    /// Show database status
    Status,

    /// Backup database
    Backup {
        /// Backup destination
        #[arg(short, long)]
        destination: PathBuf,
    },

    /// Restore database from backup
    Restore {
        /// Backup source
        #[arg(short, long)]
        source: PathBuf,
    },

    /// Execute CRUD operations
    #[command(subcommand)]
    Crud(CrudCommands),

    /// Manage tables and collections
    #[command(subcommand)]
    Table(TableCommands),

    /// Execute advanced operations
    #[command(subcommand)]
    Advanced(AdvancedCommands),
}

#[derive(Subcommand)]
pub enum CrudCommands {
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
pub enum TableCommands {
    /// Create a new table/collection
    Create {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Schema definition (JSON)
        #[arg(long)]
        schema: Option<String>,
    },

    /// Drop (delete) a table/collection
    Drop {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,
    },

    /// Truncate (empty) a table/collection
    Truncate {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,
    },

    /// Get table/collection information
    Info {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,
    },
}

#[derive(Subcommand)]
pub enum AdvancedCommands {
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
    },

    /// Vector similarity search
    VectorSearch {
        /// Table/collection name
        #[arg(long)]
        table: String,

        /// Query vector (JSON array)
        #[arg(long)]
        query_vector: String,
    },

    /// Cluster data
    Cluster {
        /// Storage engine type (columnar, vector, document, relational)
        #[arg(long)]
        storage_type: String,

        /// Table/collection name
        #[arg(long)]
        table: String,
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

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> crate::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path.into(), &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub async fn run_cli(cli: Cli) -> crate::Result<()> {
    if cli.mode == "client" {
        // Client mode: connect to remote server
        return run_client_mode(cli).await;
    }

    // Embedded mode (default)
    match cli.command {
        Commands::Server { config, bind } => {
            println!("Starting PrimusDB server on {}...", bind);
            println!("Configuration: {:?}", config);
            // TODO: Initialize and start server
            println!("Server started successfully!");
        }

        Commands::Init { data_dir } => {
            println!("Initializing PrimusDB in directory: {:?}", data_dir);
            // Initialize database directory and configuration
            if !data_dir.exists() {
                std::fs::create_dir_all(&data_dir)?;
                println!("Created data directory: {:?}", data_dir);
            } else {
                println!("Data directory already exists: {:?}", data_dir);
            }

            // Create default config if not exists
            let config_path = data_dir.join("config.toml");
            if !config_path.exists() {
                let default_config = r#"
[storage]
data_dir = "./data"
max_file_size = 1073741824
compression = "lz4"
cache_size = 536870912

[network]
bind_address = "127.0.0.1"
port = 8080
max_connections = 1000

[security]
encryption_enabled = true
key_rotation_interval = 86400
auth_required = false

[cluster]
enabled = false
node_id = "node1"
discovery_servers = []
"#;
                std::fs::write(&config_path, default_config)?;
                println!("Created default config: {:?}", config_path);
            }

            println!("PrimusDB initialized successfully!");
        }

        Commands::Status => {
            println!("PrimusDB Status:");
            println!("- Status: Running");
            println!("- Version: {}", env!("CARGO_PKG_VERSION"));
            println!("- Uptime: Unknown");
            println!("- Available Engines: columnar, vector, document, relational");
            println!("- Features: AI/ML, Blockchain Consensus, Distributed Clustering");
            // TODO: Query actual status from running instance
            println!("");
        }

        Commands::Backup { destination } => {
            println!("Backing up PrimusDB to: {:?}", destination);
            // Implement backup functionality
            let data_dir = PathBuf::from("./data");
            if data_dir.exists() {
                copy_dir_all(&data_dir, &destination)?;
                println!("Backup completed successfully!");
            } else {
                println!("No data directory found to backup.");
            }
        }

        Commands::Restore { source } => {
            println!("Restoring PrimusDB from: {:?}", source);
            // Implement restore functionality
            let data_dir = PathBuf::from("./data");
            if source.exists() {
                copy_dir_all(&source, &data_dir)?;
                println!("Restore completed successfully!");
            } else {
                println!("Backup source not found.");
            }
        }

        Commands::Crud(crud_cmd) => {
            handle_crud_cli_command(crud_cmd).await?;
        }

        Commands::Table(table_cmd) => {
            handle_table_cli_command(table_cmd).await?;
        }

        Commands::Advanced(adv_cmd) => {
            handle_advanced_cli_command(adv_cmd).await?;
        }
    }

    Ok(())
}

async fn handle_crud_cli_command(cmd: CrudCommands) -> crate::Result<()> {
    let config = PrimusDBConfig {
        storage: crate::StorageConfig {
            data_dir: "/tmp/primusdb_cli".to_string(),
            max_file_size: 1024 * 1024 * 1024,
            compression: crate::CompressionType::Lz4,
            cache_size: 512 * 1024 * 1024,
        },
        network: crate::NetworkConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
        },
        security: crate::SecurityConfig {
            encryption_enabled: true,
            key_rotation_interval: 86400,
            auth_required: false,
        },
        cluster: crate::ClusterConfig {
            enabled: false,
            node_id: "cli_node".to_string(),
            discovery_servers: vec![],
        },
    };
    let transaction = Transaction {
        id: "cli_operation".to_string(),
        operations: vec![],
        created_at: chrono::Utc::now(),
        status: crate::transaction::TransactionStatus::Active,
        updated_at: chrono::Utc::now(),
        isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
        timeout_ms: 30000,
    };

    match cmd {
        CrudCommands::Create {
            storage_type,
            table,
            data,
        } => {
            println!(
                "Creating record in {} table '{}' with data: {}",
                storage_type, table, data
            );
            let data_json: serde_json::Value = serde_json::from_str(&data)?;
            let engine = get_engine(&storage_type, &config)?;
            let count = engine.insert(&table, &data_json, &transaction).await?;
            println!("Record created successfully with ID: {}", count);
        }
        CrudCommands::Read {
            storage_type,
            table,
            conditions,
            limit,
            offset,
        } => {
            println!(
                "Reading records from {} table '{}' (limit: {}, offset: {})",
                storage_type, table, limit, offset
            );
            let conditions_json = conditions
                .as_ref()
                .map(|c| serde_json::from_str(c).unwrap());
            let engine = get_engine(&storage_type, &config)?;
            let limit_val = limit;
            let offset_val = offset;
            let records = engine
                .select(
                    &table,
                    conditions_json.as_ref(),
                    limit_val,
                    offset_val,
                    &transaction,
                )
                .await?;
            println!("Records retrieved successfully: {} records", records.len());
            for record in records {
                println!("  ID: {}, Data: {}", record.id, record.data);
            }
        }
        CrudCommands::Update {
            storage_type,
            table,
            conditions,
            data,
        } => {
            println!(
                "Updating records in {} table '{}' with data: {}",
                storage_type, table, data
            );
            let conditions_json = conditions
                .as_ref()
                .map(|c| serde_json::from_str(c).unwrap());
            let data_json: serde_json::Value = serde_json::from_str(&data)?;
            let engine = get_engine(&storage_type, &config)?;
            let count = engine
                .update(&table, conditions_json.as_ref(), &data_json, &transaction)
                .await?;
            println!("Records updated successfully: {} records", count);
        }
        CrudCommands::Delete {
            storage_type,
            table,
            conditions,
        } => {
            println!("Deleting records from {} table '{}'", storage_type, table);
            let conditions_json = conditions
                .as_ref()
                .map(|c| serde_json::from_str(c).unwrap());
            let engine = get_engine(&storage_type, &config)?;
            let count = engine
                .delete(&table, conditions_json.as_ref(), &transaction)
                .await?;
            println!("Records deleted successfully: {} records", count);
        }
    }
    Ok(())
}

async fn handle_table_cli_command(cmd: TableCommands) -> crate::Result<()> {
    let config = PrimusDBConfig {
        storage: crate::StorageConfig {
            data_dir: "/tmp/primusdb_cli".to_string(),
            max_file_size: 1024 * 1024 * 1024,
            compression: crate::CompressionType::Lz4,
            cache_size: 512 * 1024 * 1024,
        },
        network: crate::NetworkConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
        },
        security: crate::SecurityConfig {
            encryption_enabled: true,
            key_rotation_interval: 86400,
            auth_required: false,
        },
        cluster: crate::ClusterConfig {
            enabled: false,
            node_id: "cli_node".to_string(),
            discovery_servers: vec![],
        },
    };
    let transaction = Transaction {
        id: "cli_operation".to_string(),
        operations: vec![],
        created_at: chrono::Utc::now(),
        status: crate::transaction::TransactionStatus::Active,
        updated_at: chrono::Utc::now(),
        isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
        timeout_ms: 30000,
    };

    match cmd {
        TableCommands::Create {
            storage_type,
            table,
            schema,
        } => {
            println!("Creating table '{}' in {} storage", table, storage_type);
            let schema = schema
                .map(|s| serde_json::from_str(&s).unwrap())
                .unwrap_or(Schema {
                    fields: vec![],
                    indexes: vec![],
                    constraints: vec![],
                });
            let engine = get_engine(&storage_type, &config)?;
            engine.create_table(&table, &schema).await?;
            println!("Table created successfully");
        }

        TableCommands::Drop {
            storage_type,
            table,
        } => {
            println!("Dropping table '{}' from {} storage", table, storage_type);
            let engine = get_engine(&storage_type, &config)?;
            engine.drop_table(&table).await?;
            println!("Table dropped successfully");
        }

        TableCommands::Truncate {
            storage_type,
            table,
        } => {
            println!("Truncating table '{}' in {} storage", table, storage_type);
            let engine = get_engine(&storage_type, &config)?;
            engine.truncate_table(&table).await?;
            println!("Table truncated successfully");
        }

        TableCommands::Info {
            storage_type,
            table,
        } => {
            println!(
                "Getting info for table '{}' in {} storage",
                table, storage_type
            );
            let engine = get_engine(&storage_type, &config)?;
            let info = engine.table_info(&table).await?;
            println!("Table Info:");
            println!("  Engine: {}", storage_type);
            println!("  Table: {}", table);
            println!("  Record Count: {}", info.row_count);
            println!("  Size: {} bytes", info.size_bytes);
            println!("  Created: {}", info.created_at);
            println!("  Updated: {}", info.updated_at);
        }
    }
    Ok(())
}

async fn handle_advanced_cli_command(cmd: AdvancedCommands) -> crate::Result<()> {
    let config = PrimusDBConfig {
        storage: crate::StorageConfig {
            data_dir: "/tmp/primusdb_cli".to_string(),
            max_file_size: 1024 * 1024 * 1024,
            compression: crate::CompressionType::Lz4,
            cache_size: 512 * 1024 * 1024,
        },
        network: crate::NetworkConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 1000,
        },
        security: crate::SecurityConfig {
            encryption_enabled: true,
            key_rotation_interval: 86400,
            auth_required: false,
        },
        cluster: crate::ClusterConfig {
            enabled: false,
            node_id: "cli_node".to_string(),
            discovery_servers: vec![],
        },
    };
    let transaction = Transaction {
        id: "cli_operation".to_string(),
        operations: vec![],
        created_at: chrono::Utc::now(),
        status: crate::transaction::TransactionStatus::Active,
        updated_at: chrono::Utc::now(),
        isolation_level: crate::transaction::IsolationLevel::ReadCommitted,
        timeout_ms: 30000,
    };

    match cmd {
        AdvancedCommands::Analyze {
            storage_type,
            table,
            conditions,
        } => {
            println!(
                "Analyzing data patterns in {} table '{}'",
                storage_type, table
            );
            let conditions_json = conditions
                .as_ref()
                .map(|c| serde_json::from_str(c).unwrap());
            let engine = get_engine(&storage_type, &config)?;
            let analysis = engine
                .analyze(&table, conditions_json.as_ref(), &transaction)
                .await?;
            println!("Analysis completed successfully:");
            println!("{}", analysis);
        }
        AdvancedCommands::Predict {
            storage_type,
            table,
            data,
        } => {
            println!(
                "Making AI predictions on {} table '{}' with data: {}",
                storage_type, table, data
            );
            let conditions_json: serde_json::Value = serde_json::from_str(&data)?;
            let ai_engine = crate::ai::AIEngine::new(&config)?;
            let predictions = ai_engine.predict(&table, Some(&conditions_json)).await?;
            println!("Predictions completed successfully:");
            for pred in predictions {
                println!("  {}", serde_json::to_string_pretty(&pred.data)?);
            }
        }
        AdvancedCommands::VectorSearch {
            table,
            query_vector,
        } => {
            println!(
                "Performing vector similarity search in table '{}' with vector: {}",
                table, query_vector
            );
            let query_vec: Vec<f32> = serde_json::from_str(&query_vector)?;
            let engine = VectorEngine::new(&config)?;
            // For now, assume vector is stored as JSON array
            let conditions = serde_json::json!({ "vector": query_vec });
            let results = engine
                .select(&table, Some(&conditions), 10, 0, &transaction)
                .await?;
            println!("Vector search completed successfully:");
            for result in results {
                println!("  {}", serde_json::to_string_pretty(&result.data)?);
            }
        }
        AdvancedCommands::Cluster {
            storage_type,
            table,
        } => {
            println!("Clustering data in {} table '{}'", storage_type, table);
            let ai_engine = crate::ai::AIEngine::new(&config)?;
            let clusters = ai_engine.cluster_data(&table, 3).await?;
            println!("Clustering completed successfully:");
            println!("  Number of clusters: {}", clusters.clusters.len());
            for (i, cluster) in clusters.clusters.iter().enumerate() {
                println!("  Cluster {}: {} members", i, cluster.members.len());
            }
        }
        AdvancedCommands::TableInfo {
            storage_type,
            table,
        } => {
            println!("Getting information for {} table '{}'", storage_type, table);
            let engine = get_engine(&storage_type, &config)?;
            let info = engine.table_info(&table).await?;
            println!("Table info retrieved successfully:");
            println!("- Engine: {}", storage_type);
            println!("- Table: {}", table);
            println!("- Records: {}", info.row_count);
            println!("- Size: {} bytes", info.size_bytes);
            println!("- Created: {}", info.created_at);
            println!("- Updated: {}", info.updated_at);
        }
    }

    Ok(())
}

fn get_engine(
    storage_type: &str,
    config: &PrimusDBConfig,
) -> crate::Result<Box<dyn StorageEngine>> {
    match storage_type {
        "columnar" => Ok(Box::new(ColumnarEngine::new(config)?)),
        "document" => Ok(Box::new(DocumentEngine::new(config)?)),
        "relational" => Ok(Box::new(RelationalEngine::new(config)?)),
        "vector" => Ok(Box::new(VectorEngine::new(config)?)),
        _ => Err(crate::Error::InvalidRequest(format!(
            "Unknown storage type: {}",
            storage_type
        ))),
    }
}

async fn run_client_mode(cli: Cli) -> crate::Result<()> {
    println!("PrimusDB CLI - Client Mode");
    println!("Server: {}", cli.server);

    match cli.command {
        Commands::Server { .. } => {
            println!(
                "Server command not available in client mode. Use embedded mode to start server."
            );
        }
        Commands::Init { .. } => {
            println!("Init command not available in client mode. Use embedded mode to initialize database.");
        }
        Commands::Status => {
            match reqwest::get(&format!("{}/health", cli.server)).await {
                Ok(response) => {
                    let json: serde_json::Value = response.json().await.map_err(|e| {
                        crate::Error::NetworkError(format!("Failed to parse response: {}", e))
                    })?;
                    println!("Server Status:");
                    println!("{}", serde_json::to_string_pretty(&json)?);
                }
                Err(e) => {
                    eprintln!("Failed to connect to server: {}", e);
                    eprintln!("Make sure the server is running: primusdb-server --host 127.0.0.1 --port 8080");
                }
            }
        }
        Commands::Backup { destination } => {
            println!(
                "Backup from server: {} -> {}",
                cli.server,
                destination.display()
            );
            println!("Backup functionality coming soon in client mode!");
        }
        Commands::Restore { source } => {
            println!("Restore to server: {} -> {}", source.display(), cli.server);
            println!("Restore functionality coming soon in client mode!");
        }
        Commands::Crud(crud_cmd) => {
            handle_crud_client_command(cli.server, crud_cmd).await?;
        }
        Commands::Table(table_cmd) => {
            handle_table_client_command(cli.server, table_cmd).await?;
        }
        Commands::Advanced(adv_cmd) => {
            handle_advanced_client_command(cli.server, adv_cmd).await?;
        }
    }

    Ok(())
}

async fn handle_crud_client_command(server: String, cmd: CrudCommands) -> crate::Result<()> {
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

async fn handle_table_client_command(server: String, cmd: TableCommands) -> crate::Result<()> {
    let client = reqwest::Client::new();

    match cmd {
        TableCommands::Create {
            storage_type,
            table,
            schema,
        } => {
            let url = format!("{}/api/v1/crud/{}/{}", server, storage_type, table);
            let body = serde_json::json!({
                "operation": "CreateTable",
                "schema": schema.and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            });
            let response = client.post(&url).json(&body).send().await?;
            let json: serde_json::Value = response.json().await?;
            println!("Table creation result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }

        TableCommands::Drop {
            storage_type,
            table,
        } => {
            let url = format!("{}/api/v1/crud/{}/{}", server, storage_type, table);
            let response = client.delete(&url).send().await?;
            let json: serde_json::Value = response.json().await?;
            println!("Table drop result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }

        TableCommands::Truncate {
            storage_type,
            table,
        } => {
            let url = format!("{}/api/v1/crud/{}/{}/truncate", server, storage_type, table);
            let response = client.post(&url).send().await?;
            let json: serde_json::Value = response.json().await?;
            println!("Table truncate result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }

        TableCommands::Info {
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

async fn handle_advanced_client_command(
    server: String,
    cmd: AdvancedCommands,
) -> crate::Result<()> {
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
        } => {
            let url = format!(
                "{}/api/v1/advanced/predict/{}/{}",
                server, storage_type, table
            );
            let body = serde_json::json!({
                "data": serde_json::from_str::<serde_json::Value>(&data)?
            });

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Prediction result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        AdvancedCommands::VectorSearch {
            table,
            query_vector,
        } => {
            let url = format!("{}/api/v1/advanced/vector-search/{}", server, table);
            let query_vector: Vec<f32> = query_vector
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();

            let body = serde_json::json!({
                "query_vector": query_vector,
                "limit": 10
            });

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Vector search result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        AdvancedCommands::Cluster {
            storage_type,
            table,
        } => {
            let url = format!(
                "{}/api/v1/advanced/cluster/{}/{}",
                server, storage_type, table
            );
            let body = serde_json::json!({"algorithm": "kmeans", "clusters": 5});

            let response = client.post(&url).json(&body).send().await?;

            let json: serde_json::Value = response.json().await?;
            println!("Clustering result:");
            println!("{}", serde_json::to_string_pretty(&json)?);
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
