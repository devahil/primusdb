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
| [Architecture](architecture/overview.md) | System design, federation layer, and technical decisions |
| [Reference](reference/api.md) | REST API, CLI commands, and TUI guide |
| [Operations](operations/deployment.md) | Deployment, troubleshooting, and production operations |
| [Security](security/overview.md) | Security policies and best practices |
| [Changelog](../CHANGELOG.md) | Release notes and version history |

## CLI Quick Reference

```bash
primusdb --help                  # Show all commands
primusdb server start            # Start a server
primusdb tui                     # Launch interactive TUI
primusdb query "SELECT ..."      # Execute SQL query
primusdb discover                # Find local instances
primusdb completion bash         # Generate shell completions
```

v1.3.1-alpha — [Repository](https://github.com/devahil/primusdb.git)
