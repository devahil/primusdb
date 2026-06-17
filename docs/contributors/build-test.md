# Building and Testing

This guide covers how to build, test, lint, and benchmark PrimusDB.

## Prerequisites

- **Rust**: Edition 2021, minimum version 1.70. Install via [rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **System dependencies**: `build-essential`, `cmake`, `pkg-config`, `libssl-dev`
  ```bash
  # Debian/Ubuntu
  sudo apt install build-essential cmake pkg-config libssl-dev

  # Fedora/RHEL
  sudo dnf install gcc cmake pkgconfig openssl-devel

  # Arch
  sudo pacman -S base-devel cmake pkgconf openssl
  ```

## Building

### Debug Build

Build the entire workspace in debug mode (fast compilation, unoptimized):

```bash
cargo build --workspace
```

This compiles all workspace members: `primusdb`, `primusdb-core`,
`primusdb-storage`, `primusdb-crypto`, `primusdb-consensus`,
`primusdb-transaction`, `primusdb-ai`, `primusdb-cluster`,
`primusdb-drivers`, `primusdb-api`, `primusdb-error`, and
`drivers/rust`.

### Release Build

Build with optimizations (LTO, single codegen unit, panic=abort, stripped):

```bash
cargo build --release --workspace
```

Release binaries are placed in `target/release/`:
- `target/release/primusdb` — Unified CLI (primary)
- `target/release/primusdb-server` — Legacy server binary
- `target/release/primusdb-cli` — Legacy CLI binary

### Build Individual Packages

```bash
# Build only the root crate
cargo build -p primusdb

# Build only the storage crate
cargo build -p primusdb-storage

# Build only the native Rust driver
cargo build -p primusdb-driver
```

### Build Documentation

```bash
# Build crate documentation
cargo doc --workspace --no-deps

# Open in browser
cargo doc --workspace --no-deps --open
```

## Testing

### Run All Tests

```bash
# Run all tests across the workspace (unit + integration)
cargo test --workspace
```

### Run Unit Tests Only

```bash
# Unit tests (in-file #[cfg(test)] modules), excluding integration tests
cargo test --lib
```

### Run Integration Tests Only

```bash
# Tests in tests/ directory
cargo test --test integration_tests
cargo test --test e2e_rest_api
cargo test --test e2e_server
cargo test --test e2e_backup_restore
```

### Run Doc Tests

```bash
# Run tests embedded in rustdoc examples (```ignore blocks are skipped)
cargo test --doc
```

### Run Specific Test

```bash
# By test name (matches substring)
cargo test test_columnar_storage_crud

# Full path
cargo test integration_tests::test_columnar_storage_crud

# By module
cargo test output::tests
```

### Run Tests for a Specific Package

```bash
cargo test -p primusdb
cargo test -p primusdb-storage
```

### Run Tests with Output

```bash
cargo test -- --nocapture
```

### Test Filtering Examples

```bash
# Run all vector-related tests
cargo test vector

# Run all namespace tests
cargo test namespace

# Run all CRUD tests across all engines
cargo test crud
```

## Linting

### Formatting

Check formatting without modifying files:

```bash
cargo fmt --all --check
```

Auto-fix formatting:

```bash
cargo fmt --all
```

### Clippy

Run Clippy with warnings-as-errors (matches CI):

```bash
cargo clippy --workspace -- -D warnings
```

Run Clippy with all lint groups:

```bash
cargo clippy --workspace -- -W clippy::all -W clippy::pedantic -W clippy::nursery
```

### Check (Compile Verification)

Fast compile check without producing binaries:

```bash
cargo check --workspace
```

## Benchmarks

PrimusDB uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for
benchmarking. Benchmarks are defined in the `benches/` directory.

### Run All Benchmarks

```bash
cargo bench
```

### Run Specific Benchmarks

```bash
# Storage read performance
cargo bench --bench storage_read

# Vector similarity search
cargo bench --bench vector_search

