# PrimusDB System Architecture

PrimusDB is a hybrid database engine written in Rust that combines columnar, vector,
document, relational, and key-value storage into a unified system with AI/ML
capabilities, distributed consensus, and enterprise-grade security.

## Architecture Overview

```
                            ┌─────────────────────────────────────────────────────┐
                            │                    API LAYER                         │
                            │  ┌──────────┐  ┌──────────┐  ┌───────────────────┐  │
                            │  │ REST API │  │   CLI    │  │ Language Drivers   │  │
                            │  │  (Axum)  │  │  (Clap)  │  │ Py/Ruby/Java/Node │  │
                            │  └────┬─────┘  └────┬─────┘  └────────┬──────────┘  │
                            │       │              │                  │            │
                            │       └──────────────┴──────────────────┘            │
                            └──────────────────────┬──────────────────────────────┘
                                                     │
                            ┌────────────────────────┴──────────────────────────────┐
                            │                 PROCESSING LAYER                       │
                            │  ┌──────────────────────┐  ┌──────────────────────┐   │
                            │  │   Query Processor     │  │    AI/ML Engine      │   │
                            │  │  ┌─────────────────┐  │  │  ┌────────────────┐  │   │
                            │  │  │ Parser (SQL/UQL)│  │  │  │  Predictions    │  │   │
                            │  │  │ Planner (Opt.)  │  │  │  │  Clustering     │  │   │
                            │  │  │ Executor (Async)│  │  │  │  Anomaly Detect │  │   │
                            │  │  │ Aggregator      │  │  │  │  Pattern Anal.  │  │   │
                            │  │  └─────────────────┘  │  │  └────────────────┘  │   │
                            │  └──────────────────────┘  └──────────────────────┘   │
                            │  ┌──────────────────────────────────────────────┐     │
                            │  │         Transaction Manager (ACID)           │     │
                            │  │    Begin ──> Execute ──> Commit/Rollback     │     │
                            │  └──────────────────────────────────────────────┘     │
                            └────────────────────────┬──────────────────────────────┘
                                                     │
                            ┌────────────────────────┴──────────────────────────────┐
                            │                   STORAGE LAYER                        │
                            │                                                       │
                            │  ┌────────────┐  ┌────────────┐  ┌──────────────────┐ │
                            │  │  Columnar  │  │   Vector   │  │     Document     │ │
                            │  │  Engine    │  │   Engine   │  │     Engine       │ │
                            │  │ ┌────────┐ │  │ ┌────────┐ │  │ ┌──────────────┐ │ │
                            │  │ │LZ4 Comp│ │  │ │SIMD    │ │  │ │JSON Storage  │ │ │
                            │  │ │Bitmap  │ │  │ │FAISS   │ │  │ │B-Tree Index  │ │ │
                            │  │ │Indexes │ │  │ │Search  │ │  │ │Schema Val.   │ │ │
                            │  │ └────────┘ │  │ └────────┘ │  │ └──────────────┘ │ │
                            │  └────────────┘  └────────────┘  └──────────────────┘ │
                            │                                                       │
                            │  ┌────────────┐  ┌────────────┐                       │
                            │  │ Relational │  │  Key-Value │                       │
                            │  │  Engine    │  │   Engine   │                       │
                            │  │ ┌────────┐ │  │ ┌────────┐ │                       │
                            │  │ │SQL     │ │  │ │CouchDB │ │                       │
                            │  │ │ACID    │ │  │ │Compat  │ │                       │
                            │  │ │FK/Trig │ │  │ │_rev    │ │                       │
                            │  │ └────────┘ │  │ └────────┘ │                       │
                            │  └────────────┘  └────────────┘                       │
                            │                                                       │
                            │              ┌──────────────────┐                     │
                            │              │  sled (Persistence│                     │
                            │              │  Layer)           │                     │
                            │              └──────────────────┘                     │
                            └────────────────────────┬──────────────────────────────┘
                                                     │
                            ┌────────────────────────┴──────────────────────────────┐
                            │               INFRASTRUCTURE LAYER                     │
                            │                                                       │
                            │  ┌──────────────────┐  ┌──────────────────────────┐  │
                            │  │  Consensus       │  │     Cluster Manager     │  │
                            │  │  ┌──────────────┐│  │  ┌─────────────────────┐│  │
                            │  │  │Raft (leader  ││  │  │ SWIM Gossip         ││  │
                            │  │  │election, log ││  │  │ Consistent Hashing  ││  │
                            │  │  │replication)  ││  │  │ Shard Management    ││  │
                            │  │  └──────────────┘│  │  │ Replication Engine  ││  │
                            │  │                  │  │  │ Sync Coordinator    ││  │
                            │  │  ┌──────────────┐│  │  │ Gateway (LB)       ││  │
                            │  │  │Federated Raft││  │  └─────────────────────┘│  │
                            │  │  │(cross-       ││  └──────────────────────────┘  │
                            │  │  │ cluster)     ││                                 │
                            │  │  └──────────────┘│  ┌──────────────────────────┐  │
                            │  └──────────────────┘  │     Federation Layer    │  │
                            │                        │  ┌─────────────────────┐│  │
                            │  ┌──────────────────┐  │  │Multi-cluster       ││  │
                            │  │  Security Manager│  │  │DataDomains         ││  │
                            │  │  ┌──────────────┐│  │  │Cross-cluster Repl. ││  │
                            │  │  │AES-256-GCM   ││  │  │Namespace Resolution││  │
                            │  │  │Argon2 Key Der.││  │  └─────────────────────┘│  │
                            │  │  │RBAC          ││  └──────────────────────────┘  │
                            │  │  │Key Rotation  ││                                 │
                            │  │  │Audit Logging ││  ┌──────────────────────────┐  │
                            │  │  └──────────────┘│  │     Observability       │  │
                            │  └──────────────────┘  │  ┌─────────────────────┐│  │
                            │                        │  │ Prometheus Metrics  ││  │
                            │  ┌──────────────────┐  │  │ Tracing (tokio-    ││  │
                            │  │  CDC Engine      │  │  │ console/opentele-  ││  │
                            │  │  ┌──────────────┐│  │  │ metry)             ││  │
                            │  │  │Change Data    ││  │  └─────────────────────┘│  │
                            │  │  │Capture Streams││  └──────────────────────────┘  │
                            │  │  └──────────────┘│                                 │
                            │  └──────────────────┘                                 │
                            └───────────────────────────────────────────────────────┘
```

