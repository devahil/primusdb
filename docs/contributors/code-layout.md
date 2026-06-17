# Code Layout

This document describes the repository structure and key files in the PrimusDB
project.

## Repository Root

```
primusdb/
├── Cargo.toml              # Workspace manifest, dependencies, binary definitions
├── Cargo.lock              # Dependency lock file
├── CHANGELOG.md            # Full release history (v0.1.0 → v1.3.1-alpha)
├── LICENSE                 # GPL-3.0 license
├── README.md               # Project overview and quick start
├── Dockerfile              # Multi-stage Docker build
│
├── src/                    # Main library and binary sources
├── crates/                 # Workspace member crates
├── drivers/                # Multi-language client drivers
├── tests/                  # Integration tests
├── benches/                # Criterion benchmarks
├── scripts/                # Build, package, dev scripts
├── docs/                   # User and contributor documentation
├── config/                 # Configuration examples and templates
├── data/                   # Default data directory (gitignored)
├── dist/                   # Distribution packages (gitignored)
├── target/                 # Build artifacts (gitignored)
├── examples/               # Usage examples
└── img/                    # Images (architecture diagrams, etc.)
```

## `src/` — Main Source Tree

```
src/
├── lib.rs                  # Crate root: PrimusDB struct, PrimusDBConfig,
│                           #   Query/QueryResult/Record types, module declarations
│
├── main.rs                 # Unified CLI entry point (primary binary)
│                           #   Initializes tracing, calls cli::run()
│
├── bin/
│   ├── server.rs           # Legacy server binary (primusdb-server)
│   │                      #   Axum HTTP server with REST API
│   ├── server.rs.bak       # Backup of previous server.rs
│   ├── cli.rs              # Legacy CLI binary (primusdb-cli)
│   │                      #   Thin wrapper around cli::run()
│   └── cli.rs.bak          # Backup of previous cli.rs
│
├── cli/                    # Unified CLI subsystem
│   ├── mod.rs              # Dispatch: run() parses args, matches commands
│   ├── command.rs          # Clap derive types: Cli, Commands, all subcommands
│   ├── output.rs           # Output formatting: OutputFormat, OutputData, format_output()
│   ├── tui.rs              # Terminal UI (Rich-style interactive interface)
│   ├── discovery.rs        # Network node discovery via UDP broadcast
│   ├── legacy.rs           # Legacy CLI compatibility shim
│   └── cmd/                # Command handler implementations
│       ├── mod.rs          # Module declarations for all command handlers
│       ├── server.rs       #   Server lifecycle (start/stop/restart/status/health)
│       ├── query.rs        #   Query/SQL execution
│       ├── db.rs           #   Database management (list/create/drop/describe/use)
│       ├── engine.rs       #   Storage engine inspection
│       ├── namespace.rs    #   Namespace management
│       ├── config.rs       #   Configuration (init/validate/show)
│       ├── cluster.rs      #   Cluster operations
│       ├── protocol.rs     #   Protocol layer management
│       ├── backup.rs       #   Backup/restore
│       ├── auth.rs         #   Authentication, user, role management
│       ├── ai.rs           #   AI/ML model operations
│       ├── vector.rs       #   Vector search/index management
│       ├── graph.rs        #   Graph traversal
│       ├── cdc.rs          #   Change Data Capture
│       ├── doctor.rs       #   Diagnostic checks
│       └── discover.rs     #   Node discovery
│
├── api/                    # REST API server
│   └── mod.rs              # APIServer: Axum-based HTTP server with all routes
│                           #   50+ endpoints under /api/v1/
│
├── query/                  # Unified Query Language (UQL) engine
│   ├── mod.rs              # UqlEngine, UqlQuery, UqlResult types
│   ├── parser.rs           # Multi-dialect parser (SQL, MongoDB, Mango, UQL)
│   ├── planner.rs          # Query plan optimization and engine routing
│   └── executor.rs         # Async query execution across engines
│
├── storage/                # Storage engine implementations
│   ├── mod.rs              # StorageEngine trait, Schema, Field, FieldType,
│   │                      #   Index, Constraint, TableInfo, and sub-module decls
│   ├── columnar.rs         # Columnar engine: LZ4 compression, bitmap indexes
│   ├── vector.rs           # Vector engine: SIMD similarity search
│   ├── document.rs         # Document engine: JSON storage with dynamic indexing
│   ├── relational.rs       # Relational engine: SQL, FK, triggers, views, sequences
│   └── keyvalue.rs         # Key-value engine: CouchDB-compatible (_id/_rev/Mango)
│
├── crypto/                 # Cryptographic operations
│   ├── mod.rs              # CryptoManager: key management, encrypt/decrypt
│   └── file_encryption.rs  # File-level AES-256-GCM with Argon2 key derivation
│
├── consensus/              # Consensus protocol
│   ├── mod.rs              # ConsensusEngine trait, HyperledgerStyleConsensus
│   ├── blockchain.rs       # Block validation, chain management
│   └── state_machine.rs    # State machine for consensus operations
│
├── transaction/            # Transaction management
│   └── mod.rs              # TransactionManager, Transaction, journal, rollback
│
├── cluster/                # Distributed cluster subsystem
│   ├── mod.rs              # ClusterManager, ClusterConfig, FederationConfig
│   ├── rpc.rs              # TCP/bincode RPC layer (25+ message types)
│   ├── raft.rs             # Raft consensus (leader election, log replication)
│   ├── membership.rs       # SWIM gossip membership protocol
│   ├── shard.rs            # Consistent hashing shard manager
│   ├── replication.rs      # Data replication engine
│   ├── sync.rs             # SyncCoordinator, vector clocks, reconciliation
│   ├── sync/               # Sync sub-modules
│   ├── gateway.rs          # Smart load balancer with circuit breaker
│   ├── federation.rs       # Multi-cluster federation (SuperScalar)
│   ├── federated_raft.rs   # Cross-cluster Raft for federation metadata
│   └── domain.rs           # DataDomain Manager for cross-cluster replication
│
├── ai/                     # AI/ML engine
│   ├── mod.rs              # AIEngine: training, prediction, analysis
│   └── predictive.rs       # ML models: regression, clustering, anomaly detection
│
├── auth/                   # Authentication and authorization
│   └── mod.rs              # AuthService, RBAC, token management, ClusterAuth
│
├── namespace/              # Namespace isolation for multi-tenant/ multi-model
│   └── mod.rs              # NamespaceController, NamespaceConfig
│
├── drivers/                # Server-side driver protocol handling
│   └── mod.rs              # Protocol handling for external driver connections
│
├── protocol/               # Wire protocol implementation
│   ├── mod.rs              # Protocol types and traits
│   ├── api.rs              # Protocol API definitions
│   ├── handlers.rs         # Message handlers
│   ├── messaging.rs        # Message serialization
│   ├── journaling.rs       # Protocol journaling
│   ├── recovery.rs         # Recovery mechanisms
│   └── trust.rs            # Trust validation
│
├── cache/                  # In-memory caching layer
│   └── mod.rs              # LRU cache with bloom filters
│
├── metrics.rs              # Prometheus metrics collection
├── cdc.rs                  # Change Data Capture engine
├── error.rs                # Error types (Error enum with all variants)
├── fulltext.rs             # Full-text search engine
├── graph.rs                # Graph traversal engine
├── parser.rs               # Legacy SQL/query parser (deprecated, use query/)
└── main.legacy.rs.bak      # Backup of previous main.rs
```