# AI/ML model performance
cargo bench --bench ai_ml
```

### Benchmark Output

Benchmark results are written to `target/criterion/` with HTML reports.
Open `target/criterion/report/index.html` in a browser for interactive
charts and comparison views.

### Comparing Changes

Criterion automatically compares against previous runs. To do a controlled
comparison:

```bash
# Run baseline
git checkout main
cargo bench --bench storage_read -- --save-baseline main

# Run your changes
git checkout your-branch
cargo bench --bench storage_read -- --baseline main
```

## Scripts

### `scripts/check-all.sh`

Runs the full quality assurance pipeline. This is the same set of checks
used in CI:

```bash
# Run all checks
./scripts/check-all.sh

# Auto-fix formatting issues
./scripts/check-all.sh --fix

# Skip documentation checks (faster)
./scripts/check-all.sh --skip-docs
```

The pipeline executes in order:
1. `cargo fmt --all --check` (or `--all` with `--fix`)
2. `cargo clippy --workspace -- -D warnings`
3. `cargo build --workspace`
4. `cargo build --release --workspace`
5. `cargo test --workspace`
6. `cargo test --doc`
7. `scripts/check-docs.sh` (if exists, skipped with `--skip-docs`)

Any failure stops the pipeline with a non-zero exit code.

### `scripts/check-docs.sh`

Validates documentation structure, checks for broken links, and ensures
all public API items have documentation:

```bash
./scripts/check-docs.sh
```

### `scripts/build-release.sh`

Builds a release binary and optionally runs the packaging step:

```bash
./scripts/build-release.sh
```

### `scripts/package-linux.sh`

Creates a Linux distribution tarball containing binaries, docs, and configs:

```bash
# Package with auto-detected version
./scripts/package-linux.sh

# Package with explicit version
./scripts/package-linux.sh --version 1.3.1-alpha

# Package to custom output directory
./scripts/package-linux.sh --output ./dist
```

Output: `dist/primusdb-{version}-linux-{arch}.tar.gz`

## Continuous Integration

PrimusDB uses GitHub Actions for CI. The workflow file is at
`.github/workflows/ci.yml` (if present) and typically runs:

| Step | Command | Purpose |
|------|---------|---------|
| Format check | `cargo fmt --all --check` | Enforces consistent code style |
| Clippy | `cargo clippy --workspace -- -D warnings` | Catches common mistakes |
| Build | `cargo build --workspace` | Verifies compilation |
| Release build | `cargo build --release --workspace` | Ensures release build succeeds |
| Tests | `cargo test --workspace` | Runs all unit + integration tests |
| Doc tests | `cargo test --doc` | Validates documentation examples |
| Benchmarks | `cargo bench --workspace` | Performance regression check |

CI runs on every push and pull request to the main branch.

## Troubleshooting

### Build Failures

- **Outdated lockfile**: Run `cargo update` to refresh dependencies
- **SSL errors**: Ensure `libssl-dev` (or equivalent) is installed
- **Out of memory**: Release builds with LTO are memory-intensive. Use
  `CARGO_PROFILE_RELEASE_LTO=false` to disable LTO temporarily
- **Incremental compilation issues**: `cargo clean && cargo build`

### Test Failures

- **Temp directory permissions**: Tests use `tempfile` — ensure `/tmp` is writable
- **Port conflicts**: Integration tests that start servers bind to port 8080 by
  default. Kill conflicting processes or set a custom port
- **Timing-sensitive tests**: Some cluster tests have timeouts. Increase
  `TEST_TIMEOUT` env var if running on a slow machine

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_LOG` | Logging level (debug/info/warn/error) | `info` |
| `RUST_BACKTRACE` | Full backtrace on panics | `0` |
| `CARGO_PROFILE_RELEASE_LTO` | Enable/disable LTO | `true` |
| `TEST_TIMEOUT` | Test timeout in seconds | `120` |