## Layer Descriptions

### 1. API Layer

The API layer provides multiple interfaces for interacting with PrimusDB:

- **REST API** (`src/api/`): Built on Axum (0.7), exposes 50+ endpoints under `/api/v1/`
  for CRUD operations, AI/ML, cluster management, authentication, CDC, and federation.
  Middleware handles auth (Bearer tokens), CORS, compression, and request logging.

- **CLI** (`src/cli/`): Built on Clap (4.4), provides a unified command-line interface
  with subcommands for server management, querying, database operations, cluster
  management, AI/ML operations, and administration. A single binary exists:
  `primusdb` (the legacy `primusdb-server` and `primusdb-cli` binaries were removed).

- **Language Drivers** (`drivers/`): Native client libraries for:
  - **Rust** (`drivers/rust/`): Native crate with builder pattern
  - **Python** (`drivers/python/`): PyO3 native extension + pure Python client
  - **Node.js** (`drivers/node/`): TypeScript with async/await
  - **Java** (`drivers/java/`): JDBC driver with connection pooling
  - **Ruby** (`drivers/ruby/`): Faraday-based client

### 2. Processing Layer

- **Query Processor** (`src/query/`): A unified query engine (UQL) that parses SQL,
  MongoDB-style queries, Mango queries, and native UQL format. The planner creates
  optimal execution plans that route sub-operations to the appropriate storage engines.
  An executor runs these plans asynchronously, aggregating results across engines.

- **AI/ML Engine** (`src/ai/`): Built-in machine learning capabilities:
  - Linear regression and time series forecasting
  - K-means and hierarchical clustering
  - Statistical anomaly detection (Z-score, IQR)
  - Pattern recognition and trend analysis

- **Transaction Manager** (`src/transaction/`): Provides ACID transactions with:
  - Begin/commit/rollback workflow
  - Journal-based durability (sled-backed)
  - Optimistic concurrency control
  - Before/after image capture for rollback

### 3. Storage Layer

Five storage engines, each optimized for different workloads:

| Engine | File | Optimized For | Persistence | Key Features |
|--------|------|---------------|-------------|--------------|
| Columnar | `src/storage/columnar.rs` | OLAP / Analytics | sled | LZ4 compression, bitmap indexes |
| Vector | `src/storage/vector.rs` | Similarity Search | sled | SIMD, FAISS-style, Cosine/Euclidean/Dot |
| Document | `src/storage/document.rs` | JSON Documents | sled | Dynamic schema, B-Tree indexing |
| Relational | `src/storage/relational.rs` | OLTP / SQL | sled | FK constraints, triggers, views, sequences |
| Key-Value | `src/storage/keyvalue.rs` | CouchDB-compat | sled | `_id`/`_rev` MVCC, Mango queries |

All engines implement the `StorageEngine` trait (`src/storage/mod.rs`) with async
CRUD operations, schema management, and analytical capabilities. The sled library
provides the embedded database persistence layer.

