# PrimusDB Architecture Documentation

This document provides a comprehensive overview of PrimusDB's architecture, design decisions, and technical implementation details.

## System Overview

PrimusDB is a high-performance, hybrid database engine that combines multiple storage paradigms (columnar, vector, document, and relational) into a unified system optimized for modern analytical and AI workloads.

### Architecture Diagram (ASCII)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Client Layer                                  │
│  ┌─────────────────┬─────────────────┬─────────────────┐                │
│  │  CLI Interface │    HTTP API     │ Language SDKs   │                │
│  └─────────────────┴─────────────────┴─────────────────┘                │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Application Layer                               │
│  ┌─────────────────┬─────────────────┬─────────────────┬─────────────┐  │
│  │ Query Processor│Transaction Mgr │   AI/ML Engine  │Consensus Eng │  │
│  └─────────────────┴─────────────────┴─────────────────┴─────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          Storage Layer                                  │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┐              │
│  │Columnar Eng│ Vector Eng  │Document Eng │Relational En│              │
│  └─────────────┴─────────────┴─────────────┴─────────────┘              │
│  ┌─────────────────┬─────────────────┐                                  │
│  │   Cache Layer   │  Index Manager │                                  │
│  └─────────────────┴─────────────────┘                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        Persistence Layer                                │
│  ┌─────────────────┬─────────────────┬─────────────────┬─────────────┐  │
│  │ Sled Database  │ File System    │  Compression   │ Encryption   │  │
│  └─────────────────┴─────────────────┴─────────────────┴─────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

### Data Flow
1. **Client Layer** → **Application Layer** → **Storage Layer** → **Persistence Layer**
2. All operations flow through the Query Processor for optimization
3. Transaction Manager ensures ACID compliance
4. AI/ML Engine provides predictive analytics
5. Consensus Engine handles distributed operations

### Storage Engines Architecture

```
Storage Engines Interaction:
┌─────────────────────────────────────────────────────────────────────────┐
│                          Storage Layer                                  │
│                                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Columnar  │  │   Vector    │  │  Document  │  │ Relational  │     │
│  │   Engine    │  │   Engine    │  │   Engine    │  │   Engine    │     │
│  │             │  │             │  │             │  │             │     │
│  │ • LZ4 Comp  │  │ • HNSW Index│  │ • JSON Docs │  │ • ACID Txns │     │
│  │ • Bitmap Idx│  │ • Similarity│  │ • Dynamic   │  │ • Foreign   │     │
│  │ • SIMD Ops  │  │ • Distance  │  │ • Indexing  │  │ • Keys      │     │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘     │
│           │              │              │              │                │
└───────────┼──────────────┼──────────────┼──────────────┼────────────────┘
            │              │              │              │
            └──────────────┼──────────────┼──────────────┼─────────────────
                           │              │              │
                ┌──────────▼──────────────▼──────────────▼──────────┐
                │                Cache Layer                         │
                │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │
                │  │ LRU Cache  │  │ Compression │  │ Encryption  │ │
                │  └─────────────┘  └─────────────┘  └─────────────┘ │
                └─────────────────────────────────────────────────────┘
```

## Core Architecture Principles

### 1. Hybrid Storage Paradigm
- **Multiple Engines**: Specialized storage engines for different data patterns
- **Unified Interface**: Single API for all storage types
- **Automatic Optimization**: Query planner selects optimal engine per operation

### 2. Zero-Copy Architecture
- **Memory Efficiency**: Minimal data copying between layers
- **Streaming Processing**: Data processed in streams when possible
- **Lazy Evaluation**: Computations deferred until necessary

### 3. Event-Driven Concurrency
- **Async/Await**: Rust async runtime for scalable I/O
- **Actor Model**: Isolated components communicating via channels
- **Non-Blocking Operations**: All I/O operations are asynchronous

### 4. Plugin Architecture
- **Extensible Engines**: New storage engines can be added without core changes
- **Modular Components**: AI, consensus, and caching are pluggable
- **Driver System**: Language drivers follow consistent patterns

