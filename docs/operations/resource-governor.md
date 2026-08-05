# Resource Governor Operations

This guide covers day-to-day operations for the PrimusDB Resource Governor.

---

## Enabling / Disabling

The governor is enabled by default. To disable it at startup:

```toml
# config/governor.toml
enabled = false
```

To check whether the governor is currently enabled:

```bash
primusdb governor status
```

---

## Monitoring Active Executions

```bash
# Show current status
primusdb governor status

# Live metrics with 2-second refresh
primusdb governor metrics --watch

# Inspect a specific execution
primusdb governor inspect <execution-uuid>
```

### Understanding status fields

| Field              | Description                                |
|--------------------|--------------------------------------------|
| Enabled            | Whether the governor is active             |
| Active Executions  | Currently tracked executions               |
| Total Violations   | Cumulative policy violations               |
| Blocked            | Executions blocked since engine start      |
| Throttled          | Executions throttled since engine start    |
| Policies Loaded    | Number of configured policies              |
| Uptime (s)         | Seconds since engine started               |

---

## Viewing Violations

```bash
# All violations
primusdb governor violations

# Last 10 violations
primusdb governor violations --last 10

# Filter by workload type
primusdb governor violations --workload sql

# Limit output
primusdb governor violations --limit 5
```

### Interpreting violations

```
ID          Execution       Namespace  Workload  Limit         Limit Value  Usage  Action    Timestamp
v-uuid-1    e-uuid-1        analytics  sql       max_memory_mb 2048         5120   blocked   2025-01-01T...
```

When a violation occurs:
- **Monitor**: Logged but no action taken
- **Warn**: Logged with `WARN` level, execution continues
- **Throttle**: Logged, caller is told to slow down
- **Block**: Logged, execution is aborted with an error

---

## Managing Policies

```bash
# List all policies
primusdb governor policies

# List a specific policy
primusdb governor policies --name ad-hoc-sql

# Create / update a policy
primusdb governor set analytics-policy \
  --scope "namespace:analytics" \
  --action block \
  --max-memory-mb 1024 \
  --max-execution-steps 1000000
```

### Scope resolution

Policies inherit by scope specificity. When resolving a policy for an
execution, all matching policies are collected and sorted by scope priority
(least specific → most specific). More specific values override less specific
ones:

```
Global (0)          ← Cluster (1)        ← Node (2)
← Namespace (3)     ← Database (4)       ← Role (5)
← User (6)          ← WorkloadType (7)
```

---

## REST API

All endpoints are under `/api/v1/governor/` and return JSON.

```bash
# Status
curl http://localhost:8080/api/v1/governor/status

# Policies
curl http://localhost:8080/api/v1/governor/policies

# Metrics
curl http://localhost:8080/api/v1/governor/metrics

# Violations
curl http://localhost:8080/api/v1/governor/violations

# Active executions
curl http://localhost:8080/api/v1/governor/executions

# Start an execution (returns execution_id + action)
curl -X POST http://localhost:8080/api/v1/governor/executions/start \
  -H "Content-Type: application/json" \
  -d '{"namespace":"analytics","workload_type":"sql","user":"alice"}'

# Finish an execution
curl -X POST http://localhost:8080/api/v1/governor/executions/<id>/finish \
  -H "Content-Type: application/json" -d '{}'

# Check a resource limit
curl -X POST http://localhost:8080/api/v1/governor/executions/<id>/check \
  -H "Content-Type: application/json" \
  -d '{"check_type":"max_memory_mb","value":1024}'

# Create or update a policy
curl -X POST http://localhost:8080/api/v1/governor/policies/update \
  -H "Content-Type: application/json" \
  -d '{"name":"my-policy","limits":{"cpu":{"max_cpu_time_ms":60000}},"action":"warn","scope":"namespace:analytics"}'
```

---

## Prometheus Metrics

Available at `/metrics`:

```
# HELP primusdb_governor_active_executions Currently active executions
# TYPE primusdb_governor_active_executions gauge
primusdb_governor_active_executions 3

# HELP primusdb_governor_blocked_total Total blocked executions
# TYPE primusdb_governor_blocked_total counter
primusdb_governor_blocked_total 42

# HELP primusdb_governor_throttled_total Total throttled executions
# TYPE primusdb_governor_throttled_total counter
primusdb_governor_throttled_total 7

# HELP primusdb_governor_policy_violations_total Total policy violations
# TYPE primusdb_governor_policy_violations_total counter
primusdb_governor_policy_violations_total 49

# HELP primusdb_governor_memory_usage_bytes Current memory usage from active executions
# TYPE primusdb_governor_memory_usage_bytes gauge
primusdb_governor_memory_usage_bytes 104857600

# HELP primusdb_governor_cpu_time_ms Current CPU time from active executions
# TYPE primusdb_governor_cpu_time_ms gauge
primusdb_governor_cpu_time_ms 15000

# HELP primusdb_governor_ffi_calls_total Total FFI calls from active executions
# TYPE primusdb_governor_ffi_calls_total counter
primusdb_governor_ffi_calls_total 128
```

---

---

## Logging

Violations are logged via `tracing` with target `GovernorViolation`:

```
WARN GovernorViolation: Execution: query-abc Namespace: analytics
Policy: memory-limit Limit: max_memory_mb = 2048 Usage: max_memory_mb = 5120
Action: blocked
```

Query contents are not logged unless debug mode is enabled.

---

## Troubleshooting

### Governor not enforcing limits

1. Check `primusdb governor status` — is `Enabled: true`?
2. Check `primusdb governor policies` — is the expected policy present?
3. Verify the scope matches the execution (namespace, workload type, etc.)
4. Confirm the policy's action is not `monitor`

### Too many false positives

1. Increase the relevant limit: `primusdb governor set default --max-memory-mb 4096`
2. Change the action to `warn` instead of `block` while tuning
3. Check active executions with `primusdb governor inspect <id>` for the actual usage

### Violations not showing

1. The violation ring buffer caps at 10,000 entries (oldest are drained)
2. Use `primusdb governor metrics` to see the total count even if details are evicted