## `crates/` — Workspace Crates

Each crate provides focused functionality with minimal dependencies:

```
crates/
├── primusdb-core/          # Core types, traits, error definitions
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Re-exports core primitives
│       └── error.rs        # Shared error types
│
├── primusdb-storage/       # Pure storage engine abstractions
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-crypto/        # Cryptographic primitives (standalone)
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-consensus/     # Consensus protocol (extracted)
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-transaction/   # Transaction management (extracted)
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-ai/            # AI/ML engine (extracted)
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-cluster/       # Cluster management (extracted)
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-drivers/       # Driver protocol definitions
│   ├── Cargo.toml
│   └── src/
│
├── primusdb-api/           # API types (extracted)
│   ├── Cargo.toml
│   └── src/
│
└── primusdb-error/         # Error type definitions (extracted)
    ├── Cargo.toml
    └── src/
```

The workspace crates are designed for eventual extraction and separate publishing.
Currently, the main `primusdb` crate in `src/` contains the bulk of the logic,
with the workspace crates providing clean dependency boundaries.

## `drivers/` — Multi-Language Client Drivers

```
drivers/
├── rust/                   # Rust native driver
│   ├── Cargo.toml          #   Library crate (primusdb-driver)
│   ├── README.md
│   └── src/
│       └── lib.rs          #   Builder-pattern client with async API
│
├── python/                 # Python drivers
│   ├── setup.py            #   Package configuration
│   ├── Cargo.toml          #   PyO3 native extension
│   ├── primusdb/           #   Pure Python client library
│   │   └── __init__.py
│   ├── src/                #   PyO3 native Rust extension
│   │   └── lib.rs
│   └── tests/              #   Python test suite
│
├── node/                   # Node.js/TypeScript driver
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/                #   TypeScript source
│   │   └── index.ts
│   ├── dist/               #   Compiled JavaScript
│   ├── test/               #   Test suite
│   └── node_modules/
│
├── java/                   # Java JDBC driver
│   ├── pom.xml (or build.gradle)
│   └── src/
│       └── main/
│           └── java/
│               └── primusdb/
│
└── ruby/                   # Ruby gem
    ├── primusdb.gemspec
    └── lib/
        └── primusdb.rb
```