## Component Architecture

### Storage Engines

#### Columnar Engine
```rust
pub struct ColumnarEngine {
    config: PrimusDBConfig,
    db: sled::Db,
}

impl StorageEngine for ColumnarEngine {
    async fn insert(&self, table: &str, data: &Value, tx: &Transaction) -> Result<u64> {
        // Column-oriented storage with compression
        // Bitmap indexing for fast analytics
        // Vectorized operations for aggregations
    }

    async fn select(&self, table: &str, conditions: Option<&Value>,
                   limit: u64, offset: u64, tx: &Transaction) -> Result<Vec<Record>> {
        // Predicate pushdown to storage layer
        // Column pruning for efficiency
        // Parallel scan with SIMD
    }
}
```

**Key Features:**
- LZ4 compression with adaptive block sizes
- Bitmap indexes for fast filtering
- Vectorized aggregation operations
- Memory-mapped I/O for large datasets

#### Vector Engine
```rust
pub struct VectorEngine {
    config: PrimusDBConfig,
    db: sled::Db,
}

impl StorageEngine for VectorEngine {
    async fn insert(&self, table: &str, data: &Value, tx: &Transaction) -> Result<u64> {
        // HNSW indexing for approximate nearest neighbors
        // Distance metrics: Cosine, Euclidean, Dot Product
        // Quantization for memory efficiency
    }

    async fn select(&self, table: &str, conditions: Option<&Value>,
                   limit: u64, offset: u64, tx: &Transaction) -> Result<Vec<Record>> {
        // Similarity search with configurable precision
        // Multi-probe search for better recall
        // Distance computation optimization
    }
}
```

**Key Features:**
- HNSW (Hierarchical Navigable Small World) indexing
- Configurable distance metrics
- Memory-efficient quantization
- Batch processing capabilities

#### Document Engine
```rust
pub struct DocumentEngine {
    config: PrimusDBConfig,
    collections: HashMap<String, DocumentCollection>,
}

impl StorageEngine for DocumentEngine {
    async fn insert(&self, table: &str, data: &Value, tx: &Transaction) -> Result<u64> {
        // JSON document storage with schema flexibility
        // Dynamic indexing on accessed fields
        // Full-text search capabilities
    }

    async fn select(&self, table: &str, conditions: Option<&Value>,
                   limit: u64, offset: u64, tx: &Transaction) -> Result<Vec<Record>> {
        // Complex query evaluation with JSONPath
        // Index-aware query planning
        // Result pagination and sorting
    }
}
```

**Key Features:**
- Schema-less JSON storage
- Dynamic field indexing
- Complex query operators ($and, $or, $regex, etc.)
- Full-text search integration

#### Relational Engine
```rust
pub struct RelationalEngine {
    config: PrimusDBConfig,
    tables: HashMap<String, RelationalTable>,
    foreign_keys: HashMap<String, Vec<ForeignKey>>,
}

impl StorageEngine for RelationalEngine {
    async fn insert(&self, table: &str, data: &Value, tx: &Transaction) -> Result<u64> {
        // ACID-compliant insertions
        // Foreign key constraint validation
        // Trigger execution
    }

    async fn select(&self, table: &str, conditions: Option<&Value>,
                   limit: u64, offset: u64, tx: &Transaction) -> Result<Vec<Record>> {
        // SQL-like query processing
        // Join optimization and execution
        // Aggregate function computation
    }
}
```

**Key Features:**
- ACID transaction support
- Foreign key constraints
- Complex joins (inner, left, right, full outer)
- Schema management and validation

### Query Processing Pipeline

```
Query Processing Flow
═══════════════════════

1. Parse Query
   ↓
2. Query Analysis
   ↓
3. Engine Selection
   ↓
4. Query Optimization
   ↓
5. Plan Execution
   ↓
6. Result Formatting
   ↓
7. Response Delivery
```

