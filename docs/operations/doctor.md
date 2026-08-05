# PrimusDB Doctor

The `primusdb doctor` command runs a comprehensive set of diagnostic checks on the system to verify that everything is properly configured and healthy.

## Usage

```bash
# Quick health check
primusdb doctor

# Comprehensive diagnostics (checks disk space, metrics endpoint)
primusdb doctor --aggressive

# Save diagnostic report to a file
primusdb doctor --report /tmp/diagnostic-report.txt

# Aggressive with report
primusdb doctor --aggressive --report /tmp/diagnostic-report.txt
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--aggressive` | Run aggressive diagnostics (disk space, metrics endpoint) | `false` |
| `--report <PATH>` | Write diagnostic report to this file | — |

### Output Formats

The doctor command uses the global `--format` flag to control output:

```bash
# Table output (default)
primusdb doctor
primusdb doctor --format table

# JSON output
primusdb doctor --format json

# Plain text output
primusdb doctor --format plain
```

## What It Checks

### 1. Rust Version

Checks the installed Rust toolchain version by running `rustc --version` or inspecting the `RUSTUP_TOOLCHAIN` environment variable.

```
● Rust toolchain    rustc 1.75.0 (9dcdc4a5c 2023-12-17)
```

### 2. PrimusDB Version

Displays the running binary's version from `CARGO_PKG_VERSION`.

```
● PrimusDB version  1.3.2-alpha
```

### 3. Build Profile

Shows whether the binary was compiled in debug or release mode.

```
● Build profile     debug
```

### 4. Configuration File Existence

Checks common config file paths in order:

1. `./primusdb.toml`
2. `./config.toml`
3. `./config/primusdb.toml`
4. `~/.config/primusdb/config.toml`

```
✓ Config file       primusdb.toml
```

If no config file is found, a warning is shown:

```
~ Config file       Not found in default locations (primusdb.toml, config.toml)
```

### 5. Data Directory

Checks whether the data directory exists and is readable/writable.

```
✓ Data directory    ./data
```

If the directory does not exist:

```
● Data directory    Not created yet (will be created on first start)
```

### 6. Port Availability

Checks whether port 8080 (the default) is available for binding.

```
✓ Port 8080         Available
```

If something is already listening on the port:

```
~ Port 8080         In use or unavailable
```

### 7. Server Health (HTTP /health)

Attempts to connect to `http://127.0.0.1:8080/health` and checks for a successful response. Only tested if the server is reachable.

```
✓ Server health     Running and responding
```

If the server is not running:

```
~ Server health     Not reachable at http://127.0.0.1:8080
```

### 8. Status Endpoint

Attempts to connect to `http://127.0.0.1:8080/status` and checks for a successful response.

```
✓ Status endpoint   Responds correctly
```

### 9. Metrics Endpoint (with `--aggressive`)

When `--aggressive` is enabled, the doctor also checks the `/metrics` endpoint:

```
✓ Metrics endpoint  Available
```

### 10. Docker Availability (Optional)

Checks whether Docker is installed and available on the PATH.

```
✓ Docker            Available
```

If Docker is not detected:

```
● Docker            Not detected
```

### 11. Disk Space (with `--aggressive`)

When `--aggressive` is enabled, the doctor runs `df -h` on the current directory to report free disk space:

```
✓ Disk space        45G free (current dir)
```

### 12. OS Info

Reports the operating system and CPU architecture.

```
● OS                linux
● Architecture      x86_64
```

## Output Format

### Table (Default)

```
  Check                Result
  ────────────────────────────────────────────────────────
● Rust toolchain       rustc 1.75.0 (9dcdc4a5c 2023-12-17)
● PrimusDB version     1.3.2-alpha
● Build profile        debug
● License              GPL-3.0
✓ Config file          primusdb.toml
✓ Data directory       ./data
✓ Port 8080            Available
✓ Docker               Available
● OS                   linux
● Architecture         x86_64
```

### JSON

```json
{
  "checks": [
    {"check": "Rust toolchain", "status": "INFO", "detail": "rustc 1.75.0"},
    {"check": "PrimusDB version", "status": "INFO", "detail": "1.3.2-alpha"},
    {"check": "Config file", "status": "PASS", "detail": "primusdb.toml"},
    {"check": "Port 8080", "status": "PASS", "detail": "Available"}
  ]
}
```

### Status Icons

| Icon | Status | Meaning |
|------|--------|---------|
| ✓ | Pass | Check succeeded |
| ~ | Warn | Issue detected, may need attention |
| ✗ | Fail | Check failed, requires action |
| ● | Info | Informational, not a pass/fail |

## Report File with `--report PATH`

When `--report` is specified, the doctor writes a plain-text summary to the given file in addition to the terminal output:

```bash
primusdb doctor --report /tmp/primusdb-diag.txt
```

Example report content:

```
Rust toolchain: INFO — rustc 1.75.0 (9dcdc4a5c 2023-12-17)
PrimusDB version: INFO — 1.3.2-alpha
Build profile: INFO — debug
License: INFO — GPL-3.0
Config file: PASS — primusdb.toml
Data directory: PASS — ./data
Port 8080: PASS — Available
Server health: PASS — Running and responding
Status endpoint: PASS — Responds correctly
Docker: PASS — Available
OS: INFO — linux
Architecture: INFO — x86_64
```

## Interpreting Results

### All Checks Pass

Your PrimusDB installation is correctly configured and the server is running.

### Config File Warning

No configuration file was found. PrimusDB will use built-in defaults. Create one with:

```bash
primusdb config init
```

### Port Unavailable

Another process is listening on port 8080. Either stop the other process or use `--bind` to specify a different port:

```bash
# Check what is using the port
netstat -tlnp | grep 8080

# Start on a different port
primusdb server start --bind 0.0.0.0:9090
```

### Server Not Reachable

The PrimusDB server is not running or is not accessible at `http://127.0.0.1:8080`. Start the server:

```bash
primusdb server start
```

### Data Directory Missing

The data directory does not exist. It will be created automatically when the server starts, but you can create it manually:

```bash
mkdir -p ./data
```

## Example Output

### Minimal Output (no server running)

```
  Check                Result
  ────────────────────────────────────────────────────────
● Rust toolchain       rustc 1.75.0 (9dcdc4a5c 2023-12-17)
● PrimusDB version     1.3.2-alpha
● Build profile        debug
● License              GPL-3.0
~ Config file          Not found in default locations
● Data directory       Not created yet
✓ Port 8080            Available
~ Server health        Not reachable at http://127.0.0.1:8080
~ Status endpoint      Not reachable
● Docker               Not detected
● OS                   linux
● Architecture         x86_64
```

### Full Output (server running, aggressive mode)

```
  Check                Result
  ────────────────────────────────────────────────────────
● Rust toolchain       rustc 1.75.0 (9dcdc4a5c 2023-12-17)
● PrimusDB version     1.3.2-alpha
● Build profile        debug
● License              GPL-3.0
✓ Config file          primusdb.toml
✓ Data directory       ./data
✓ Port 8080            Available
✓ Server health        Running and responding
✓ Status endpoint      Responds correctly
✓ Metrics endpoint     Available
✓ Docker               Available
✓ Disk space           45G free (current dir)
● OS                   linux
● Architecture         x86_64
```
