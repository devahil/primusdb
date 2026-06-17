# TUI Local Discovery

The PrimusDB TUI automatically discovers running PrimusDB instances on your local machine each time it starts, and on demand via the `discover` command.

---

## How Auto-Discovery Works

On launch, the TUI probes localhost ports for running PrimusDB instances:

```
Startup ──► Scan default ports ──► Health probe ──► Display results
                                       │
                                    ┌──┴──┐
                                    │/health│
                                    │/status│
                                    │/protocol/health│
                                    └──────┘
```

### Port Scanning

The TUI scans the following default ports on `127.0.0.1`:

```
8080, 8081, 8082, 8083, 9090, 9091, 9092, 9093
```

### Health Probing

Each port is probed by sending HTTP GET requests to three endpoints, in order:

1. `http://127.0.0.1:<port>/health`
2. `http://127.0.0.1:<port>/status`
3. `http://127.0.0.1:<port>/protocol/health`

The first endpoint to return a 2xx response identifies the instance. A 500ms timeout is used per probe.

### Display

When instances are found, they are displayed in a table:

```
Discovered instances:

Endpoint                   Node ID              Version    Status
---------------------------------------------------------------------------
http://127.0.0.1:8080      local_node           1.3.1-alpha healthy
http://127.0.0.1:8081      secondary_node        1.3.1-alpha healthy
```

If no instances are found:

```
No PrimusDB instances found on localhost.
Start one with: primusdb server start
```

---

## Connecting to Discovered Instances

### Via Auto-Connect Flag

Launch the TUI and immediately connect to a specific server:

```bash
primusdb tui --server http://localhost:8080
```

This bypasses the discovery prompt and shows:

```
Connected to: http://localhost:8080

Discovered instances:
...
```

### Via Manual Connect

Within the TUI, use the `connect` command:

```bash
primusdb> connect http://localhost:8080
Connected to http://localhost:8080
```

The URL can be a full URL (`http://host:port`) or just `host:port` (auto-prepends `http://`):

```bash
primusdb> connect 192.168.1.100:8080
```

The TUI verifies the connection by checking `/health` with a 5-second timeout.

### Via Re-Discovery

Re-scan for instances without restarting the TUI:

```bash
primusdb> discover
Scanning for local PrimusDB instances...

Discovered instances:
...
```

---

## Manual Connection

To connect to an instance that is not on the default scanned ports or is on a remote machine, use the `connect` command with the full URL:

```bash
primusdb> connect http://10.0.0.50:9090
```

Or specify the server at launch:

```bash
primusdb tui --server http://10.0.0.50:9090
```

---

## Discovery Data

Each discovered instance provides the following information (from the `/health` response):

| Field | Description |
|-------|-------------|
| Endpoint | Full URL (e.g., `http://127.0.0.1:8080`) |
| Node ID | Instance identifier from config (`node_id`) |
| Version | PrimusDB version (e.g., `1.3.1-alpha`) |
| Status | Health status (`healthy`, `degraded`, `unknown`) |
| Engines | List of enabled storage engines (shown in TUI status) |

---

## Alpha Limitations

- **Localhost only** — auto-discovery on startup only scans `127.0.0.1`. Remote instances must be connected to manually.
- **Fixed port list** — only ports 8080–8083 and 9090–9093 are scanned. Custom ports are not auto-discovered.
- **No UDP broadcast** — network-level service discovery is not implemented; use `primusdb discover` for broadcast-based discovery.
- **No config-file scanning** — unlike `primusdb instance list`, the TUI does not scan config files for custom ports.
- **No process scanning** — the TUI does not inspect `/proc` for running PrimusDB processes.
