# PrimusDB Documentation

Welcome to the PrimusDB documentation. PrimusDB is a high-performance, hybrid database engine written in Rust that combines multiple storage paradigms (columnar, vector, document, and relational) into a unified system.

## Quick Links

- **[Getting Started](getting-started/install.md)** — Build and install PrimusDB
- **[Quick Start](getting-started/quickstart.md)** — First steps
- **[CLI Guide](cli/README.md)** — Unified command-line interface
- **[API Reference](reference/api.md)** — REST API documentation
- **[Changelog](../CHANGELOG.md)** — Version history

## Documentation Structure

| Section | Description |
|---------|-------------|
| [Getting Started](getting-started/install.md) | Installation, build guide, and quick start tutorial |
| [User Guide](user-guide/operations.md) | End-user operations, administration, and examples |
| [Time Series](timeseries/overview.md) | Time-series engine: querying, aggregation, retention, rollups, API |
| [Architecture](architecture/overview.md) | System design, federation layer, and technical decisions |
| [Reference](reference/api.md) | REST API and CLI commands |
| [Operations](operations/deployment.md) | Deployment, troubleshooting, and production operations |
| [Security](security/overview.md) | Security policies and best practices |
| [CLI Guide](cli/README.md) | Full CLI command reference and REPL |
| [Changelog](../CHANGELOG.md) | Release notes and version history |

> **Deprecated**: The TUI was removed from the build in v1.3.2-alpha. The `docs/tui/` pages are retained for historical reference; use the CLI/REPL or REST API instead.

## CLI Quick Reference

```bash
primusdb --help                  # Show all commands
primusdb server start            # Start a server
primusdb query "SELECT ..."      # Execute SQL query
primusdb shell                   # Launch the interactive REPL
primusdb connect                 # Connect to a server and open the REPL
primusdb discover                # Find local instances
primusdb completion bash         # Generate shell completions
```

v1.3.2-alpha — [Repository](https://github.com/devahil/primusdb.git)
