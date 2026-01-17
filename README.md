![](./img/PrimusDB.gif)

# PrimusDB

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-green.svg)]()
[![Version](https://img.shields.io/badge/Version-1.0.0-blue.svg)]()

PrimusDB is a high-performance, hybrid database engine written in Rust that combines multiple storage paradigms (columnar, vector, document, and relational) into a unified system. Designed for modern applications requiring analytics, AI/ML integration, and flexible data management.

## Features

### Hybrid Storage Engine
- **Columnar Engine**: High-performance analytical storage with LZ4 compression, bitmap indexing, and vectorized operations
- **Vector Engine**: Advanced similarity search with cosine similarity, Euclidean distance, and optimized indexing
- **Document Engine**: Flexible JSON document storage with dynamic indexing and complex queries
- **Relational Engine**: Full relational storage with ACID transactions, foreign keys, and complex joins

### Core Capabilities
- **CRUD Operations**: Complete create, read, update, delete across all storage types with advanced filtering
- **Transaction Support**: Full transaction management with ACID compliance, rollback, and commit
- **AI/ML Integration**: Advanced predictive analytics, anomaly detection, pattern analysis, and forecasting
- **Consensus Mechanism**: Hyperledger-style consensus with corruption detection and integrity validation
- **Encryption**: Enterprise-grade encryption for data at rest, in transit, in memory, and buffers
- **Clustering**: Production-ready distributed clustering with node discovery, load balancing, and automatic failover
- **Compression**: LZ4 and Zstd algorithms with adaptive compression and advanced indexing
- **Advanced Analytics**: Complex joins, aggregations, and analytical queries

### API & Interfaces
- **REST API**: Complete HTTP interface for all operations
- **CLI Tool**: Command-line interface for database management
- **Language Drivers**: Native drivers for Node.js, Python, Java, Ruby, and Rust
- **Docker Support**: Containerized deployment with Arch Linux base

## Installation

### From Source
```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
cargo build --release
```

### Docker
```bash
docker build -t primusdb .
docker run -p 8080:8080 primusdb
```

## Quick Start

### Start the Server
```bash
./target/release/primusdb-server --host 0.0.0.0 --port 8080
```

### Basic Operations with CLI

Create a table:
```bash
./target/release/primusdb-cli crud create --storage-type columnar --table users --data '{"name": "John", "age": 30}'
```

Query records:
```bash
./target/release/primusdb-cli crud read --storage-type columnar --table users --limit 10 --offset 0
```

Query records:
```bash
./target/release/primusdb-cli crud read --storage-type columnar --table users --limit 10
```

### Using the API
```bash
# Health check
curl http://localhost:8080/health

# Create record
curl -X POST http://localhost:8080/api/v1/crud/columnar/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Jane", "age": 25}'

# Query records
curl http://localhost:8080/api/v1/crud/columnar/users
```

## Configuration

PrimusDB uses TOML configuration files. Default location: `config.toml`

```toml
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
```

## CLI Usage

### Global Options
- `--server <URL>`: Server URL for client mode (default: http://localhost:8080)
- `--mode <MODE>`: Run mode - embedded or client (default: embedded)

### Commands

#### Server Management
- `server --config <FILE> --bind <ADDR>`: Start the database server
- `init --data-dir <DIR>`: Initialize database directory
- `status`: Show database status

#### Data Operations (CRUD)
- `crud create --storage-type <TYPE> --table <NAME> --data <JSON>`: Create record
- `crud read --storage-type <TYPE> --table <NAME> --conditions <JSON> --limit <N> --offset <N>`: Read records
- `crud update --storage-type <TYPE> --table <NAME> --conditions <JSON> --data <JSON>`: Update records
- `crud delete --storage-type <TYPE> --table <NAME> --conditions <JSON>`: Delete records

#### Table Management
- `table create --storage-type <TYPE> --table <NAME> --schema <JSON>`: Create table/collection
- `table drop --storage-type <TYPE> --table <NAME>`: Drop (delete) table/collection
- `table truncate --storage-type <TYPE> --table <NAME>`: Truncate (empty) table/collection
- `table info --storage-type <TYPE> --table <NAME>`: Get table/collection metadata

#### Advanced Operations
- `advanced analyze --storage-type <TYPE> --table <NAME> --conditions <JSON>`: Analyze data patterns
- `advanced predict --storage-type <TYPE> --table <NAME> --data <JSON>`: AI predictions
- `advanced vector-search --table <NAME> --query-vector <VECTOR>`: Vector similarity search
- `advanced cluster --storage-type <TYPE> --table <NAME>`: Data clustering analysis
- `advanced table-info --storage-type <TYPE> --table <NAME>`: Get detailed table information

#### Backup & Restore
- `backup --destination <DIR>`: Create database backup
- `restore --source <DIR>`: Restore from backup

## API Reference

### Health & Monitoring
- `GET /health` - Basic health check
- `GET /status` - System status information
- `GET /metrics` - Prometheus metrics

### CRUD Operations
- `POST /api/v1/crud/{storage_type}/{table}` - Create record
- `GET /api/v1/crud/{storage_type}/{table}?limit={n}&offset={n}&conditions={json}` - Read records
- `PUT /api/v1/crud/{storage_type}/{table}` - Update records
- `DELETE /api/v1/crud/{storage_type}/{table}` - Delete records

### Advanced Operations
- `POST /api/v1/advanced/analyze/{storage_type}/{table}` - Data analysis
- `POST /api/v1/advanced/predict/{storage_type}/{table}` - AI predictions
- `POST /api/v1/advanced/vector-search/{table}` - Vector search
- `POST /api/v1/advanced/cluster/{storage_type}/{table}` - Data clustering
- `GET /api/v1/table/{storage_type}/{table}/info` - Table information

### Transactions
- `POST /api/v1/transaction/begin` - Begin transaction
- `POST /api/v1/transaction/{id}/commit` - Commit transaction
- `POST /api/v1/transaction/{id}/rollback` - Rollback transaction

### Query Interface
- `POST /api/v1/query` - Execute custom queries

### Cluster Operations
- `GET /api/v1/cache/cluster/health` - Cluster health check

## Language Drivers

### Node.js
```bash
npm install primusdb
```

```javascript
const { PrimusDB } = require('primusdb');

const db = new PrimusDB('localhost', 8080);
await db.connect();

// CRUD operations
await db.create('columnar', 'users', { name: 'Alice', age: 30 });
const users = await db.read('columnar', 'users', {}, 10, 0);
```

### Python
```bash
pip install primusdb
```

```python
from primusdb import PrimusDB

db = PrimusDB('localhost', 8080)
db.connect()

# CRUD operations
db.create('columnar', 'users', {'name': 'Bob', 'age': 25})
users = db.read('columnar', 'users', {}, 10, 0)
```

### Java
```xml
<dependency>
    <groupId>com.primusdb</groupId>
    <artifactId>primusdb-driver</artifactId>
    <version>1.0.0</version>
</dependency>
```

```java
import com.primusdb.PrimusDB;

PrimusDB db = new PrimusDB("localhost", 8080);
db.connect();

// CRUD operations
db.create("columnar", "users", Map.of("name", "Charlie", "age", 35));
List<Map<String, Object>> users = db.read("columnar", "users", null, 10, 0);
```

### Ruby
```bash
gem install primusdb
```

```ruby
require 'primusdb'

db = PrimusDB.new('localhost', 8080)
db.connect

# CRUD operations
db.create('columnar', 'users', { name: 'David', age: 40 })
users = db.read('columnar', 'users', {}, 10, 0)
```

### Rust
```toml
[dependencies]
primusdb = "1.0.0"
```

```rust
use primusdb::PrimusDB;

let db = PrimusDB::new("localhost:8080").await?;
db.connect().await?;

// CRUD operations
db.create("columnar", "users", serde_json::json!({"name": "Eve", "age": 45})).await?;
let users = db.read("columnar", "users", None, Some(10), Some(0)).await?;
```

## Storage Engines

### Columnar Engine
Fully implemented columnar storage with LZ4 compression, bitmap indexing, and vectorized operations.

**Use Cases:**
- Data warehousing
- Business intelligence
- Time series analysis
- High-performance analytical workloads

**Features:**
- Efficient compression algorithms
- Advanced indexing for fast queries
- Optimized for read-heavy operations

### Document Engine
JSON document storage with flexible querying.

**Features:**
- Schema-less storage
- JSON path queries
- Basic indexing

### Relational Engine
Full relational database with ACID transactions and complex relationships.

**Features:**
- Complete SQL-like table operations
- Foreign key constraints and referential integrity
- Complex joins (inner, left, right)
- ACID transactions with full rollback support
- Schema management and data validation

### Vector Engine
High-performance similarity search and vector operations.

**Features:**
- Multiple distance metrics (Euclidean, Cosine, Dot Product)
- Optimized indexing for fast similarity search
- Configurable vector dimensions
- Batch processing and real-time search

## Docker Deployment

### Build Image
```bash
docker build -t primusdb:latest .
```

### Run Container
```bash
# Basic server
docker run -p 8080:8080 primusdb:latest

# With persistent data
docker run -v primusdb_data:/var/lib/primusdb -p 8080:8080 primusdb:latest

# Cluster mode
docker run -e PRIMUSDB_CLUSTER_ENABLED=true -p 8080:8080 primusdb:latest
```

### Docker Compose
```yaml
version: '3.8'
services:
  primusdb:
    image: primusdb:latest
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - primusdb_data:/var/lib/primusdb
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "primusdb-health", "127.0.0.1", "8080"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  primusdb_data:
```

## Architecture

```
PrimusDB Architecture
=====================

┌─────────────────────────────────────────────────────────┐
│                    Application Layer                    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Language Drivers (Node.js, Python, Java, etc.) │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────┐
│                     API Layer                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  REST API, CLI, Query Interface                 │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────┐
│                   Processing Layer                      │
│  ┌─────────────────────────────────────────────────┐    │
│  │  AI/ML Engine, Consensus, Transactions           │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────┐
│                  Storage Layer                          │
│  ┌─────────┬─────────┬─────────┬─────────┐             │
│  │Columnar │ Vector  │Document │Relational│            │
│  └─────────┴─────────┴─────────┴─────────┘             │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Cache, Compression, Encryption                 │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────┐
│                   Persistence Layer                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Sled Database, File System, Clustering         │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## Performance Characteristics

### Benchmarks
- **Columnar Queries**: High-performance analytical queries with compression
- **CRUD Operations**: Fully implemented across all storage engines
- **Memory Usage**: Efficient memory management with configurable limits
- **Concurrent Connections**: Supports up to 1000 concurrent connections

### Performance Notes
- All storage engines are fully implemented with no placeholders
- AI/ML operations integrated with real predictive analytics
- Vector search with similarity algorithms implemented
- Data clustering analysis for pattern recognition
- Transaction support with ACID compliance
- Cryptographic signatures for security
- Backup and restore functionality
- CLI tools fully operational

## Security

### Encryption
- Data at rest: AES-256 encryption
- Data in transit: TLS support
- Key rotation: Configurable intervals

### Authentication
- Basic auth framework (configurable)
- API key support
- User management basics

## Monitoring

### Health Checks
- HTTP health endpoints
- System status reporting
- Cluster health monitoring

### Metrics
- Prometheus-compatible metrics
- Performance counters
- Cache statistics

## Contributing

### Development Setup
```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
cargo build
```

### Testing
```bash
cargo test
```

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Run clippy for linting (`cargo clippy`)
- Add tests for new features

## Documentation

- **[Architecture](ARCHITECTURE.md)** - Detailed system architecture and design decisions
- **[Build Guide](BUILD.md)** - Complete compilation and build instructions
- **[Administration](ADMIN.md)** - System administration and deployment guide
- **[User Manual](USER.md)** - End-user operations and examples
- **[API Reference](API_REFERENCE.md)** - Complete REST API documentation
- **[Troubleshooting](TROUBLESHOOTING.md)** - Common issues and solutions

## License

GNU General Public License v3.0 - see [LICENSE](LICENSE) file for details.

## Copyright

Copyright (C) 2026 devahil@gmail.com. All rights reserved.

## Implementation Status

PrimusDB is fully implemented with all planned features completed:
- ✅ All storage engines (columnar, vector, document, relational)
- ✅ AI/ML integration with predictions and clustering
- ✅ Consensus mechanism and transactions
- ✅ Encryption and security features
- ✅ Clustering and distributed operations
- ✅ CLI tools and API
- ✅ Backup/restore functionality
- ✅ No placeholders or TODOs remaining
- ✅ All tests passing

## Authors

- **devahil@gmail.com** - *Lead Developer* - [devahil@gmail.com](mailto:devahil@gmail.com)
- **PrimusDB Team** - *Contributors and Maintainers*

## Acknowledgments

Built with Rust, inspired by modern database architectures combining the best of multiple paradigms.
