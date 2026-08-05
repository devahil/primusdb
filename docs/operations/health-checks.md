# Health Checks

PrimusDB exposes several endpoints and CLI commands to monitor server health and diagnose issues.

## GET /health

Basic health check endpoint. Returns a lightweight response indicating whether the server is alive and accepting requests.

```bash
curl http://localhost:8080/health
```

**Example Response:**

```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "node_id": "node-abc123",
    "instance_id": "instance-xyz789",
    "version": "1.3.2-alpha",
    "uptime_seconds": 3600,
    "architecture": "centralized"
  }
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | `"healthy"` or `"unhealthy"` |
| `node_id` | string | Unique node identifier |
| `instance_id` | string | Unique instance identifier |
| `version` | string | PrimusDB version string |
| `uptime_seconds` | number | Process uptime |
| `architecture` | string | Deployment architecture (`"centralized"` for a single-node install) |

**HTTP Status Codes:**

- `200 OK` — Server is healthy
- `503 Service Unavailable` — Server is not ready or in a failure state

## GET /status

Detailed system status including storage engines, cluster state, and feature availability.

```bash
curl http://localhost:8080/status
```

**Example Response:**

```json
{
  "success": true,
  "data": {
    "status": "running",
    "version": "1.3.2-alpha",
    "uptime_seconds": 3600,
    "storage_engines": {
      "columnar": "available",
      "vector": "available",
      "document": "available",
      "relational": "available",
      "keyvalue": "available"
    },
    "ai_enabled": true,
    "cache_enabled": true,
    "transactions_enabled": true
  }
}
```

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `status` | string | Overall server status (`"running"`) |
| `version` | string | Server version |
| `uptime_seconds` | number | Process uptime |
| `storage_engines` | object | Per-engine availability status |
| `ai_enabled` | boolean | Whether the AI/ML subsystem is active |
| `cache_enabled` | boolean | Whether caching is active |
| `transactions_enabled` | boolean | Whether transaction support is active |

## `primusdb server health`

Check server health from the CLI without needing `curl`:

```bash
# Basic health check
primusdb server health

# Deep health check (checks all subsystems)
primusdb server health --deep
```

The `--deep` flag performs additional checks including storage engine health, cache status, and cluster connectivity.

## `primusdb connect`

Test connectivity to a running PrimusDB instance:

```bash
# Connect to default server
primusdb connect

# Connect to a specific server
primusdb connect http://192.168.1.100:8080

# Connect with longer timeout
primusdb connect http://192.168.1.100:8080 --timeout 30
```

The command hits the `/health` endpoint and reports success or failure:

```
Connected to http://localhost:8080
```

or:

```
Connection failed: error sending request for url (http://localhost:8080/health)
```

## `primusdb doctor`

Run full diagnostic checks on the system. See the [Doctor](doctor.md) page for complete details.

```bash
# Quick diagnostic
primusdb doctor

# Comprehensive diagnostics with disk space check
primusdb doctor --aggressive

# Save report to a file
primusdb doctor --aggressive --report /tmp/diagnostic-report.txt

# JSON output
primusdb doctor --format json
```

## Example Curl Commands

```bash
# Basic health check
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/status

# Health with pretty-printed JSON
curl http://localhost:8080/health | jq .

# Check only the status field
curl -s http://localhost:8080/health | jq .data.status

# Check uptime
curl -s http://localhost:8080/health | jq .data.uptime_seconds

# Check engine availability
curl -s http://localhost:8080/status | jq .data.engines

# Prometheus metrics
curl http://localhost:8080/metrics
```

## Using for Monitoring

### Nagios / Icinga

```bash
./check_http -H localhost -p 8080 -u /health -e "healthy"
```

### Prometheus Blackbox Exporter

```yaml
modules:
  http_2xx:
    prober: http
    timeout: 5s
    http:
      valid_status_codes: [200]
      method: GET
      preferred_ip_protocol: ip4
```

### Custom Shell Script

```bash
#!/bin/bash
HEALTH=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health)
if [ "$HEALTH" = "200" ]; then
    echo "PrimusDB is healthy"
    exit 0
else
    echo "PrimusDB health check failed (HTTP $HEALTH)"
    exit 2
fi
```

## Troubleshooting

### Server Not Reachable

```bash
# Check if the server process is running
ps aux | grep primusdb

# Check if the port is listening
netstat -tlnp | grep 8080

# Try a raw TCP connection
telnet localhost 8080
```

### Health Returns 503

- Server has not finished initializing — wait a few seconds and retry
- Storage engine failed to open — check the server logs
- Cluster node is isolated — verify cluster connectivity

### Status Shows Engine Unavailable

- Check that the data directory is accessible and has sufficient disk space
- Verify the storage engine configuration in `primusdb.toml`
- Look for engine-specific errors in the server log

### High Uptime but Slow Responses

- Run `primusdb doctor --aggressive` for a full system diagnostic
- Check `GET /metrics` for resource usage patterns
- Review server logs for slow query warnings
