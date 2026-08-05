# CLI Migration Guide

This guide helps users transition from the legacy `primusdb-server` and `primusdb-cli` binaries to the new unified `primusdb` CLI.

> **Note:** This is about migrating CLI **binaries**. For migrating **databases** from external systems (PostgreSQL, MySQL, MongoDB, CouchDB), see the [Database Migration Guide](../migration/README.md).

## Overview

Starting with **v1.3.2-alpha**, PrimusDB ships a single unified binary `primusdb` that replaces both `primusdb-server` and `primusdb-cli`. The legacy binaries **have been removed**; use the unified binary for everything.

## Migration Timeline

| Version | Status |
|---------|--------|
| v1.3.1-alpha | `primusdb-server` and `primusdb-cli` still shipped alongside `primusdb`. |
| v1.3.2-alpha | Unified CLI is the only binary. Legacy binaries removed (`src/bin/` deleted); server and CLI components migrated into `primusdb server start` and `primusdb <command>`. |

## `primusdb-server` → `primusdb server`

The server lifecycle commands move under the `primusdb server` subcommand.

### Starting the Server

| Old | New |
|-----|-----|
| `primusdb-server` | `primusdb server start` |
| `primusdb-server --host 0.0.0.0 --port 8080` | `primusdb server start --bind 0.0.0.0:8080` |
| `primusdb-server -p 9090` | `primusdb server start --bind 127.0.0.1:9090` |
| `primusdb-server --config prod.toml` | `primusdb server start --config prod.toml` |
| `primusdb-server --data-dir /var/lib/primusdb` | `primusdb server start --data-dir /var/lib/primusdb` |
| `primusdb-server --log-level debug` | `primusdb server start --log-level debug` |
| `primusdb-server --cluster` | `primusdb server start --cluster-id my-cluster --federation-discovery coordinator:8080` |

**Note:** The old `--host` and `--port` separate flags are consolidated into a single `--bind` flag in the new CLI (format: `host:port`).

### Server Lifecycle

| Old | New |
|-----|-----|
| `primusdb-server` (background) | `primusdb server start --daemon` |
| N/A (manual kill) | `primusdb server stop` |
| N/A | `primusdb server restart` |
| N/A | `primusdb server status` |
| N/A | `primusdb server health` |

### Configuration

| Old | New |
|-----|-----|
| Edit `config.toml` manually | `primusdb server config --set key=value` |
| Read `config.toml` manually | `primusdb server config --list` |
| N/A | `primusdb server config --get storage.data_dir` |

### Federation Flags

| Old | New |
|-----|-----|
| `primusdb-server --cluster --federation-id default` | `primusdb server start --cluster-id my-cluster --federation-id default` |
| `primusdb-server --cluster-id mycluster` | `primusdb server start --cluster-id mycluster` |
| `primusdb-server --region us-east` | `primusdb server start --region us-east` |
| `primusdb-server --federation-discovery peer1:8080` | `primusdb server start --federation-discovery peer1:8080` |

## `primusdb-cli` → `primusdb`

The old `primusdb-cli` subcommands are available directly under the unified CLI.

### CRUD Operations

The old CRUD subcommands are replaced by SQL queries:

| Old | New |
|-----|-----|
| `primusdb-cli crud create --storage-type document --table users --data '{"name":"Alice"}'` | `primusdb query "INSERT INTO users (name) VALUES ('Alice')"` |
| `primusdb-cli crud read --storage-type document --table users --limit 10` | `primusdb query "SELECT * FROM users LIMIT 10"` |
| `primusdb-cli crud update --storage-type document --table users --conditions '{"id":1}' --data '{"name":"Bob"}'` | `primusdb query "UPDATE users SET name='Bob' WHERE id=1"` |
| `primusdb-cli crud delete --storage-type document --table users --conditions '{"id":1}'` | `primusdb query "DELETE FROM users WHERE id=1"` |

### Table Management

| Old | New |
|-----|-----|
| `primusdb-cli table create --storage-type document --table users --schema '...'` | `primusdb db create users --engine document` or `primusdb query "CREATE TABLE users (...)"` |
| `primusdb-cli table drop --storage-type document --table users` | `primusdb db drop users` or `primusdb query "DROP TABLE users"` |
| `primusdb-cli table truncate --storage-type document --table users` | `primusdb query "TRUNCATE TABLE users"` |
| `primusdb-cli table info --storage-type document --table users` | `primusdb db describe users --schema` |

