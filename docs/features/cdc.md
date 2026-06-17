# Change Data Capture (CDC)

Change Data Capture provides a stream-oriented, ordered log of every data
mutation (INSERT, UPDATE, DELETE) applied to the database. Consumers poll the
log to react to changes in near-real-time — enabling event-driven
architectures, cache invalidation, audit trails, replication pipelines, and
cross-system synchronisation.

---

## Overview

The CDC subsystem is built around an in-memory write-ahead log (WAL) that
records every mutation with a monotonically increasing sequence number,
a Unix-millisecond timestamp, the collection and document identifiers, the
type of change, and optional before/after document snapshots.

| Property              | Detail                                              |
|-----------------------|-----------------------------------------------------|
| **Tracking**          | Every INSERT, UPDATE, DELETE on a document          |
| **Ordering**          | Strict, monotonically increasing sequence numbers   |
| **Consumption**       | Poll-based (`events_after`, `get_since`)            |
| **Memory**            | Bounded VecDeque with automatic FIFO pruning        |
| **Persistence**       | Optional sled-based disk persistence                |
| **Runtime control**   | Enable / disable on the fly                         |
| **Dependencies**      | `serde`, `sled` (optional), `std`                   |

### Core types

| Type             | Role                                                    |
|------------------|---------------------------------------------------------|
| `ChangeType`     | Enum: `Insert`, `Update`, `Delete`                      |
| `ChangeEvent`    | Single mutation record with metadata and document data  |
| `CdcConfig`      | Builder-style configuration for the engine              |
| `CdcEngine`      | Main engine: owns the WAL, sequence counter, and store  |

---

## Architecture

### Write-Ahead Log (WAL)

The WAL is a `VecDeque<ChangeEvent>` with a configurable upper bound. New
events are pushed to the back; when the bound is exceeded the oldest event is
popped from the front. This gives O(1) amortized push, O(n) scan for
polling, and automatic memory bounding.

```
┌──────────────────────────────────────────────────────────────────┐
│                         CdcEngine                                 │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │              WAL (VecDeque<ChangeEvent>)                  │     │
│  │                                                           │     │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐           │     │
│  │  │ seq1 │ │ seq2 │ │ seq3 │ │ …..  │ │ seqN │           │     │
│  │  └──────┘ └──────┘ └──────┘ └──────┘ └──────┘           │     │
│  │         ▲                                    ▲           │     │
│  └─────────┼────────────────────────────────────┼───────────┘     │
│            │ record_change                      │ events_after    │
│            │                                    │ get_since       │
│  ┌─────────┴────────────────────────────────────┴───────────┐     │
│  │              Public API Layer                             │     │
│  └──────────────────────────────────────────────────────────┘     │
│                                                                   │
│  ┌──────────────────────────────────────────────────────────┐     │
│  │        Optional Persistence (sled)                        │     │
│  │        tree "changes"   →  keyed by big-endian seq        │     │
│  │        tree "meta"      →  last_sequence                  │     │
│  └──────────────────────────────────────────────────────────┘     │
└──────────────────────────────────────────────────────────────────┘
```

### Sequence numbers

Each event carries a `u64` sequence number that is incremented atomically (in
single-threaded context) on every `record_change()` call. Sequence numbers
are never reused. When the engine is cleared via `clear()` the counter
resets to 0.

### Timestamps

Timestamps are captured as Unix-millisecond integers via
`SystemTime::now()` at the moment `record_change()` is called. They are
not monotonic across events (wall-clock ordering is best-effort), but
sequence numbers provide the definitive ordering.

### Persistence layer

When a `persist_path` is provided in `CdcConfig`, the engine opens a sled
database with two trees:

- **`changes`** — each event stored under its 8-byte big-endian sequence key
- **`meta`** — stores the current `last_sequence` value

Writes happen synchronously during `record_change()` (single-event flush for
latency-sensitive workloads) and a full `persist()` call writes the entire
WAL. On engine startup with `CdcEngine::with_config()`, the sled store is
re-opened and previously persisted events are restored into memory.

---

## ChangeEvent fields