#### Query Parser
```rust
pub struct QueryParser;

impl QueryParser {
    pub fn parse(input: &str) -> Result<Query> {
        // Tokenization and syntax analysis
        // AST construction
        // Semantic validation
    }
}
```

#### Query Optimizer
```rust
pub struct QueryOptimizer;

impl QueryOptimizer {
    pub fn optimize(query: Query, stats: &Statistics) -> Result<ExecutionPlan> {
        // Cost-based optimization
        // Index selection
        // Join reordering
        // Predicate pushdown
    }
}
```

#### Execution Engine
```rust
pub struct ExecutionEngine;

impl ExecutionEngine {
    pub async fn execute(plan: ExecutionPlan, context: &ExecutionContext) -> Result<QueryResult> {
        // Parallel execution coordination
        // Resource management
        // Error handling and recovery
        // Result aggregation
    }
}
```

### Transaction Management

#### Transaction Manager
```rust
pub struct TransactionManager {
    config: PrimusDBConfig,
    active_transactions: HashMap<String, Transaction>,
    transaction_log: Arc<FileTransactionLog>,
    journal: Arc<JournalManager>,
}

impl TransactionManager {
    pub async fn begin_transaction(&mut self, isolation_level: IsolationLevel) -> Result<String> {
        // Generate transaction ID
        // Create transaction context
        // Initialize isolation level
        // Log transaction start
    }

    pub async fn commit_transaction(&mut self, tx_id: &str) -> Result<()> {
        // Validate transaction state
        // Acquire commit locks
        // Write-ahead logging
        // Atomic commit protocol
        // Release locks
    }

    pub async fn rollback_transaction(&mut self, tx_id: &str) -> Result<()> {
        // Identify transaction operations
        // Reverse operations in order
        // Release locks
        // Clean up transaction state
    }
}
```

#### Isolation Levels
- **Read Uncommitted**: No isolation, dirty reads possible
- **Read Committed**: Prevents dirty reads
- **Repeatable Read**: Prevents non-repeatable reads
- **Serializable**: Full isolation, prevents phantom reads

#### Concurrency Control
```rust
pub enum LockMode {
    Shared,
    Exclusive,
    Update,
    Intent,
}

pub struct LockManager {
    locks: HashMap<String, Vec<LockRequest>>,
    wait_queue: Vec<LockRequest>,
}

impl LockManager {
    pub async fn acquire_lock(&mut self, resource: &str, mode: LockMode,
                             timeout: Duration) -> Result<Lock> {
        // Check for conflicts
        // Wait for conflicting locks to release
        // Grant lock or timeout
    }

    pub async fn release_lock(&mut self, lock: Lock) -> Result<()> {
        // Remove lock from resource
        // Wake waiting transactions
        // Update lock metadata
    }
}
```

### AI/ML Integration

#### AI Engine Architecture
```rust
pub struct AIEngine {
    config: PrimusDBConfig,
    models: HashMap<String, Model>,
    predictors: HashMap<String, Predictor>,
}

impl AIEngine {
    pub async fn train_model(&mut self, request: &TrainingRequest) -> Result<Model> {
        // Data preparation and validation
        // Model initialization
        // Training loop with optimization
        // Model validation and testing
        // Model persistence
    }

    pub async fn predict(&self, request: &PredictionRequest) -> Result<PredictionResult> {
        // Input validation and preprocessing
        // Model loading and inference
        // Confidence calculation
        // Result formatting
    }
}
```

#### Supported Algorithms
- **Linear Regression**: Continuous value prediction
- **Logistic Regression**: Binary classification
- **Time Series**: Temporal forecasting
- **Anomaly Detection**: Statistical outlier detection
- **Clustering**: Unsupervised grouping

### Consensus and Replication

