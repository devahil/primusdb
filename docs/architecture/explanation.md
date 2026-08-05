# PrimusDB Feature Explanations

This document provides detailed explanations of PrimusDB features.

## Hybrid Storage Architecture

### Why Multiple Engines

Traditional databases force you to choose a single storage paradigm. PrimusDB implements four specialized engines that share common infrastructure.

### Choosing an Engine

| Your Need | Recommended Engine |
|-----------|-------------------|
| Analytics, aggregations | Columnar |
| Similarity search, recommendations | Vector |
| Flexible schemas, JSON data | Document |
| Transactions, joins, constraints | Relational |

## Columnar Storage

### How It Works

Columnar storage stores data column by column instead of row by row. This drastically reduces I/O for analytical queries.

### LZ4 Compression

PrimusDB uses LZ4 compression for its excellent balance between compression ratio and speed.

### Bitmap Indexing

For columns with low cardinality, bitmap indexes are extremely efficient.

## Vector Search

### HNSW Indexing

Hierarchical Navigable Small World (HNSW) is a state-of-the-art algorithm for approximate nearest neighbor search.

### Distance Metrics

- **Cosine similarity**: Best for text embeddings
- **Euclidean distance**: Intuitive distance between points
- **Dot product**: Computationally efficient

## Document Storage

### JSON Storage Model

The document engine stores JSON documents as binary blobs.

### Query Operators

```json
{"$and": [{"age": {"$gte": 18}}, {"status": "active"}]}
{"tags": {"$in": ["premium", "featured"]}}
```

## Relational Storage

### ACID Transactions

- **Atomicity**: All-or-nothing transactions
- **Consistency**: Constraints enforced
- **Isolation**: Concurrent transactions
- **Durability**: Changes survive failures

### Join Algorithms

- **Nested loop**: Efficient for small tables
- **Hash join**: Efficient for large equi-joins

## AI/ML Engine

### Predictive Analytics

- **Linear regression**: Continuous value prediction
- **Time series**: Seasonality-aware forecasting
- **Anomaly detection**: Outlier identification

### Clustering

The K-means algorithm groups similar points without requiring labeled examples.

## Security

### Encryption

PrimusDB implements AES-256-GCM encryption for data at rest.

### Access Control

Role-based access control (RBAC) restricts operations.

## Clustering and High Availability

### Cluster Architecture

Coordinator nodes manage metadata. Worker nodes store data and process queries. Gateway nodes handle client connections and route requests to appropriate workers.

### Consensus

The Hyperledger-style consensus mechanism ensures all nodes agree on state.

## Language Drivers

**IMPORTANT:** No driver is published. All must be compiled locally from source code in `drivers/`.

### Python

```bash
cd drivers/python
pip install setuptools-rust aiohttp pydantic typing-extensions
python setup.py build_ext --inplace
```

### Node.js

```bash
cd drivers/node
npm install
npm run build
```

### Java

```bash
cd drivers/java
mvn clean compile
```

### Ruby

```bash
cd drivers/ruby
gem build primusdb.gemspec
gem install ./primusdb-0.1.0.gem
```

## Performance

### Queries

Performance depends on engine, query patterns, and system configuration.

### Concurrency

The async architecture handles thousands of concurrent connections.

### Scalability

Horizontal sharding allows scaling to arbitrarily large datasets.

## Configuration

```toml
[storage]
data_dir = "/var/lib/primusdb"
cache_size = 536870912

[network]
port = 8080
max_connections = 1000

[cluster]
enabled = false
```

## References

- **[README.md](../README.md)** - General information
- **[USER.md](../user-guide/operations.md)** - User guide
- **[API_REFERENCE.md](../reference/api.md)** - API
- **[drivers/](drivers/)** - Driver source code