| Field          | Type                     | Description                                       |
|----------------|--------------------------|---------------------------------------------------|
| `sequence`     | `u64`                    | Monotonically increasing event ID                 |
| `timestamp`    | `u64`                    | Unix-millisecond timestamp at recording time       |
| `collection`   | `String`                 | Collection or table name                          |
| `document_id`  | `String`                 | Document or row identifier                        |
| `change_type`  | `ChangeType`             | `Insert`, `Update`, or `Delete`                   |
| `old_value`    | `Option<serde_json::Value>` | Previous document snapshot (Update, Delete)   |
| `new_value`    | `Option<serde_json::Value>` | New document snapshot (Insert, Update)        |

### ChangeType semantics per mutation

| Mutation | old_value          | new_value          |
|----------|---------------------|--------------------|
| INSERT   | `None`              | Inserted document  |
| UPDATE   | Document before     | Document after     |
| DELETE   | Deleted document    | `None`             |

---

## CdcConfig

| Field          | Type                | Default    | Description                               |
|----------------|---------------------|------------|-------------------------------------------|
| `max_wal_size` | `usize`             | `10000`    | Max WAL entries (0 = unlimited)           |
| `auto_start`   | `bool`              | `true`     | CDC active on creation                    |
| `persist_path` | `Option<PathBuf>`   | `None`     | Path for sled-backed persistence          |

### Default configuration

```rust
CdcConfig {
    max_wal_size: 10000,
    auto_start:   true,
    persist_path: None,
}
```

---

## Engine lifecycle and API

### Creating an engine

```rust
use primusdb::cdc::{CdcEngine, CdcConfig, ChangeType};

// Simple bounded engine (default 10 000 events)
let mut engine = CdcEngine::new(10_000);

// With explicit config and disk persistence
let config = CdcConfig {
    max_wal_size: 50_000,
    auto_start:   true,
    persist_path: Some("/var/lib/primusdb/cdc".into()),
};
let mut engine = CdcEngine::with_config(&config);
```

### Recording changes

```rust
let seq = engine.record_change(
    "users",                          // collection
    "user_a1b2",                      // document_id
    ChangeType::Insert,               // change type
    None,                             // old_value (None for Insert)
    Some(serde_json::json!({          // new_value
        "name": "Alice",
        "email": "alice@example.com",
    })),
);

// Update
let seq = engine.record_change(
    "users",
    "user_a1b2",
    ChangeType::Update,
    Some(serde_json::json!({"name": "Alice", "email": "alice@example.com"})),
    Some(serde_json::json!({"name": "Alice", "email": "alice@newdomain.com"})),
);

// Delete
let seq = engine.record_change(
    "users",
    "user_a1b2",
    ChangeType::Delete,
    Some(serde_json::json!({"name": "Alice"})),
    None,
);
```

Returns the sequence number assigned, or `0` if CDC is inactive.

### Consuming events

```rust
// All events after a given sequence (polling)
let events = engine.events_after(last_known_seq);

// Paginated: up to 100 events after a given sequence
let page = engine.get_since(last_known_seq, 100);

// Latest sequence number (for checkpointing)
let current_seq = engine.latest_sequence();
```

### Lifecycle management

```rust
// Enable / disable
engine.set_active(false);   // record_change() stops storing events
engine.set_active(true);    // resume recording

// Check state
let active = engine.is_active();
let count  = engine.len();
let empty  = engine.is_empty();

// Full reset
engine.clear();   // removes all events, resets sequence counter to 0
```

### Persistence operations

```rust
// Flush entire WAL to sled
engine.persist()?;

// Load WAL from sled (called automatically by with_config())
engine.load()?;
```

---

## CLI commands

CDC is exposed under the `primusdb cdc` top-level command group.

### `primusdb cdc status`

Show whether the CDC subsystem is active and optionally print verbose state.

**Usage:**
```
primusdb cdc status [OPTIONS]
```

**Options:**

| Option        | Description               | Default |
|---------------|---------------------------|---------|
| `--verbose`   | Show detailed engine info | `false` |

**Example:**
```bash
primusdb cdc status
primusdb cdc status --verbose
```

### `primusdb cdc stream`

Manage a named CDC stream. Streams are the user-facing abstraction that
wraps an underlying `CdcEngine` instance.

**Usage:**
```
primusdb cdc stream <NAME> [OPTIONS]
```

**Arguments:**

| Argument | Description    |
|----------|----------------|
| `NAME`   | Stream name    |

**Options:**

| Option             | Description                           |
|--------------------|---------------------------------------|
| `-t, --table`      | Table to capture changes from         |
| `--create`         | Create the stream                     |
| `--stop`           | Stop (pause) the stream               |
| `--delete`         | Delete the stream                     |

