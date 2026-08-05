# Storage Engines

PrimusDB provides six storage engines, each optimised for a distinct data model
and workload.  All engines share a common `StorageEngine` trait (CRUD +
analytics) but differ in internal layout, index structures, compression, and
transaction semantics.

Engine maturity varies — several are still alpha-quality.  See the status table
at the end of this document.

---

## 1. Columnar Engine

**Purpose:** Analytical / OLAP workloads.  
**Data layout:** Column-major; each column is stored as a contiguous segment.

### Features

- **LZ4 compression** per column chunk — typical compression ratios of 3–8× on
  numerical data.
- **Bitmap indexing** for low-cardinality columns; enables fast set operations
  (AND, OR, NOT) during filtering.
- **Vectorised execution** via SIMD-friendly column iteration.
- Column pruning — only the columns referenced by a query are loaded.
- Snapshot isolation for read consistency.

### Ideal use-cases

- Data warehousing and business intelligence.
- Time-series aggregation (SUM, AVG, COUNT over large ranges).
- Reporting queries that touch many rows but few columns.

### Limitations

- Single-row writes are expensive; bulk-insert (batched) is strongly
  recommended.
- No full-text or vector indexes.
- Transaction model is snapshot-level only (no serialisable ACID).

---

## 2. Vector Engine

**Purpose:** Similarity search and embedding storage.  
**Data layout:** Dense float vectors with optional metadata.

### Features

- **Distance metrics:** Cosine similarity, Euclidean (L2), Dot Product,
  Manhattan (L1).
- **HNSW indexing** (Hierarchical Navigable Small World) — graph-based ANN
  search with configurable `ef_construction` and `M` parameters.
- **IVF** (inverted-file) indexing for larger corpora.
- Flat (brute-force) mode for exact k-NN on small collections.
- SIMD-accelerated distance computations via `ndarray`.

### Ideal use-cases

- Semantic / vector search on embeddings (text, image, audio).
- Recommendation systems (user/item vectors).
- Nearest-neighbour lookup for ML pipelines.

### Limitations

- Alpha maturity — index parameters not yet auto-tuned.
- No ACID transactions; writes are atomic per-vector only.
- Metadata filtering is basic (pre-filter, no post-filter optimisation).

---

## 3. Document Engine

**Purpose:** Flexible JSON document storage.  
**Data layout:** BSON-like serialised documents with an in-memory B-tree index.

### Features

- Schema-less: documents can have arbitrary fields.
- Dynamic B-tree indexes on any field or nested path.
- Filter queries with operators: `$eq`, `$gt`, `$lt`, `$in`, `$regex`,
  `$exists`, etc.
- Optional schema validation (`Schema` struct with field types and constraints).
- Collection-level metadata (row count, size, timestamps).

### Ideal use-cases

- Content management systems.
- Application data with evolving schemas.
- Prototyping and rapid iteration.

### Limitations

- No joins or cross-document references (use the relational engine for that).
- Indexes are rebuilt on server restart (not persisted to disk in alpha).
- Query performance degrades on documents > 1 MB (no streaming).

---

## 4. Relational Engine

**Purpose:** Traditional SQL-style tables with ACID guarantees.  
**Data layout:** Row-major, with separate B-Tree indexes for PKs and secondary
keys.

### Features

- **ACID transactions** with serialisable isolation and full rollback.
- **Foreign keys** with `CASCADE`, `SET NULL`, `SET DEFAULT`, `RESTRICT`,
  `NO ACTION` on delete/update.
- **JOIN support:** inner, left, right, cross (hash and nested-loop).
- SQL DDL: `CREATE TABLE`, `ALTER TABLE` (add/drop/modify column, add/drop
  constraint), `DROP TABLE`, `TRUNCATE`.
- Sequences (`CREATE SEQUENCE`), views (`CREATE VIEW`), triggers (`CREATE
  TRIGGER`).
- `INSERT ... RETURNING`, `UPDATE ... RETURNING`, `DELETE ... RETURNING`.
- 13 SQL-standard data types: `SmallInt`, `BigInt`, `Decimal`, `Varchar`,
  `Char`, `Timestamp`, `Time`, `Uuid`, `Enum`, `Serial`, `BigSerial`, `Money`,
  `Interval`.
- Information schema system tables.

### Ideal use-cases

- Line-of-business applications requiring strong consistency.
- Systems that need referential integrity and complex joins.
- Migrating from traditional RDBMS (PostgreSQL-style dialect).

