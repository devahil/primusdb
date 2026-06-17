# Execution Governance & Resource Management (Governor)

The Governor subsystem prevents runaway queries, resource exhaustion, and
uncontrolled workloads across all PrimusDB engines — SQL, vector, AI/ML,
graph, CDC, backup, restore, migration, cluster ops, protocol, UDFs, stored
procedures, plugins, and FFI.

---

## Overview

Every execution — whether a SQL query, a vector similarity search, a graph
traversal, or a backup job — is tracked against a configurable policy.
When a limit is exceeded the Governor can **monitor**, **warn**, **throttle**,
or **block** the execution depending on the policy action.

| Property                | Detail                                                   |
|-------------------------|----------------------------------------------------------|
| **Scope**               | Global, cluster, node, namespace, database, role, user, workload type |
| **Resources**           | CPU, memory, query complexity, pipeline steps, FFI calls, AI/ML tokens, vector candidates, graph depth, migration/import rows, backup size |
| **Enforcement**         | Monitor / Warn / Throttle / Block                        |
| **Tracking**            | Per-execution context with live active-execution registry |
| **Violations**          | Capped ring buffer (10k entries, auto-drain to 5k)       |
| **Metrics**             | Prometheus 7-metric suite + runtime snapshot              |
| **API**                 | 9 REST endpoints (5 GET + 4 POST) + 6 CLI subcommands    |
| **TUI**                 | Dedicated Governor panel with live status + violations    |

### Workload types

| Variant          | Description                            |
|------------------|----------------------------------------|
| `Sql`            | SQL queries and statements             |
| `Vector`         | Vector index operations                |
| `AiMl`           | AI/ML model inference or training      |
| `Graph`          | Graph traversals and mutations         |
| `Cdc`            | Change data capture streams            |
| `Backup`         | Backup operations                      |
| `Restore`        | Restore operations                     |
| `Migration`      | Schema or data migration               |
| `ClusterOp`      | Cluster management operations          |
| `Protocol`       | Wire-protocol handling                 |
| `Udf`            | User-defined function execution        |
| `StoredProcedure`| Stored procedure execution             |
| `Plugin`         | Third-party plugin execution           |
| `Ffi`            | Foreign function interface calls       |

### Enforcement actions

| Action     | Behaviour                                               |
|------------|---------------------------------------------------------|
| **Monitor**| Log the usage, do not interfere (default for no policy) |
| **Warn**   | Log a violation warning, allow the execution to continue|
| **Throttle**| Log and suggest the caller slow down                   |
| **Block**  | Return an error, abort the execution                    |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        GovernorEngine                           │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ PolicyManager│  │ ActiveExecs  │  │ ViolationRing (10k)    │ │
│  │  - config    │  │  HashMap     │  │  VecDeque<Violation>   │ │
│  │  - scope     │  │  <Uuid, Ctx> │  │  (auto-drain at 5k)   │ │
│  │  - resolve() │  └──────────────┘  └────────────────────────┘ │
│  └──────┬───────┘                                                │
│         │                                                        │
│  ┌──────▼───────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ Global Stats  │  │ Prometheus   │  │     Log (tracing)      │ │
│  │  (AtomicU64) │  │  (7 metrics) │  │  warn!("violation")    │ │
│  └──────────────┘  └──────────────┘  └────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
         │                    ▲
         │  start/finish      │ check_limit
         ▼                    │
┌─────────────────────────────────────────────────────────────────┐
│                       Executing Code                             │
│  (SQL engine, vector index, graph engine, CDC, backup, …)       │
└─────────────────────────────────────────────────────────────────┘
```

### Execution lifecycle

1. **`start_execution(namespace, workload_type, user, role)`** — create an
   `ExecutionContext`, resolve the applicable policy, increment the active
   execution counter, and return an `ExecutionHandle`.
2. **`check_limit`, `check_memory`, `check_steps`** — called periodically by
   the executing code. If the limit is exceeded, the configured enforcement
   action is returned (or an error for `Block`).
3. **`finish_execution(id)`** — remove the context from the active registry,
   decrement the active execution counter.

### Policy inheritance

Policies are resolved by scope specificity. Override semantics apply:
more specific scopes override less specific ones.

```
Global  ←  Cluster  ←  Node  ←  Namespace  ←  Database
                                               ↓
                              Role  ←  User  ←  WorkloadType
```

When resolving, the policy manager collects all matching policies and merges
them from least-specific to most-specific. Non-`None` fields in a more
specific scope override the less specific value.

---

## Configuration

### Default limits

| Limit                | Default     |
|----------------------|-------------|
| max_memory_mb        | 2048 (2 GB) |
| max_execution_steps  | 10,000,000  |
| max_cpu_time_ms      | 300,000 (5 min) |
| max_vector_candidates| 100,000     |
| max_graph_depth      | 100         |
| max_import_rows      | 10,000,000  |
| max_backup_size_mb   | 10,000      |
| max_ai_tokens        | 100,000     |
| max_ffi_calls        | 10,000      |
| action               | "monitor"   |

### config/governor.toml

```toml
[[policies]]
name = "default"
scope = "global"
action = "monitor"
max_memory_mb = 2048
max_execution_steps = 10_000_000
max_cpu_time_ms = 300_000
max_vector_candidates = 100_000
max_graph_depth = 100
max_import_rows = 10_000_000
max_backup_size_mb = 10_000
max_ai_tokens = 100_000
max_ffi_calls = 10_000

