# Resource Policies Reference

Complete reference for Resource Governor policy configuration.

---

## Configuration File

Policies are defined in `config/governor.toml`:

```toml
# Global default — applies to all workloads unless overridden
[[policies]]
name = "default"
scope = "global"
action = "monitor"
max_memory_mb = 2048
max_execution_steps = 10_000_000
max_cpu_time_ms = 300_000
max_query_complexity = 100
max_join_count = 10
max_sort_rows = 1_000_000
max_pipeline_depth = 100
max_pipeline_stages = 50
max_ffi_calls = 10_000
max_ffi_memory_mb = 512
max_ffi_time_ms = 30_000
max_training_iterations = 100_000
max_prediction_batch_size = 10_000
max_embedding_batch_size = 10_000
max_vector_candidates = 100_000
max_vector_expansions = 100
max_graph_depth = 100
max_graph_nodes = 1_000_000
max_graph_edges = 10_000_000
max_import_rows = 10_000_000
max_import_batches = 10_000
max_backup_size = 107374182400
max_restore_size = 107374182400
```

### Per-namespace override

```toml
[[policies]]
name = "analytics"
scope = "namespace:analytics"
action = "warn"
max_memory_mb = 4096
```

### Per-user override

```toml
[[policies]]
name = "admin-user"
scope = "user:admin"
action = "monitor"
max_memory_mb = 16384
max_execution_steps = 100_000_000
```

### Per-workload override

```toml
[[policies]]
name = "vector-heavy"
scope = "workload_type:vector_search"
action = "throttle"
max_vector_candidates = 500_000
max_vector_expansions = 200
```

### Per-role override

```toml
[[policies]]
name = "readonly-role"
scope = "role:readonly"
action = "block"
max_memory_mb = 512
max_execution_steps = 10_000
```

---

## Limit Reference

### CPU

| Field               | Type     | Default      | Description                  |
|---------------------|----------|--------------|------------------------------|
| `max_execution_steps` | `u64`  | `10_000_000` | Maximum logical operations   |
| `max_cpu_time_ms`    | `u64`    | `300_000`    | Maximum CPU time (5 min)     |

### Memory

| Field           | Type   | Default  | Description                  |
|-----------------|--------|----------|------------------------------|
| `max_memory_mb` | `u64`  | `2048`   | Maximum memory in MiB        |

### Query Complexity

| Field                 | Type   | Default       | Description                  |
|-----------------------|--------|---------------|------------------------------|
| `max_query_complexity` | `u32` | `100`         | Maximum query complexity     |
| `max_join_count`       | `u32` | `10`          | Maximum JOINs per query     |
| `max_sort_rows`        | `u64` | `1_000_000`   | Maximum rows to sort        |

### Pipeline

| Field                | Type   | Default  | Description                  |
|----------------------|--------|----------|------------------------------|
| `max_pipeline_depth` | `u32`  | `100`    | Maximum pipeline depth       |
| `max_pipeline_stages`| `u32`  | `50`     | Maximum pipeline stages      |

### FFI

| Field              | Type   | Default   | Description                  |
|--------------------|--------|-----------|------------------------------|
| `max_ffi_calls`    | `u64`  | `10_000`  | Maximum FFI calls            |
| `max_ffi_memory_mb`| `u64`  | `512`     | Maximum FFI memory in MiB    |
| `max_ffi_time_ms`  | `u64`  | `30_000`  | Maximum FFI time in ms       |

### AI/ML

| Field                       | Type   | Default    | Description                  |
|-----------------------------|--------|------------|------------------------------|
| `max_training_iterations`   | `u64`  | `100_000`  | Maximum training iterations  |
| `max_prediction_batch_size` | `u64`  | `10_000`   | Maximum prediction batch     |
| `max_embedding_batch_size`  | `u64`  | `10_000`   | Maximum embedding batch      |

### Vector

| Field                   | Type   | Default    | Description                  |
|-------------------------|--------|------------|------------------------------|
| `max_vector_candidates` | `u64`  | `100_000`  | Maximum vector candidates    |
| `max_vector_expansions` | `u64`  | `100`      | Maximum vector expansions    |

### Graph

| Field            | Type   | Default       | Description                  |
|------------------|--------|---------------|------------------------------|
| `max_graph_depth`| `u32`  | `100`         | Maximum traversal depth      |
| `max_graph_nodes`| `u64`  | `1_000_000`   | Maximum visited nodes        |
| `max_graph_edges`| `u64`  | `10_000_000`  | Maximum visited edges        |

### Migration

| Field              | Type   | Default       | Description                  |
|--------------------|--------|---------------|------------------------------|
| `max_import_rows`  | `u64`  | `10_000_000`  | Maximum rows to import       |
| `max_import_batches`| `u64` | `10_000`      | Maximum import batches       |

### Backup / Restore

| Field              | Type   | Default         | Description                  |
|--------------------|--------|-----------------|------------------------------|
| `max_backup_size`  | `u64`  | `107374182400`  | Max backup size (100 GiB)    |
| `max_restore_size` | `u64`  | `107374182400`  | Max restore size (100 GiB)   |

---

## Scope Reference

| Scope            | Format                      | Example                       |
|------------------|-----------------------------|-------------------------------|
| Global           | `global`                    | `scope = "global"`            |
| Cluster          | `cluster:<name>`            | `scope = "cluster:prod"`      |
| Node             | `node:<id>`                 | `scope = "node:node-1"`       |
| Namespace        | `namespace:<name>`          | `scope = "namespace:analytics"`|
| Database         | `database:<name>`           | `scope = "database:appdb"`    |
| Role             | `role:<name>`               | `scope = "role:readonly"`     |
| User             | `user:<name>`               | `scope = "user:alice"`        |
| Workload Type    | `<workload_type>`           | `scope = "sql"`               |

### Workload type identifiers

| Identifier          | Variant            |
|---------------------|--------------------|
| `sql`               | Sql                |
| `vector_search`     | VectorSearch       |
| `ai_ml`             | AIML               |
| `graph_traversal`   | GraphTraversal     |
| `cdc_pipeline`      | CdcPipeline        |
| `backup`            | Backup             |
| `restore`           | Restore            |
| `migration`         | Migration          |
| `ffi`               | Ffi                |

Scopes in `config/governor.toml` are parsed as raw strings. For workload
types, use the short identifier (e.g., `sql`, `vector_search`). For typed
scopes, use `type:value` syntax (e.g., `namespace:analytics`).

---

## Enforcement Actions

| Action     | Description                                    |
|------------|------------------------------------------------|
| `monitor`  | Track and log only, never interfere            |
| `warn`     | Log a warning, allow execution to continue     |
| `throttle` | Log and signal the caller to reduce throughput |
| `block`    | Log and abort the execution with an error      |

---

## Inheritance & Override Semantics

When multiple policies match an execution, they are merged from least
specific to most specific scope. Non-`None` fields from more specific
scopes override the less specific values. Fields set to `None` in a more
specific scope inherit from the less specific scope.

Example:

| Policy       | Scope          | max_memory_mb | max_execution_steps | action  |
|--------------|----------------|---------------|---------------------|---------|
| `default`    | global         | 2048          | 10_000_000          | monitor |
| `analytics`  | namespace:analytics | 4096    | None                | warn    |
| **Result**   |                | **4096**      | **10_000_000**      | **warn**|