### Limitations

- No hash partitioning or table inheritance (alpha).
- Query planner uses simple heuristics — no cost-based optimisation yet.
- `ALTER TABLE` on large tables locks the table for the duration of the
  operation.

---

## 5. Key-Value Engine

**Purpose:** CouchDB-compatible document store with MVCC.  
**Data layout:** JSON documents keyed by `_id` with `_rev` version strings.

### Features

- **MVCC** via `_id` / `_rev` — every write creates a new revision; conflicts
  are exposed to the client.
- **Mango queries** — selector-based filtering (CouchDB `_find` syntax) with
  `$eq`, `$gt`, `$lt`, `$in`, `$regex`, `$and`, `$or`, `$not`.
- **Bulk operations** (`_bulk_docs`) with `all_or_nothing` semantics.
- **Index creation** (`_index`) on arbitrary fields.
- **Collection-level encryption** (`_encrypt` / `_decrypt` endpoints) using
  AES-256-GCM.
- **Tombstone deletion** for replication and CDC support.
- `_all_docs` pagination, `_compact`, and view definitions.

### Ideal use-cases

- Session storage, user profiles, configuration.
- Mobile / offline-first apps (MVCC conflict resolution).
- Drop-in replacement for CouchDB endpoints.

### Limitations

- No secondary indexes beyond Mango selectors (no JOIN, no aggregation).
- MVCC revision chain is not automatically compacted (manual `_compact`
  required).
- Alpha — the Mango query engine does not yet use indexes for all selector
  forms.

---

## 6. Time-Series Engine

**Purpose:** Append-heavy, time-ordered numeric data.  
**Data layout:** Nanosecond-precision points grouped into metrics, chunked by
wall-clock time and indexed by tag.

### Features

- **Points & metrics** — each point carries a timestamp, string tags, and
  numeric fields; metrics hold chunking, resolution, and retention metadata.
- **Multi-resolution rollups** — `raw` plus configurable rollup resolutions
  with downsampling and aggregation.
- **15 aggregation functions** — `avg`, `sum`, `min`, `max`, `count`, `p99`,
  etc., with gap-fill policies.
- **Tag index** (`_ts_tags`) for efficient filtering by tag key/value.
- **Retention policies** — automatic pruning of expired chunks
  (`_ts_chunk_*` trees), configurable per resolution in days.
- **Chunked storage** — points grouped into daily chunks for fast range scans.

### Ideal use-cases

- Application telemetry and metrics.
- IoT sensor data collection.
- Monitoring and observability pipelines.

### Limitations

- Numerical fields only (no string/JSON field values).
- No SQL-level joins with other engines.
- Retention enforcement is triggered manually (`ts retain`) or on lifecycle
  operations, not continuously in the background.

---

## Creating a Database with a Specific Engine

Use the `primusdb db create` command with the `--engine` flag:

```bash
# Document (default)
primusdb db create mydocdb

# Columnar (analytics)
primusdb db create analytics --engine columnar

# Vector (embeddings)
primusdb db create embeddings --engine vector

# Relational (SQL)
primusdb db create appdb --engine relational

# Key-Value (CouchDB-compatible)
primusdb db create kvstore --engine keyvalue

# Time-Series
primusdb db create metrics --engine timeseries

# Under a namespace
primusdb db create analytics --engine columnar --namespace tenant1/project2
```

List available engines:

```bash
primusdb engine list --verbose
```

Inspect engine internals:

```bash
primusdb engine inspect columnar
primusdb engine inspect vector --component index
```

---

## Engine Status

| Engine      | Maturity | Transactions | Persistence   | CLI Support |
|-------------|----------|--------------|---------------|-------------|
| Columnar    | Beta     | Snapshot     | On disk (LZ4) | Full        |
| Vector      | Alpha    | None         | On disk       | Full        |
| Document    | Beta     | None         | On disk       | Full        |
| Relational  | Beta     | ACID         | On disk       | Full        |
| Key-Value   | Alpha    | MVCC         | On disk       | Full        |
| Time-Series | Alpha    | None         | On disk       | Full (`primusdb ts`) |

- **Alpha:** Core functionality works but may have rough edges, incomplete
  error handling, or missing optimisations.  Not recommended for production.
- **Beta:** Feature-complete and tested; suitable for staging and
  light-production use.
- **Stable:** (not yet reached) — production-ready with full performance
  guarantees.

See [CHANGELOG.md](../../CHANGELOG.md) for per-engine changes in each release.
