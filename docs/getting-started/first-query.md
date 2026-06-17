# First Query in 10 Minutes

This guide walks through the "first 10 minutes" with PrimusDB — from cloning the repository to running your first SQL query.

## Prerequisites

- Rust 1.70+ (`rustc --version`)
- Git
- Build tools:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config libssl-dev

# Arch Linux
sudo pacman -S base-devel openssl

# macOS
xcode-select --install
```

## Step-by-Step

### 1. Clone the Repository

```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
```

**Expected output:**
```
Cloning into 'primusdb'...
remote: Enumerating objects: ...
Receiving objects: 100% (...)
Resolving deltas: 100% (...)
```

### 2. Build the Release Binary

```bash
cargo build --release
```

This compiles the unified `primusdb` CLI binary along with its server, client, and all storage engine crates. The first build downloads and compiles dependencies (~200 crates). Subsequent builds are incremental.

**Expected output (last lines):**
```
   Compiling primusdb v1.3.1-alpha (/home/user/primusdb)
    Finished `release` profile [optimized + LTO] target(s) in 5m 12s
```

> **Tip:** The binary is at `./target/release/primusdb`. Add it to your PATH:
> ```bash
> export PATH="$PWD/target/release:$PATH"
> ```

### 3. Initialize a Configuration

```bash
./target/release/primusdb config init --profile local
```

This writes a default `primusdb.toml` to the current directory with settings suitable for local development (bind `127.0.0.1:8080`, LZ4 compression, no auth required).

**Expected output:**
```
Config written to primusdb.toml (profile: local)
```

### 4. Validate the Configuration

```bash
./target/release/primusdb config validate
```

Checks that the TOML is well-formed and contains the required `[storage]` and `[network]` sections.

**Expected output:**
```
Config is valid: primusdb.toml
```

### 5. Start the Server

```bash
./target/release/primusdb server start
```

Starts the PrimusDB server daemon on `127.0.0.1:8080` in the foreground. You will see log output.

**Expected output:**
```
Starting PrimusDB server on 127.0.0.1:8080...
```

> **Note:** Press `Ctrl+C` to stop. For background operation, use `--daemon` or a terminal multiplexer like `tmux`/`screen`.

### 6. Check Server Health

Open a **second terminal** in the same directory:

```bash
./target/release/primusdb health
```

Probes the `/health` endpoint and returns the server status.

**Expected output:**
```json
{
  "status": "healthy",
  "version": "1.3.1-alpha",
  "uptime_seconds": 12
}
```

### 7. Show Server Status

```bash
./target/release/primusdb status
```

**Expected output:**
```json
{
  "status": "running",
  "version": "1.3.1-alpha",
  "node_id": "local_node",
  "uptime_seconds": 30
}
```

### 8. Run Your First Query

```bash
./target/release/primusdb query "SELECT 1"
```

Executes a trivial SQL expression against the server. This validates the full query pipeline (parser → planner → executor → formatter).

**Expected output (table format, default):**
```
 1
---
 1
```

**JSON format:**
```bash
./target/release/primusdb query "SELECT 1" --format json
```

```json
[{"1": 1}]
```

### 9. Stop the Server

```bash
./target/release/primusdb server stop
```

Sends a graceful shutdown signal (SIGTERM). The server drains in-flight requests, flushes writes, and exits.

**Expected output:**
```
Stopping PrimusDB server...
```

### 10. Run Diagnostics

```bash
./target/release/primusdb doctor
```

Runs a comprehensive system check — Rust version, binary info, config file, data directory, port availability, and server health.

**Expected output:**
```
  Check                Result
  ────────────────────────────────────────────────────────
● Rust toolchain       rustc 1.75.0 (9dcdc4a5c 2023-12-17)
● PrimusDB version     1.3.1-alpha
● Build profile        release
● License              GPL-3.0
✓ Config file          primusdb.toml
✓ Data directory       ./data
✓ Port 8080            Available
● Docker               Not detected
● OS                   linux
● Architecture         x86_64
```

## Next Steps

| Topic | Guide |
|-------|-------|
| Configuration | [Configuration Reference](../getting-started/configuration.md) |
| CLI Commands | [CLI Usage Guide](../usage/cli.md) |
| Querying | [Query Execution Guide](../usage/querying.md) |
| Namespaces | [Namespace Usage Guide](../usage/namespaces.md) |
| Language Drivers | [Driver Usage Guide](../usage/drivers.md) |
| TUI | [TUI Guide](../tui/README.md) |