## `tests/` — Integration Tests

```
tests/
├── integration_tests.rs    # Core integration tests: CRUD across all 5 engines,
│                           #   namespace isolation, sequence operations, etc.
├── e2e_rest_api.rs         # End-to-end REST API tests
├── e2e_server.rs           # Server lifecycle and health checks
└── e2e_backup_restore.rs   # Backup/restore round-trip tests
```

## `benches/` — Criterion Benchmarks

```
benches/
├── storage_read.rs         # Storage engine read performance benchmarks
├── vector_search.rs        # Vector similarity search benchmarks
└── ai_ml.rs                # AI/ML model performance benchmarks
```

## `scripts/` — Developer and CI Scripts

```
scripts/
├── check-all.sh            # Run all quality checks (fmt, clippy, build, test)
├── check-docs.sh           # Documentation validation
├── build-release.sh        # Build release artifacts
├── package-linux.sh        # Create Linux distribution tarball
├── build-drivers.sh        # Build all language drivers
├── generate-completions.sh # Generate shell completion scripts
├── install.sh              # System installation helper
├── dev-start.sh            # Start development environment
├── dev-stop.sh             # Stop development environment
└── dev-reset.sh            # Reset development state
```

## Key Files and Their Purpose

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace definition with 11 members, dependencies, 3 binary targets, 3 benchmark targets |
| `src/lib.rs` | Library root — `PrimusDB` engine struct, `PrimusDBConfig`, 5 `StorageType` variants, `Query`/`QueryResult`/`Record`, all module declarations |
| `src/main.rs` | Unified binary entry — 14 lines, initializes tracing, calls `cli::run()` |
| `src/cli/command.rs` | All clap derive types — 910 lines, defines `Cli` struct and all subcommand enums |
| `src/cli/mod.rs` | CLI dispatch — parses args, matches 23 command variants to handler functions |
| `src/cli/output.rs` | Output formatting — `OutputFormat` (Table/Json/Csv/Yaml/Plain), `OutputData`, `format_output()` |
| `src/api/mod.rs` | REST API — 3200+ lines, Axum server with all route handlers grouped by category |
| `src/storage/mod.rs` | Storage trait + types — `StorageEngine` trait, `Schema`, `Field`, `FieldType`, `Index`, `Constraint` |
| `src/error.rs` | Error enum — all error variants organized by subsystem |
| `src/query/mod.rs` | UQL engine — multi-dialect parser, planner, executor for cross-engine queries |
| `src/cluster/mod.rs` | Cluster manager — coordinates RPC, Raft, SWIM, replication, sharding, federation |

## Version Compatibility

All crates in the workspace should be bumped together during releases. The current
version is `1.3.1-alpha`. The `primusdb` root crate and all sub-crates share the
same version number for simplicity, though individual crates may diverge when
published separately.