**Examples:**
```bash
# Create a new stream tracking the "orders" table
primusdb cdc stream orders_stream --create --table orders

# Pause the stream (events still accumulate in WAL)
primusdb cdc stream orders_stream --stop

# Delete the stream and its WAL
primusdb cdc stream orders_stream --delete
```

### `primusdb cdc subscribe`

Subscribe to a CDC stream and begin consuming change events.

**Usage:**
```
primusdb cdc subscribe <STREAM> [OPTIONS]
```

**Arguments:**

| Argument | Description                  |
|----------|------------------------------|
| `STREAM` | Stream name to subscribe to  |

**Options:**

| Option             | Description                          | Default |
|--------------------|--------------------------------------|---------|
| `--from-start`     | Read from the beginning of the stream | `false` |
| `--offset`         | Start from a specific offset          | —       |
| `--format`         | Output format for events             | `json`  |

**Output formats:** `json`, `avro`, `raw`

**Examples:**
```bash
# Subscribe from the earliest available event
primusdb cdc subscribe orders_stream --from-start --format json

# Subscribe starting from a specific offset
primusdb cdc subscribe orders_stream --offset "2025-01-01T00:00:00Z"

# Subscribe with Avro-encoded output
primusdb cdc subscribe orders_stream --from-start --format avro
```

### `primusdb cdc offsets`

Inspect or manipulate consumer offsets for a CDC stream. Offsets track how
far a consumer has read through the WAL.

**Usage:**
```
primusdb cdc offsets <STREAM> [OPTIONS]
```

**Arguments:**

| Argument | Description   |
|----------|---------------|
| `STREAM` | Stream name   |

**Options:**

| Option          | Description                           |
|-----------------|---------------------------------------|
| `--partitions`  | Show per-partition offset information |
| `--set`         | Manually set the consumer offset      |

**Examples:**
```bash
# View current offset
primusdb cdc offsets orders_stream

# Show per-partition offsets
primusdb cdc offsets orders_stream --partitions

# Rewind consumer to a specific timestamp
primusdb cdc offsets orders_stream --set "2025-01-15T00:00:00Z"
```

---

## Configuration options

CDC can be configured at the engine level via `CdcConfig` or at the
application level via a TOML configuration file for the PrimusDB server.

### Engine-level configuration (embedded usage)

```rust
use primusdb::cdc::CdcConfig;

let config = CdcConfig {
    // Keep at most 100 000 events in memory.
    // Older events are evicted FIFO.
    max_wal_size: 100_000,

    // Start capturing changes immediately.
    auto_start: true,

    // Persist events to disk so they survive a restart.
    persist_path: Some("/data/primusdb/cdc".into()),
};
```

### TOML config file (server mode)

When using the PrimusDB server, CDC settings can be placed in the server
configuration file (e.g. `primusdb.toml`):

```toml
[cdc]
# Maximum WAL entries in memory (0 = unlimited)
max_wal_size = 100000

# Start CDC on server boot
auto_start = true

# Optional path for persistent storage via sled.
# If omitted, CDC is in-memory only.
persist_path = "/var/lib/primusdb/cdc"
```

### Important sizing notes

- Each `ChangeEvent` carries two optional `serde_json::Value` fields that
  can be large. A `max_wal_size` of 10 000 events with 4 KB document
  snapshots consumes roughly 80 MB.
- Setting `max_wal_size = 0` disables pruning — memory grows unboundedly
  until `clear()` is called or the process terminates.
- Disk persistence via sled adds write amplification but guarantees
  crash recovery. Each `record_change()` call performs a single-event
  sled write (synchronous).

---

## Use cases

### Real-time data pipelines

Forward mutations to an external system such as Elasticsearch, Kafka, or
a cache.

```
Application  →  PrimusDB  ──  CDC WAL  ──  poll  ──  Pipeline  ──  Elasticsearch
```

```rust
let mut last_seq = 0;
loop {
    let events = engine.events_after(last_seq);
    for event in &events {
        match event.change_type {
            ChangeType::Insert | ChangeType::Update => {
                // Re-index in Elasticsearch
                index_document(&event.collection, &event.document_id, &event.new_value);
            }
            ChangeType::Delete => {
                // Remove from search index
                remove_document(&event.collection, &event.document_id);
            }
        }
        last_seq = event.sequence;
    }
    std::thread::sleep(Duration::from_millis(100));
}
```

