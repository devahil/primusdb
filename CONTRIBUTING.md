# Contributing to PrimusDB

Thank you for your interest in contributing to PrimusDB! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, inclusive, and constructive. We welcome contributions from everyone.

## Quick Start

```bash
# Clone the repository
git clone https://github.com/devahil/primusdb.git
cd primusdb

# Build
cargo build

# Run tests
cargo test

# Check linting
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```

## How to Contribute

### Reporting Issues

- **Bug reports**: Include PrimusDB version, OS, steps to reproduce, and expected vs actual behavior.
- **Feature requests**: Describe the use case and proposed solution.
- **Documentation issues**: Point out inaccuracies or missing information.

### Pull Requests

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/amazing-feature`).
3. Make your changes.
4. Run the full test suite:
   ```bash
   cargo test --workspace
   cargo clippy --workspace -- -D warnings
   cargo fmt --all --check
   ```
5. Commit with a clear message (`git commit -m 'Add amazing feature'`).
6. Push to your fork (`git push origin feature/amazing-feature`).
7. Open a Pull Request.

### PR Checklist

- [ ] Code follows Rust formatting (`cargo fmt`)
- [ ] Clippy passes with no warnings (`cargo clippy -- -D warnings`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] New code includes tests
- [ ] New CLI commands include help text
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated
- [ ] No secrets or credentials committed

## Development Setup

### Prerequisites

- Rust toolchain (1.70+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Git

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# All workspace members
cargo build --workspace
```

### Testing

```bash
# All tests
cargo test --workspace

# Documentation tests
cargo test --doc

# Specific test
cargo test -- test_name

# With output
cargo test -- --nocapture
```

### Linting

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all --check

# Run clippy
cargo clippy --workspace -- -D warnings
```

### Running Locally

```bash
# Generate default config
cargo run -- config init

# Start server
cargo run -- server start

# In another terminal, run CLI commands
cargo run -- version
cargo run -- doctor
cargo run -- query "SELECT 1"
```

## Project Structure

```
src/
├── main.rs          # Entry point -> cli::run()
├── lib.rs           # Library exports
├── cli/             # CLI implementation
│   ├── command.rs   # Clap command definitions
│   ├── mod.rs       # Dispatch logic
│   ├── output.rs    # Output formatting (table/json/csv/yaml/plain)
│   ├── discovery.rs # Node discovery
│   └── cmd/         # Command handlers
│       ├── server.rs
│       ├── query.rs
│       ├── db.rs
│       ├── config.rs
│       ├── doctor.rs
│       └── ...
├── bin/
│   ├── server.rs    # Legacy primusdb-server binary
│   └── cli.rs       # Legacy primusdb-cli binary
├── api/             # REST API (Axum)
├── protocol/        # P2P protocol layer
├── consensus/       # Blockchain validation
├── ai/              # AI/ML inference
├── auth/            # Authentication & RBAC
├── cluster/         # Cluster management
├── query/           # SQL parser
├── graph.rs         # Graph engine
├── cdc.rs           # Change Data Capture
├── fulltext.rs      # Full-text search
├── metrics.rs       # Prometheus metrics
└── storage/         # Storage engines

crates/
├── primusdb-core/   # Core types
├── primusdb-storage/
├── primusdb-crypto/
├── primusdb-consensus/
├── primusdb-transaction/
├── primusdb-ai/
├── primusdb-cluster/
├── primusdb-drivers/
├── primusdb-api/
└── primusdb-error/

docs/               # Documentation
├── getting-started/
├── user-guide/
├── architecture/
├── operations/
├── features/
├── contributors/
├── cli/
├── reference/
├── security/
└── README.md

scripts/            # Development scripts
└── check-all.sh
```

## Adding a CLI Command

See the full guide at [docs/contributors/adding-cli-commands.md](docs/contributors/adding-cli-commands.md).

Briefly:
1. Define the command struct in `src/cli/command.rs`
2. Create a handler module in `src/cli/cmd/`
3. Wire the dispatch in `src/cli/mod.rs`
4. Register the module in `src/cli/cmd/mod.rs`

## Documentation

- Documentation lives under `docs/`.
- Use clear, concise language.
- Include command examples.
- Mark alpha limitations honestly.
- Run `scripts/check-docs.sh` to validate.

## Release Process

1. Update version in `Cargo.toml` and sub-crates.
2. Update `CHANGELOG.md`.
3. Run full test suite.
4. Build release.
5. Tag and push.
6. Package binaries.

See [docs/contributors/release-process.md](docs/contributors/release-process.md) for details.

## License

By contributing, you agree that your contributions will be licensed under the GPL-3.0 license.
