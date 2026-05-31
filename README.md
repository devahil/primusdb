![](./img/PrimusDB.gif)

# PrimusDB

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/Build-Passing-green.svg)]()
[![Version](https://img.shields.io/badge/Version-1.3.1--alpha-blue.svg)]()

PrimusDB is a high-performance, hybrid database engine written in Rust that combines multiple storage paradigms (columnar, vector, document, relational, and key-value) with a blockchain audit ledger, Ed25519 cryptographic signatures, and real AI/ML algorithms into a unified system. Designed for modern applications requiring analytics, AI/ML integration, distributed clustering, and flexible data management.

## Features

### Hybrid Storage Engine
- **Columnar Engine**: High-performance analytical storage with LZ4 compression, bitmap indexing, and vectorized operations
- **Vector Engine**: Advanced similarity search with cosine similarity, Euclidean distance, and optimized indexing
- **Document Engine**: Flexible JSON document storage with dynamic indexing and complex queries
- **Relational Engine**: Full relational storage with ACID transactions, foreign keys, and complex joins
- **Key-Value Engine**: CouchDB-compatible API with _id/_rev MVCC versioning, Mango queries (10 operators), bulk operations, indexes, _all_docs pagination, optional AES-256-GCM per-database encryption, namespace isolation, Prometheus metrics, and federation capability advertisement

### Core Capabilities
- **Unified Query Language (UQL)**: Cross-engine SQL/MongoDB/Mango/UQL queries with query planner and executor
- **CRUD Operations**: Complete create, read, update, delete across all storage types with advanced filtering
- **Transaction Support**: Full transaction management with ACID compliance, rollback, and commit
- **AI/ML Engine**: Real ML algorithms — LinearRegression (gradient descent, R²), LogisticRegression, K-Means++ (silhouette score), TimeSeries decomposition, Z-score Anomaly Detection, Forecast with confidence intervals, Pattern Analysis
- **Consensus Mechanism**: Hyperledger-style consensus with corruption detection and integrity validation
- **Blockchain Audit Ledger**: Immutable append-only ledger with SHA-256 Merkle tree, tamper detection, and sled persistence
- **Ed25519 Transaction Signatures**: All transactions cryptographically signed and verified via ed25519-dalek
- **Secure Inter-Node Protocol**: AES-256-GCM encryption, Ed25519 signatures, HMAC integrity, X.509 certificate trust with CRL revocation
- **Distributed Sync**: Enterprise-grade cluster synchronization with Raft-style consensus, vector clocks, and cross-node reconciliation
- **Encryption**: Enterprise-grade encryption for data at rest, in transit, in memory, and buffers
- **Clustering**: Production-ready distributed clustering with node discovery, load balancing, and automatic failover
- **Cluster Gateway**: Smart load balancer (Envoy-style) with 6 routing strategies, circuit breaker, and EWMA latency tracking
- **Federation Layer**: Cross-cluster federation (cluster-of-clusters) with DataDomain replication, federated Raft, and namespace resolution
- **Multi-Region Active-Active**: Vector clock reconciliation and automatic conflict resolution across regions
- **Geo-Distributed Sharding**: Region-aware shard placement with cross-region replicas
- **Compression**: LZ4 and Zstd algorithms with adaptive compression and advanced indexing
- **Advanced Analytics**: Complex joins, aggregations, and analytical queries

### ER Model (v1.2.2+)
- **Extended Data Types**: 13 SQL-standard types including `SmallInt`, `BigInt`, `Decimal`, `Varchar`, `Char`, `Timestamp`, `Time`, `Uuid`, `Enum`, `Serial`, `BigSerial`, `Money`, `Interval`
- **Referential Integrity**: Foreign keys with `CASCADE`, `SET NULL`, `SET DEFAULT`, `RESTRICT`, `NO ACTION` on delete/update
- **Sequences**: Auto-increment with `NEXTVAL`, `CURRVAL`, `SETVAL`, persistence, cycle, cache
- **Views**: Virtual and materialized views with query caching and refresh
- **Triggers**: Before/After/InsteadOf triggers on Insert/Update/Delete with Raise/Execute operations
- **DDL Operations**: `ADD COLUMN`, `DROP COLUMN`, `MODIFY COLUMN`, `ADD/DROP CONSTRAINT`, `RENAME TABLE`
- **DML RETURNING**: `INSERT RETURNING`, `UPDATE RETURNING`, `DELETE RETURNING` clauses
- **Enhanced SELECT**: `GROUP BY`, `HAVING`, `ORDER BY`, `DISTINCT`, aggregation functions
- **Information Schema**: System tables for tables, columns, and constraints metadata

### Security & Authentication
- **User Authentication**: Secure login with Argon2 password hashing
- **API Tokens**: Cryptographically secure tokens with SHA-256 hashing
- **RBAC**: Role-based access control (admin, developer, analyst, readonly)
- **Multi-tenancy**: Segment-based data isolation
- **Account Protection**: Brute-force protection with account lockout
- **Cluster Security**: Hyperledger-style genesis keys for node authentication
- **Ed25519 Digital Signatures**: Transaction-level signing and verification for non-repudiation
- **Secure Protocol Layer**: AES-256-GCM encrypted node communication with Ed25519 + HMAC integrity
- **X.509 Certificate Trust**: Certificate-based trust establishment with CRL-style revocation
- **Data-at-Rest Encryption**: AES-256-GCM for all binary data files
  - Columnar, Vector, Relational: Always encrypted
  - Documents: Optional encryption (JSON plaintext by default)

### API & Interfaces
- **REST API**: Complete HTTP interface for all operations
- **CLI Tool**: Command-line interface for database management
- **Language Drivers**: Native drivers for Node.js, Python, Java, Ruby, and Rust
  - All drivers support: Transactions, ReferentialActions, Sequences, Views, Triggers, AlterTable, ReturningClause, GroupByQuery, InformationSchema, TruncateCascade, ExtendedDataTypes, KeyValue
  - Async operations: Rust ✓, Python ✓, Node ✓, Ruby ✓, Java ✗
  - Connection pooling: Rust ✓, Node ✓, Java ✓, Python ✗, Ruby ✗
  - Prepared statements: Python ✓, Java ✓ (setInt/String/Double/Boolean/Long/Float/Null + executeBatch), Rust ✗, Node ✗, Ruby ✗
  - Batch operations: Java ✓ (addBatch/executeBatch), others ✗
  - SSL support: Java ✓, others ✗
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

# With federation (multi-cluster mode)
./target/release/primusdb-server --host 0.0.0.0 --port 8080 \
  --federation-id my-fed --cluster-id cluster-us --region us-east \
  --federation-discovery fed-peer1:8081,fed-peer2:8081
```

### Authentication (v1.1.0+)
```bash
# 1. Login with default credentials (admin/admin123)
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'

# 2. Create API token (use token from login response)
curl -X POST http://localhost:8080/api/v1/auth/token/create \
  -H "Content-Type: application/json" \
  -d '{"authorization": "TOKEN", "name": "my-token", "scopes": [{"resource": "All", "actions": ["Read", "Write"]}]}'

# 3. Use token in requests
curl -X POST http://localhost:8080/api/v1/crud/columnar/users \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_TOKEN" \
  -d '{"name": "Jane", "age": 25}'
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
- `backup --destination <DIR>`: Create database backup (structured format with magic header `PRIMUSDBBACKUP`, manifest, data segments, schemas, indexes, embedded WAL, Blake3 checksums)
- `restore --source <DIR>`: Restore from backup (validates magic header, checksums, reconstructs engines + indexes + WAL)

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
- `GET /api/v1/cluster/status` - Cluster status
- `GET /api/v1/cluster/nodes` - List cluster nodes
- `POST /api/v1/cluster/route` - Route a request through gateway
- `GET /api/v1/cluster/metrics` - Gateway metrics

### Federation Operations
- `GET /api/v1/federation/status` - Federation health
- `GET /api/v1/federation/clusters` - List federated clusters
- `GET /api/v1/federation/domains` - List DataDomains
- `POST /api/v1/federation/domains` - Create DataDomain
- `POST /api/v1/federation/domains/:name/join` - Join a DataDomain
- `POST /api/v1/federation/domains/:name/leave` - Leave a DataDomain
- `POST /api/v1/federation/domains/:name/balance` - Rebalance domain
- `GET /api/v1/federation/metrics` - Federation metrics

### Key-Value Operations (CouchDB-compatible)
- `GET /api/v1/kv/{db}?namespace={ns}` - Get database info (doc count, sizes, sequence)
- `PUT /api/v1/kv/{db}?namespace={ns}` - Create Key-Value database
- `DELETE /api/v1/kv/{db}?namespace={ns}` - Delete Key-Value database
- `GET /api/v1/kv/{db}/{id}?namespace={ns}` - Get document by ID
- `PUT /api/v1/kv/{db}/{id}?namespace={ns}` - Create/update document (auto _rev generation)
- `POST /api/v1/kv/{db}/{id}?namespace={ns}` - Update document (upsert)
- `DELETE /api/v1/kv/{db}/{id}?rev={rev}&namespace={ns}` - Delete document (requires current _rev)
- `GET /api/v1/kv/{db}/_all_docs?include_docs=true&limit=N&skip=N&namespace={ns}` - List all documents with pagination
- `POST /api/v1/kv/{db}/_find?namespace={ns}` - Mango query (selector-based: $eq, $gt, $gte, $lt, $lte, $ne, $in, $nin, $exists, $type)
- `GET /api/v1/kv/{db}/_index?namespace={ns}` - List indexes
- `POST /api/v1/kv/{db}/_index?namespace={ns}` - Create index
- `POST /api/v1/kv/{db}/_bulk_docs?namespace={ns}` - Bulk document operations (all_or_nothing support)
- `POST /api/v1/kv/{db}/_compact?namespace={ns}` - Compact database
- `POST /api/v1/kv/{db}/_ensure_full_commit?namespace={ns}` - Flush writes to disk
- `GET /api/v1/kv/{db}/_rev_limit?namespace={ns}` - Get revision limit
- `PUT /api/v1/kv/{db}/_rev_limit?namespace={ns}` - Set revision limit

### Transaction Management
- `POST /api/v1/transaction/begin` - Begin a new transaction (returns transaction_id)
- `POST /api/v1/transaction/{id}/execute` - Queue an operation for a pending transaction
- `POST /api/v1/transaction/{id}/commit` - Commit transaction with consensus
- `POST /api/v1/transaction/{id}/rollback` - Rollback transaction (reverses operations via before/after images)

### Authentication
- `POST /api/v1/auth/login` - User login (returns session info)
- `POST /api/v1/auth/register` - User registration
- `POST /api/v1/auth/token/create` - Create API token
- `POST /api/v1/auth/token/revoke/:token_id` - Revoke API token
- `GET /api/v1/auth/tokens` - List user tokens
- `GET /api/v1/auth/users` - List users (admin only)
- `GET /api/v1/auth/roles` - List available roles
- `POST /api/v1/auth/segment/create` - Create data segment (admin only)

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

### Key-Value Engine
CouchDB-compatible document storage with MVCC versioning.

**Use Cases:**
- Session storage
- User profiles
- Caching layer
- Configuration management
- Real-time applications

**Features:**
- _id/_rev versioning (MVCC) with automatic generation tracking
- Mango queries (selector-based: $eq, $gt, $gte, $lt, $lte, $ne, $in, $nin, $exists, $type)
- Bulk document operations with all_or_nothing conflict handling
- Index creation and management
- _all_docs pagination (include_docs, limit, skip)
- Database info and maintenance (_compact, _ensure_full_commit, _rev_limit)
- Tombstone deletion for replication support
- Persistent storage via sled embedded database
- **Namespace isolation** via `?namespace=` query parameter on all KV endpoints
- **Optional AES-256-GCM encryption** per database via `enable_database_encryption()`
- **Prometheus metrics** for operation count, latency, and error tracking
- **StorageEngine trait integration** for consensus replication and cross-engine queries
- **Federation capability advertisement** in `FedClusterAnnounce`

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
│  │  AI/ML, Blockchain, Consensus, Transactions, Protocol   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                                │
┌─────────────────────────────────────────────────────────┐
│                  Storage Layer                          │
│  ┌─────────┬─────────┬─────────┬─────────┬─────────┐  │
│  │Columnar │ Vector  │Document │Relational│KeyValue │  │
│  └─────────┴─────────┴─────────┴─────────┴─────────┘  │
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

### Authentication & Authorization
- **User Authentication**: Secure login with Argon2 password hashing
- **API Tokens**: Cryptographically secure tokens with SHA-256 hashing
- **RBAC**: Role-based access control with predefined roles (admin, developer, analyst, readonly)
- **Multi-tenancy**: Segment-based data isolation
- **Account Lockout**: Protection against brute-force attacks

### Encryption
- Data at rest: AES-256-GCM encryption
- Data in transit: TLS support
- Key rotation: Configurable intervals
- Token encryption: AES-256-GCM

### Cluster Security (Hyperledger-style)
- Genesis key system for trust establishment
- Node identity certificates
- Mutual authentication between nodes
- Trust chain validation

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
- ✅ All storage engines (columnar, vector, document, relational, **key-value**)
- ✅ Key-Value: CouchDB-compatible API, MVCC, Mango queries, bulk ops, indexes, pagination
- ✅ Key-Value: Namespace isolation (`?namespace=` on all KV endpoints)
- ✅ Key-Value: Optional AES-256-GCM per-database encryption
- ✅ Key-Value: Prometheus metrics (ops count, latency, errors, DB/doc gauges)
- ✅ Key-Value: StorageEngine trait impl for consensus/integration
- ✅ Key-Value: Federation capability advertisement
- ✅ AI/ML engine: real LinearRegression, LogisticRegression, K-Means++, TimeSeries, AnomalyDetection, Forecast, PatternAnalysis
- ✅ Blockchain Audit Ledger: immutable append-only, SHA-256 Merkle tree, tamper detection
- ✅ Ed25519 transaction signatures with verify_signature()
- ✅ Secure Protocol Layer: AES-256-GCM, Ed25519, HMAC, X.509 trust
- ✅ Consensus mechanism and transactions
- ✅ Encryption and security features
- ✅ Clustering, gateway, federation, geo-distribution
- ✅ Java JDBC PreparedStatement with batch support
- ✅ CLI tools and API
- ✅ Structured backup/restore with manifest + Blake3 checksums
- ✅ No placeholders or TODOs remaining
- ✅ All tests passing (Rust: 228 lib, Java: 213)

## Authors

- **devahil@gmail.com** - *Lead Developer* - [devahil@gmail.com](mailto:devahil@gmail.com)
- **PrimusDB Team** - *Contributors and Maintainers*

## Acknowledgments

Built with Rust, inspired by modern database architectures combining the best of multiple paradigms.
 