### Audit logging

Record every mutation with the full before-and-after document state for
compliance and forensic analysis.

```rust
engine.record_change(
    "accounts",
    &account_id,
    ChangeType::Update,
    Some(old_balance),   // previous document
    Some(new_balance),   // new document
);
```

An external audit consumer reads the CDC stream and writes events to
append-only object storage.

### Cache invalidation

Invalidate a distributed cache (e.g. Redis) whenever a document is updated
or deleted.

```rust
// In the application layer after a mutation:
let seq = engine.record_change("products", &product_id, ChangeType::Update, old, new);

// A background worker polls and invalidates:
let events = engine.events_after(cache_last_seq);
for e in &events {
    if e.change_type == ChangeType::Update || e.change_type == ChangeType::Delete {
        cache.invalidate(&format!("{}:{}", e.collection, e.document_id));
    }
}
```

### Cross-region replication

A downstream PrimusDB instance polls the CDC stream of a primary instance
and applies the same mutations, keeping two regions eventually consistent.

### Analytical materialised views

Maintain a denormalised materialised view (e.g. a reporting table) by
reacting to changes in source tables.

```
Source: orders  ──┐
Source: payments ──┤── CDC WAL ──  materialiser  ──  reporting.orders_with_payments
Source: users    ──┘
```

---

## Comparison: stream vs. poll

| Approach  | Mechanism                         | Best for                                 |
|-----------|-----------------------------------|------------------------------------------|
| Poll      | `events_after()` / `get_since()`  | Simple consumers, batch jobs, audit logs |
| Subscribe | Long-lived CLI/API subscription   | Real-time dashboards, live pipelines     |

The engine currently provides poll-based consumption. The CLI `subscribe`
command is a convenience that wraps polling with automatic checkpointing and
continuous output.

---

## Testing and validation

The CDC module includes a comprehensive test suite in `cdc.rs`:

| Test                                  | What it validates                                   |
|---------------------------------------|-----------------------------------------------------|
| `test_record_and_retrieve`            | Basic record → retrieve round-trip                  |
| `test_events_after`                   | Polling with exclusive sequence boundary            |
| `test_max_wal_size_prunes`            | FIFO eviction when the WAL exceeds the limit        |
| `test_inactive_no_recording`          | `set_active(false)` suppresses recording            |
| `test_since_with_limit`               | `get_since()` with limit and offset                 |
| `test_sequence_monotonic`             | Strictly increasing sequence numbers                |
| `test_old_value_for_update`           | UPDATE carries correct old/new values               |
| `test_new_value_for_insert`           | INSERT carries new_value, old_value is None         |
| `test_delete_has_no_new_value`        | DELETE carries old_value, new_value is None         |
| `test_empty_wal`                      | Initial state                                       |
| `test_cdc_config_default`             | Default config values                               |
| `test_clear_resets_engine`            | `clear()` removes all events and resets sequence    |
| `test_unlimited_wal`                  | `max_wal_size = 0` never prunes                     |
| `test_timestamp_is_set`               | Events always have a non-zero timestamp             |
| `test_persist_and_load_roundtrip`     | Full sled persist → crash → restore round-trip      |
| `test_persist_no_path_returns_error`  | `persist()` fails gracefully when no path set       |
| `test_load_after_persist_preserves_all_fields` | All ChangeEvent fields survive serialisation |
| `test_events_after_unknown_sequence`  | Polling past the end returns empty vec              |
| `test_get_since_exact_boundary`       | `get_since()` correctness at boundary values        |

Run the tests with:

```bash
cargo test --lib cdc
```

---

## Code layout

| Path                    | Contents                               |
|-------------------------|----------------------------------------|
| `src/cdc.rs`            | `CdcEngine`, `ChangeEvent`, `ChangeType`, `CdcConfig`, tests |
| `src/cli/command.rs`    | `CdcSubcommands` enum with CLI definitions |
| `src/cli/cmd/cdc.rs`    | CLI handler dispatching subcommands     |
| `src/cli/mod.rs`        | Top-level routing for `primusdb cdc`    |

---

## Thread safety

Currently `CdcEngine` is **not** `Send` or `Sync`. All operations are
intended to be called from a single thread or externally synchronised (e.g.
behind a `Mutex<CdcEngine>`). The engine does not use internal locks.

Future versions may introduce a concurrent-friendly design with
lock-free WAL appends and separate read handles.