#### Hyperledger-Style Consensus
```rust
pub struct HyperledgerConsensus {
    config: PrimusDBConfig,
    current_state: ChainState,
    validators: HashMap<String, Validator>,
    pending_transactions: Vec<Transaction>,
}

impl ConsensusEngine for HyperledgerConsensus {
    async fn propose_transaction(&mut self, tx: Transaction) -> Result<()> {
        // Transaction validation
        // Endorsement collection
        // Ordering service submission
        // Commit preparation
    }

    async fn commit_block(&mut self, block: Block) -> Result<()> {
        // Block validation
        // State updates
        // Ledger persistence
        // Notification broadcasting
    }
}
```

#### Replication Strategy
- **Synchronous Replication**: Strong consistency, higher latency
- **Asynchronous Replication**: Eventual consistency, better performance
- **Semi-Synchronous**: Balance between consistency and performance

### Clustering Architecture

#### Node Roles
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeRole {
    Coordinator,  // Cluster coordination and metadata
    Worker,       // Data processing and storage
    Gateway,      // Client connection handling
}
```

#### Cluster Communication
```rust
pub struct GossipProtocol {
    node_id: String,
    peers: HashMap<String, PeerInfo>,
    message_queue: VecDeque<GossipMessage>,
}

impl GossipProtocol {
    pub async fn broadcast(&mut self, message: GossipMessage) -> Result<()> {
        // Message serialization
        // Peer selection for dissemination
        // Infection-style propagation
        // Acknowledgment handling
    }

    pub async fn receive_message(&mut self, message: GossipMessage) -> Result<()> {
        // Message validation
        // Duplicate detection
        // State updates
        // Further propagation
    }
}
```

### Caching Layer

#### Multi-Level Caching
```rust
pub struct CacheManager {
    l1_cache: LruCache<String, Vec<u8>>,  // Hot data, in-memory
    l2_cache: sled::Tree,                 // Warm data, on-disk
    l3_cache: Option<RedisClient>,        // Cold data, distributed
}

impl CacheManager {
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // L1 cache lookup
        // L2 cache lookup on miss
        // L3 cache lookup on miss
        // Cache miss handling
    }

    pub async fn put(&mut self, key: String, value: Vec<u8>,
                     ttl: Option<Duration>) -> Result<()> {
        // Write-through to all levels
        // TTL management
        // Eviction policy application
    }
}
```

### Security Architecture

#### Encryption Layers
```rust
pub struct CryptoManager {
    config: SecurityConfig,
    master_key: Vec<u8>,
    key_rotation_schedule: KeyRotationSchedule,
}

impl CryptoManager {
    pub fn encrypt_data(&self, data: &[u8], context: &EncryptionContext) -> Result<Vec<u8>> {
        // Key derivation for context
        // AES-GCM encryption
        // Authentication tag generation
        // Metadata attachment
    }

    pub fn decrypt_data(&self, encrypted_data: &[u8],
                       context: &EncryptionContext) -> Result<Vec<u8>> {
        // Key derivation for context
        // AES-GCM decryption
        // Authentication verification
        // Data integrity validation
    }
}
```

#### Access Control
```rust
pub struct AccessController {
    users: HashMap<String, User>,
    roles: HashMap<String, Role>,
    permissions: HashMap<String, Vec<Permission>>,
}

impl AccessController {
    pub async fn check_permission(&self, user: &str, resource: &str,
                                 action: &str) -> Result<bool> {
        // User authentication verification
        // Role resolution
        // Permission evaluation
        // Policy decision
    }
}
```

## Performance Characteristics

### Scalability Metrics
- **Horizontal Scaling**: Linear performance increase with node addition
- **Data Volume**: Petabyte-scale storage capacity
- **Concurrent Connections**: 10,000+ simultaneous clients
- **Query Throughput**: 100,000+ queries per second in clustered setup

### Memory Architecture
```rust
// Memory layout optimization
#[repr(align(64))]  // Cache line alignment
pub struct Record {
    pub id: String,
    pub data: Value,
    pub metadata: HashMap<String, String>,
}

