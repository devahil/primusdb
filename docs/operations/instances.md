# Managing PrimusDB Instances

Instance management commands let you discover, inspect, connect to, stop, and monitor PrimusDB instances running locally or on the network.

## Command Overview

```
primusdb instance
├── list           List all local instances (default ports)
├── discover       Scan for instances on a host/port range
├── inspect        Show detailed instance information
├── connect        Test connectivity to an instance
├── stop           Stop a running instance
└── logs           View instance logs (alpha limitation)
```

---

## `primusdb instance list`

Lists all running PrimusDB instances found by probing default ports on localhost and scanning config files for known endpoints.

**Usage:**
```bash
primusdb instance list
primusdb instance list --all
primusdb instance list --format json
```

**Discovery mechanism:**
1. Probes ports `8080`–`8083` and `9090`–`9093` on `127.0.0.1`
2. Checks known config file paths (`primusdb.toml`, `config.toml`, `~/.config/primusdb/config.toml`)
3. Searches running processes for `primusdb` binaries (where supported)

Each candidate is verified by sending HTTP GET requests to `/health`, `/status`, and `/protocol/health` endpoints. Any endpoint returning a 2xx response identifies the instance.

**Example output:**
```
Endpoint                   Node ID              Version    Status
---------------------------------------------------------------------------
http://127.0.0.1:8080      local_node           1.3.1-alpha healthy
http://127.0.0.1:8081      secondary_node        1.3.1-alpha healthy
```

**JSON output:**
```bash
primusdb instance list --format json
```
```json
[
  {
    "endpoint": "http://127.0.0.1:8080",
    "node_id": "local_node",
    "version": "1.3.1-alpha",
    "status": "healthy",
    "uptime_seconds": 3600,
    "enabled_engines": ["columnar", "vector", "document", "relational"],
    "cluster_role": null,
    "protocol_status": "active"
  }
]
```

**No instances found:**
```
No PrimusDB instances found.
```

---

## `primusdb instance discover`

Scans a specific host address and port range for PrimusDB instances.

**Usage:**
```bash
primusdb instance discover --host 127.0.0.1 --start-port 8080 --max-ports 10
primusdb instance discover --host 192.168.1.100 --start-port 8080 --max-ports 5 --timeout 10
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--host` | Host address to scan | `127.0.0.1` |
| `--start-port` | First port to probe | `8080` |
| `--max-ports` | Number of ports to scan from start | `5` |
| `--timeout` | Per-probe timeout in seconds | `5` |

**Example output:**
```
Endpoint                   Node ID              Version    Status
---------------------------------------------------------------------------
http://192.168.1.100:8080  node_alpha           1.3.1-alpha healthy
http://192.168.1.100:9090  node_beta            1.3.1-alpha healthy
```

**No instances found:**
```
No PrimusDB instances found on 192.168.1.100:8080-8084.
```

---

## `primusdb instance inspect`

Retrieves detailed information about a specific instance by probing its health and status endpoints.

**Usage:**
```bash
primusdb instance inspect http://localhost:8080
primusdb instance inspect localhost:8080
primusdb instance inspect http://192.168.1.50:8080 --verbose
```

**Arguments:**

| Argument | Description |
|----------|-------------|
| `endpoint` | Instance URL (`http://host:port` or `host:port`) |

**Options:**

| Option | Description |
|--------|-------------|
| `--verbose` | Show full health and status response fields |

**Example output (standard):**
```
Key              Value
---              -----
Endpoint         http://localhost:8080
Health           healthy
status.status    running
status.version   1.3.1-alpha
status.node_id   local_node
```

**Example output (verbose):**
```
Key                        Value
---                        -----
Endpoint                   http://localhost:8080
Health                     healthy
health.status              healthy
health.version             1.3.1-alpha
health.uptime_seconds      3600
health.enabled_engines     ["columnar","vector","document","relational"]
status.status              running
status.version             1.3.1-alpha
status.node_id             local_node
status.uptime_seconds      3600
status.enabled_engines     ["columnar","vector","document","relational"]
```

---

## `primusdb instance connect`

Tests connectivity to a PrimusDB instance by sending a health check request.

**Usage:**
```bash
primusdb instance connect http://localhost:8080
primusdb instance connect 192.168.1.100:8080 --timeout 30
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--timeout` | Connection timeout in seconds | `10` |

**Expected output (success):**
```
Connected to http://localhost:8080
```

**Expected output (failure):**
```
Connection failed: HTTP 503
Connection failed: connection refused
```

---

## `primusdb instance stop`

Sends a stop signal to a running instance via HTTP. Falls back to locating the process by port and sending a kill signal.

**Usage:**
```bash
primusdb instance stop http://localhost:8080
primusdb instance stop localhost:8080
primusdb instance stop http://localhost:8080 --force
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--force` | Skip graceful shutdown, kill immediately | `false` |

Without `--force`, sends a `POST /stop` request. With `--force`, sends `DELETE /stop`. If the HTTP endpoint is unreachable, falls back to `lsof` + `kill` on the port.

**Expected output:**
```
Stop signal sent to http://localhost:8080
```

**Fallback output:**
```
Stopped process on port 8080
```

**If all methods fail:**
```
Could not stop http://localhost:8080: No process found on port 8080. Try: kill $(lsof -ti :8080)
```

---

## `primusdb instance logs`

> **Alpha limitation:** Log retrieval is not fully implemented in v1.3.1-alpha.

**Usage:**
```bash
primusdb instance logs http://localhost:8080
primusdb instance logs http://localhost:8080 --lines 100
primusdb instance logs http://localhost:8080 --follow
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `--lines` | Number of recent log lines | `50` |
| `--follow` | Tail log output continuously | `false` |

**Expected output:**
```
Log retrieval not implemented in v1.3.1-alpha.
Use: journalctl -u primusdb or check server output.
```

**Workarounds for viewing logs:**

```bash
# If running with systemd
journalctl -u primusdb -n 100 -f

# If running in a terminal/session
# Check the terminal output where the server was started

# If using dev-start.sh
./scripts/dev-start.sh --log-file /tmp/primusdb.log
tail -f /tmp/primusdb.log
```

---

## Discovery via PID Files and Config Files

`primusdb instance list` scans the following sources to find running instances:

### Default Ports

```
8080, 8081, 8082, 8083, 9090, 9091, 9092, 9093
```

Each port on `127.0.0.1` is probed with HTTP GET requests to:
- `/health`
- `/status`
- `/protocol/health`

### Config File Paths

The following config files are checked for `network.port` and `network.bind_address` entries:

1. `./primusdb.toml` (current directory)
2. `./config.toml` (current directory)
3. `~/.config/primusdb/config.toml`
4. `/etc/primusdb/config.toml`

### Process List

On Linux, running PrimusDB processes are detected by scanning `/proc` for binaries named `primusdb` or `primusdb-server` and extracting their listening ports.

---

## Alpha Limitations

- **`instance logs`**: Not yet implemented. Use system-level tools (`journalctl`, `tail`) to view logs.
- **`instance list` via config**: Config file scanning checks only the default paths listed above.
- **`instance stop` remote**: If the HTTP endpoint is down, fallback relies on `lsof`, which may not be available on all systems.
- **Cross-host discovery**: `instance discover` scans only the specified host — it does not perform network-wide broadcast discovery (use `primusdb discover` for that).