### Server Status

| Old | New |
|-----|-----|
| `primusdb-cli status` | `primusdb server status` |
| `primusdb-cli --mode client status` | `primusdb server status` |
| `primusdb-cli status` (embedded) | `primusdb server status` |

### Backup and Restore

| Old | New |
|-----|-----|
| `primusdb-cli backup /path/to/backup` | `primusdb backup create --destination /path/to/backup` |
| `primusdb-cli restore /path/to/backup` | `primusdb backup restore /path/to/backup` or `primusdb restore /path/to/backup` |

### Initialization

| Old | New |
|-----|-----|
| `primusdb-cli init --data-dir ./data` | `primusdb server start --data-dir ./data` (auto-initializes) |

### Advanced Operations

| Old | New |
|-----|-----|
| `primusdb-cli advanced analyze --storage-type columnar --table sales` | `primusdb ai analyze sales` |
| `primusdb-cli advanced predict --storage-type columnar --table sales --data '...'` | `primusdb ai predict model_name '...'` |
| `primusdb-cli advanced vector-search --table embeddings --query-vector "0.1,0.2"` | `primusdb vector search index_name '[0.1,0.2]'` |
| `primusdb-cli advanced cluster --storage-type document --table customers` | `primusdb ai train clustering_model customers --model-type clustering` |

### Namespace Operations

| Old | New |
|-----|-----|
| `primusdb-cli namespace list` | `primusdb namespace list` |
| `primusdb-cli namespace create root.tenant` | `primusdb namespace create tenant` |
| `primusdb-cli namespace delete root.tenant` | `primusdb namespace drop tenant` |
| `primusdb-cli namespace info root.tenant` | `primusdb namespace describe tenant` |
| `primusdb-cli namespace children root` | `primusdb namespace list --parent root` |
| `primusdb-cli namespace policy root.tenant` | `primusdb namespace policy tenant --list` |
| `primusdb-cli namespace resources root.tenant` | `primusdb namespace describe tenant --resources` |

### Client Mode

| Old | New |
|-----|-----|
| `primusdb-cli --mode client --server http://localhost:8080 status` | `primusdb --server-url http://localhost:8080 server status` |
| `primusdb-cli --mode client crud create ...` | `primusdb --server-url http://localhost:8080 query "..."` |

## Breaking Changes

1. **Legacy binaries removed:** `primusdb-server` and `primusdb-cli` no longer exist. Use `primusdb server start` / `primusdb <command>`.
2. **`--host` and `--port` consolidated:** The old server used `--host` and `--port` as separate flags. The new CLI uses `--bind host:port`.
3. **No `--mode` flag:** The unified CLI always operates in client mode for server operations. Embedded mode is available for direct database access.
4. **CRUD via SQL:** Direct CRUD subcommands are replaced by SQL queries. Use `primusdb query` with standard SQL syntax.
5. **Default port:** The old CLI server defaulted to `8080`. The new CLI defaults to `127.0.0.1:8080`.

## Migration Script Example

```bash
#!/bin/bash
# Migrate from old primusdb-cli commands to new unified CLI

# Old: primusdb-cli --mode client status
# New:
primusdb server status

# Old: primusdb-cli crud create --storage-type document --table users --data '{"name":"Alice"}'
# New:
primusdb query "INSERT INTO users (name) VALUES ('Alice')"

# Old: primusdb-cli advanced predict --storage-type columnar --table sales --data '{"quarter":"Q1"}'
# New:
primusdb ai predict sales-model '{"quarter":"Q1"}'

# Old: primusdb-server --host 0.0.0.0 --port 8080 --cluster
# New:
primusdb server start --bind 0.0.0.0:8080 --cluster-id my-cluster --federation-discovery coordinator:8080
```

## Verification

After migrating, verify the new CLI works:

```bash
# Check version
primusdb version --verbose

# List commands
primusdb --help

# Test server lifecycle
primusdb server start --daemon
primusdb server status
primusdb server stop
```