### 4. Infrastructure Layer

- **Consensus** (`src/consensus/`): Raft-style consensus protocol with leader
  election, log replication, and term-based epoch validation. Includes a
  Hyperledger-style block validation system.

- **Cluster Manager** (`src/cluster/`): Full distributed cluster infrastructure:
  - **RPC Layer** (`rpc.rs`): TCP/bincode inter-node messaging with 25+ message types
  - **Raft** (`raft.rs`): Leader election, safety properties, snapshot installation
  - **SWIM Gossip** (`membership.rs`): Infection-style membership protocol
  - **Consistent Hashing** (`shard.rs`): Virtual-node hash ring for data distribution
  - **Replication** (`replication.rs`): Sync/Async/Quorum replication modes
  - **Gateway** (`gateway.rs`): Smart load balancer with circuit breaker, EWMA latency
  - **Federation** (`federation.rs`): Multi-cluster SuperScalar federation
  - **Federated Raft** (`federated_raft.rs`): Cross-cluster metadata consensus
  - **DataDomains** (`domain.rs`): Selective cross-cluster replication domains

- **Security Manager** (`src/crypto/`, `src/auth/`):
  - **Encryption**: AES-256-GCM authenticated encryption for data at rest
  - **Key Derivation**: Argon2id for per-file key generation
  - **RBAC**: Role-based access control with predefined roles (admin, developer,
    analyst, readonly, cluster_node)
  - **Key Rotation**: Automatic key rotation with configurable intervals
  - **Cluster Auth**: Hyperledger-style genesis key system with secp256k1 signatures

- **CDC Engine** (`src/cdc.rs`): Change Data Capture for streaming changes to
  external systems with offset tracking and multiple format support.

- **Observability** (`src/api/mod.rs`, `src/protocol/messaging.rs`): Prometheus
  metrics exposed via the `/metrics` endpoint, structured tracing via the
  `tracing` crate, and health check endpoints (`/health`, `/status`).

## Key Design Decisions

### 1. Hybrid Storage Architecture
PrimusDB does not force a single storage paradigm. Each engine implements the
common `StorageEngine` trait but internally optimizes for its specific workload.
The UQL engine can route sub-queries across engines, enabling cross-engine joins
and unified querying.

### 2. Embedded Database Core (sled)
All storage engines use [sled](https://github.com/spacejam/sled) as the
persistence layer rather than wrapping external databases. This eliminates
network round-trips for local operations, simplifies deployment (no separate
DB process), and enables tight integration with the transaction manager.

### 3. Async-First Design
Built on `tokio` with async traits throughout. The REST API (Axum), RPC layer,
cluster communication, and storage engine operations are all async, enabling
high concurrency with efficient resource usage.

### 4. Defense in Depth Security
Data is encrypted at rest using AES-256-GCM with per-file Argon2-derived keys.
All API endpoints require Bearer token authentication by default. Cluster
communication uses authenticated message exchange with secp256k1 signatures.
Encrypted files include integrity verification to detect tampering.

### 5. Raft + SWIM for Distributed Coordination
The cluster layer combines Raft (for strong consistency in leader election
and log replication) with SWIM gossip (for scalable membership management).
This hybrid approach provides both the safety guarantees needed for metadata
consensus and the scalability needed for large clusters.

### 6. Federated Multi-Cluster (SuperScalar)
The federation layer enables cluster-of-clusters topologies with cross-cluster
data replication domains, federated namespace resolution, and geo-distributed
sharding. This allows PrimusDB to scale horizontally across data centers and
regions.

### 7. Unified CLI Architecture
The CLI uses clap derive macros for type-safe argument parsing. Commands follow
a consistent pattern: types in `command.rs`, dispatch in `mod.rs`, handlers in
`cmd/*.rs`, and output formatting in `output.rs`. This makes adding new commands
straightforward.

### 8. Built-in AI/ML
Rather than requiring separate ML infrastructure, PrimusDB embeds lightweight
ML capabilities directly in the engine. This enables in-database predictions,
clustering, and anomaly detection without data movement.

### 9. Crate Organization
The workspace is split into focused crates (`primusdb-core`, `primusdb-storage`,
`primusdb-crypto`, `primusdb-consensus`, etc.) for clean separation of concerns
and incremental compilation. The main `primusdb` crate re-exports and integrates
all functionality.

### 10. Comprehensive Testing Strategy
Tests span multiple levels: in-file unit tests (`#[cfg(test)]`), integration tests
in `tests/`, doc tests in rustdoc, and benchmarks in `benches/`. CLI commands
are designed to be testable through programmatic API access.