// Zero-copy data structures
pub struct ZeroCopyBuffer {
    data: *const u8,
    len: usize,
    _marker: PhantomData<&'static [u8]>,
}
```

### I/O Optimization
- **Direct I/O**: Bypass OS cache for large transfers
- **Vectored I/O**: Multiple buffer operations in single syscall
- **AIO (Asynchronous I/O)**: Non-blocking disk operations
- **Memory-Mapped Files**: Efficient large file access

## Deployment Architectures

### Single Node
```
┌─────────────────┐
│   PrimusDB      │
│  ┌────────────┐ │
│  │ All Engines │ │
│  └────────────┘ │
│  ┌────────────┐ │
│  │   Cache     │ │
│  └────────────┘ │
│  ┌────────────┐ │
│  │  Storage    │ │
│  └────────────┘ │
└─────────────────┘
```

### Multi-Node Cluster
```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ Coordinator │◄──►│   Worker    │◄──►│   Worker    │
│             │    │             │    │             │
│ ┌─────────┐ │    │ ┌─────────┐ │    │ ┌─────────┐ │
│ │  Meta   │ │    │ │ Storage │ │    │ │ Storage │ │
│ │  Data   │ │    │ │ Engine  │ │    │ │ Engine  │ │
│ └─────────┘ │    │ └─────────┘ │    │ └─────────┘ │
└─────────────┘    └─────────────┘    └─────────────┘
       ▲                   ▲                   ▲
       │                   │                   │
       └───────────────────┼───────────────────┘
                       │
                ┌─────────────┐
                │   Gateway   │
                │             │
                │ ┌─────────┐ │
                │ │ Clients  │ │
                │ │ Access  │ │
                │ └─────────┘ │
                └─────────────┘
```

### Edge Deployment
```
┌─────────────────┐    ┌─────────────────┐
│   Cloud DB      │    │   Edge Node     │
│  ┌────────────┐ │    │  ┌────────────┐ │
│  │ Full DB    │ │◄──►│  │ Sync DB     │ │
│  │ Features   │ │    │  │ Features    │ │
│  └────────────┘ │    │  └────────────┘ │
└─────────────────┘    └─────────────────┘
       ▲                        ▲
       │                        │
       └────────────────────────┘
            Low Latency
```

## Monitoring and Observability

### Metrics Collection
```rust
pub struct MetricsCollector {
    counters: HashMap<String, AtomicU64>,
    histograms: HashMap<String, Histogram>,
    gauges: HashMap<String, AtomicU64>,
}

impl MetricsCollector {
    pub fn record_query_duration(&mut self, engine: &str, duration: Duration) {
        let key = format!("query_duration_{}", engine);
        self.histograms.get_mut(&key).unwrap().record(duration.as_millis() as u64);
    }

    pub fn increment_operation_count(&mut self, operation: &str) {
        let key = format!("operation_count_{}", operation);
        self.counters.get_mut(&key).unwrap().fetch_add(1, Ordering::Relaxed);
    }
}
```

### Logging Architecture
```rust
pub struct Logger {
    level: LogLevel,
    outputs: Vec<Box<dyn LogOutput>>,
    format: LogFormat,
}

impl Logger {
    pub fn log(&self, level: LogLevel, message: &str, context: &LogContext) {
        if level >= self.level {
            let formatted = self.format.format(message, context);
            for output in &self.outputs {
                output.write(&formatted);
            }
        }
    }
}
```

## Future Architecture Extensions

### Quantum-Resistant Cryptography
- Post-quantum key exchange algorithms
- Lattice-based encryption schemes
- Quantum-safe signature algorithms

### AI-Native Database
- Automatic query optimization using ML
- Predictive caching and prefetching
- Intelligent data placement
- Self-tuning performance parameters

### Multi-Model Extensions
- Graph database integration
- Time series optimizations
- Geospatial indexing
- Full-text search enhancements

This architecture documentation provides the foundation for understanding PrimusDB's design and implementation decisions.