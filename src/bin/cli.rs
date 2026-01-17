/*!
# PrimusDB CLI Binary - Command Line Interface

This is the main entry point for the PrimusDB command-line interface.
It provides both embedded database operations and client-server connectivity.

## Features

- **Embedded Mode**: Run PrimusDB locally without external server
- **Client Mode**: Connect to remote PrimusDB servers
- **CRUD Operations**: Full create, read, update, delete functionality
- **Advanced Analytics**: AI/ML operations and data analysis
- **Interactive Help**: Comprehensive command documentation

## Usage

### Basic Operations
```bash
# Show help
primusdb --help

# Embedded mode operations
primusdb crud create --storage-type document --table users --data '{"name": "Alice"}'
primusdb crud read --storage-type document --table users --limit 10

# Client mode
primusdb --mode client --server http://localhost:8080 info
```

### Advanced Features
```bash
# AI predictions
primusdb advanced predict --storage-type columnar --table sales \
  --data '{"quarter": "Q1"}' --prediction-type revenue

# Vector search
primusdb advanced vector-search --table embeddings \
  --query-vector "0.1,0.2,0.3" --limit 5

# Data clustering
primusdb advanced cluster --storage-type document --table customers
```

## Architecture

This binary serves as a thin wrapper around the core CLI logic,
providing command-line argument parsing and async runtime initialization.
The actual command processing is handled by the `primusdb::cli` module.
*/

use clap::Parser;
use primusdb::cli::{run_cli, Cli};

/// Main entry point for PrimusDB CLI
///
/// Initializes tracing for logging, parses command-line arguments,
/// and delegates execution to the core CLI logic.
#[tokio::main]
async fn main() -> primusdb::Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt::init();

    // Parse command-line arguments using clap
    let cli = Cli::parse();

    // Execute CLI command and return result
    run_cli(cli).await
}