[[policies]]
name = "ad-hoc-sql"
scope = "workload_type:Sql"
action = "block"
max_memory_mb = 512

[[policies]]
name = "ollama-user"
scope = "user:ollama"
action = "warn"
max_ai_tokens = 500_000
```

---

## CLI

### `primusdb governor status`

Show whether the governor is enabled and how many executions are active.

### `primusdb governor policies`

List all configured policies with scope, action, and limits.

### `primusdb governor inspect <id>`

Inspect a specific execution by UUID.

### `primusdb governor metrics [--watch]`

Show a live metrics snapshot (active executions, blocked/throttled/total
violations, memory usage, CPU time, FFI calls). Pass `--watch` to poll
every 2 seconds.

### `primusdb governor violations [--last]`

Show recent violations. With `--last`, show only the most recent N.

### `primusdb governor set <name>`

Create or update a policy. Flags:

```
--scope <SCOPE>            e.g. global, namespace:prod, user:alice
--action <ACTION>          monitor|warn|throttle|block
--max-memory-mb <MB>
--max-execution-steps <N>
--max-cpu-time-ms <MS>
--max-vector-candidates <N>
--max-graph-depth <N>
--max-import-rows <N>
--max-backup-size-mb <MB>
--max-ai-tokens <N>
--max-ffi-calls <N>
```

Output format is controlled by `--format table|json|csv|yaml|plain`.

---

## REST API

| Method | Path                                     | Description                  |
|--------|------------------------------------------|------------------------------|
| GET    | `/api/v1/governor/status`                | Enabled + active executions  |
| GET    | `/api/v1/governor/policies`              | All configured policies      |
| GET    | `/api/v1/governor/metrics`               | Metrics snapshot             |
| GET    | `/api/v1/governor/violations`            | Recent violations            |
| GET    | `/api/v1/governor/executions`            | Active executions            |
| POST   | `/api/v1/governor/executions/start`      | Start a tracked execution    |
| POST   | `/api/v1/governor/executions/:id/finish` | Finish an execution          |
| POST   | `/api/v1/governor/executions/:id/check`  | Check a resource limit       |
| POST   | `/api/v1/governor/policies/update`       | Create or update a policy    |

All endpoints return JSON. Example:

```bash
curl http://localhost:8080/api/v1/governor/status
{"enabled":true,"active_executions":3}

# Start an execution
curl -X POST http://localhost:8080/api/v1/governor/executions/start \
  -H "Content-Type: application/json" \
  -d '{"namespace":"analytics","workload_type":"sql"}'
```

---

## Prometheus metrics

| Metric                                   | Type    | Description                 |
|------------------------------------------|---------|-----------------------------|
| `primusdb_governor_active_executions`    | Gauge   | Currently active executions |
| `primusdb_governor_blocked_total`        | Counter | Total blocked executions    |
| `primusdb_governor_throttled_total`      | Counter | Total throttled executions  |
| `primusdb_governor_policy_violations_total` | Counter | Total policy violations   |
| `primusdb_governor_memory_usage_bytes`   | Gauge   | Current memory usage        |
| `primusdb_governor_cpu_time_ms`          | Gauge   | Current CPU time            |
| `primusdb_governor_ffi_calls_total`      | Counter | Total FFI calls             |

---

## TUI

The Governor panel in the TUI (activated via the `Navigation` sidebar or the
keybindings below) shows four sections:

| Key | Section          |
|-----|------------------|
| `s` | Status (enabled, active count) |
| `v` | Recent violations (up to 8)    |
| `m` | Metrics snapshot              |
| `r` | Refresh all                  |

---

## Logging

All violations are logged via `tracing::warn!(target: "GovernorViolation")`
with the execution ID, violated policy, limit exceeded, actual usage, and
enforcement action. Query contents are **not** logged unless debug mode is
enabled.

---

## Implementation

### Files

| File                              | Lines | Role                         |
|-----------------------------------|-------|------------------------------|
| `src/governor/mod.rs`            | 553   | Core data structures + types |
| `src/governor/policy.rs`         | 510   | PolicyManager + scope resolution |
| `src/governor/engine.rs`         | 1198  | GovernorEngine + execution lifecycle |
| `src/cli/cmd/governor.rs`        | 367   | CLI subcommands              |
| `src/api/mod.rs`                 | ~300  | REST endpoints               |
| `src/cli/tui/sections/governor.rs`| ~110 | TUI render function          |
| `src/metrics.rs`                 | ~30   | Prometheus registration      |

### Key design decisions

- **Module location**: `src/governor/` (not a separate crate) for tight
  integration with PrimusDB internals; follows the same pattern as `src/cdc/`,
  `src/auth/`.
- **Static engine instance**: API handlers use a `OnceLock`-initialized
  static instance (same pattern as existing health/metrics endpoints) rather
  than threading through `AppState`.
- **Per-instance counters**: `active_executions`, `total_blocked`,
  `total_throttled`, and `total_violations` are `AtomicU64`/`AtomicUsize`
  fields on the `Inner` struct for lock-free reads without global state.
- **Violation ring**: bounded `VecDeque` with a 10k-entry cap; when full
  the oldest 5k entries are drained to keep memory bounded.
