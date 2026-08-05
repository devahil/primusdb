# Testing Strategy

PrimusDB uses a multi-level testing strategy to ensure correctness,
performance, and reliability across all subsystems.

## Test Levels

```
Level           Location                Scope               Speed
──────────────────────────────────────────────────────────────────
Unit tests      In-file #[cfg(test)]    Single function     Fast
Integration     tests/ directory        Multiple modules    Medium
Doc tests       /// ``` blocks          API examples        Medium
CLI tests       tests/ via assert_cmd   End-to-end CLI      Slow
Benchmarks      benches/                Performance         N/A
E2E tests       tests/e2e_*             Full system         Slow
```

## 1. Unit Tests

Unit tests are placed inside each source file in a `#[cfg(test)]` module.
They test individual functions and data structures in isolation.

**Location**: Inline in every `.rs` file under `src/`.

**Pattern**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_some_functionality() {
        let result = my_function(input);
        assert_eq!(result, expected);
    }
}
```

**Running**:

```bash
# All unit tests in the workspace
cargo test --lib

# Unit tests for a specific module
cargo test storage::tests
```

**Examples in the codebase**:

- `src/cli/output.rs` — Tests output formatting for all formats
  (Table, Json, Csv, Plain) and edge cases (empty tables, CSV escaping)
- `src/storage/mod.rs` — Tests for schema validation, constraint checking
- `src/error.rs` — Tests for error type conversion and display

## 2. Integration Tests

Integration tests verify that multiple modules work together correctly.
They use the public API and exercise real storage engines with temporary
directories.

**Location**: `tests/integration_tests.rs` (plus `tests/e2e_*.rs`).

**Pattern**:

```rust
use primusdb::{PrimusDB, PrimusDBConfig, Query, QueryOperation, QueryResult};
use tempfile::TempDir;

async fn setup_test_db() -> Result<(Arc<PrimusDB>, TempDir)> {
    let temp_dir = TempDir::new()?;
    let config = PrimusDBConfig {
        storage: primusdb::StorageConfig {
            data_dir: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        },
        ..Default::default()
    };
    let db = Arc::new(PrimusDB::new(config)?);
    Ok((db, temp_dir))
}

#[tokio::test]
async fn test_columnar_storage_crud() -> Result<()> {
    let (db, _temp_dir) = setup_test_db().await?;

    // Insert
    let insert_query = Query {
        storage_type: StorageType::Columnar,
        operation: QueryOperation::Create,
        table: "sales".to_string(),
        data: Some(serde_json::json!({"amount": 100.0})),
        ..Default::default()
    };
    let result = db.execute_query(insert_query).await?;
    assert!(matches!(result, QueryResult::Insert(1)));

    // Read
    let read_query = Query {
        storage_type: StorageType::Columnar,
        operation: QueryOperation::Read,
        table: "sales".to_string(),
        ..Default::default()
    };
    let result = db.execute_query(read_query).await?;
    assert!(matches!(result, QueryResult::Select(_)));

    Ok(())
}
```

**Running**:

```bash
# Run all integration tests
cargo test --test integration_tests

# Run a specific integration test
cargo test test_columnar_storage_crud

# Run all e2e tests
cargo test --test e2e_rest_api
cargo test --test e2e_server
cargo test --test e2e_backup_restore
```

**Current test suite** (`tests/integration_tests.rs` covers):

- CRUD operations across all 6 storage engines (columnar, vector,
  document, relational, key-value, time-series)
- Namespace isolation (CRUD within namespaces, DDL operations)
- Sequence operations (create, nextval, currval, setval, drop)
- DDL/ER operations (alter table, views, triggers)
- Error cases (not-found, constraint violations)

## 3. Doc Tests

Documentation examples that appear in rustdoc comments can be marked
as executable tests.

**Pattern**:

````rust
/// Adds two numbers together.
///
/// ```
/// use primusdb::my_module::add;
/// assert_eq!(add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
````

**Important**: In the PrimusDB codebase, most doc examples use
` ```ignore ` instead of ` ``` ` because the full `PrimusDB` struct
requires async context and complex setup. To add a new doc test
that actually compiles:

```rust
/// ```
/// assert_eq!(2 + 2, 4);
/// ```
```

To mark an example as non-compiling (documentation-only):

```rust
/// ```ignore
/// let db = PrimusDB::new(config).await?;
/// ```
```

**Running**:

```bash
cargo test --doc
```

**Current status**: 73 doc tests (all `ignore`-marked, 0 failures).

## 4. CLI Tests

CLI commands are tested through the programmatic API that backs them.
Since each CLI handler in `src/cli/cmd/` is a regular async function
accepting typed arguments, they can be tested directly without spawning
a subprocess.

Alternatively, use `assert_cmd` for subprocess-based CLI testing:

```rust
// Example (not yet in the codebase — pattern for future tests)
use assert_cmd::Command;

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("primusdb").unwrap();
    let assert = cmd.arg("version").assert();
    assert.success().stdout(predicates::str::contains("1.3.2-alpha"));
}
```

## 5. Benchmark Tests

Performance benchmarks use the [Criterion](https://docs.rs/criterion)
framework and are defined in the `benches/` directory.

**Location**: `benches/storage_read.rs`, `benches/vector_search.rs`,
`benches/ai_ml.rs`

**Pattern**:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_read(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Setup ...

    c.bench_function("columnar_read_1k", |b| {
        b.to_async(&rt).iter(|| async {
            // Benchmark body ...
        });
    });
}

criterion_group!(benches, bench_read);
criterion_main!(benches);
```

**Running**:

```bash
# All benchmarks
cargo bench

# Specific benchmark
cargo bench --bench storage_read
cargo bench --bench vector_search
cargo bench --bench ai_ml
```

**Profiling**: Release builds with LTO produce optimized binaries that
give realistic performance numbers:
```bash
cargo bench --bench storage_read -- --profile-time 30
```

## 6. Edge Case and Property Tests

For critical subsystems, consider adding property-based tests
using `proptest` or `quickcheck`.

**Pattern** (not yet in the codebase):

```rust
// In src/storage/mod.rs or tests/
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_encoding_roundtrip(data: Vec<u8>) {
        let encoded = encode(&data);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
```

## Running Specific Test Suites

```bash
# ── Build verification ──
cargo build --workspace                    # Debug build
cargo build --release --workspace          # Release build
cargo clippy --workspace -- -D warnings    # Lint check

# ── All tests ──
./scripts/check-all.sh                     # CI-compatible full check
cargo test --workspace                     # All unit + integration
cargo test --doc                           # Doc tests

# ── Unit tests by area ──
cargo test --lib                           # All unit tests
cargo test output::tests                   # CLI output formatting
cargo test storage                         # Storage engine tests
cargo test crypto                          # Crypto module tests
cargo test cache                           # Cache module tests

# ── Integration tests by file ──
cargo test --test integration_tests        # Core integration suite
cargo test --test e2e_rest_api             # REST API end-to-end
cargo test --test e2e_server               # Server lifecycle
cargo test --test e2e_backup_restore       # Backup/restore

# ── Integration tests by name ──
cargo test test_columnar_storage_crud      # Single test
cargo test namespace                       # All namespace tests
cargo test sequence                        # All sequence tests

# ── With output ──
cargo test -- --nocapture                  # Print stdout from tests
RUST_LOG=debug cargo test -- --nocapture   # With debug logging

# ── Excluding slow tests ──
cargo test -- --skip e2e                   # Skip end-to-end tests
```

## Test Patterns Used in the Project

### Setup/Teardown with TempDir

Integration tests use `tempfile::TempDir` for isolated data directories.
The `TempDir` is automatically cleaned up when dropped:

```rust
let temp_dir = TempDir::new()?;
let config = PrimusDBConfig {
    storage: StorageConfig {
        data_dir: temp_dir.path().to_string_lossy().to_string(),
        ..Default::default()
    },
    ..Default::default()
};
```

### Encryption Disabled in Tests

Tests disable file encryption for speed (`encryption_enabled: false`).
This avoids the overhead of AES-256-GCM during test execution.

### Async Test Runners

All integration tests use `#[tokio::test]` for async execution:

```rust
#[tokio::test]
async fn test_something() -> Result<()> {
    // async test body
}
```

### Helper Function for Common Setup

`setup_test_db()` is used across integration tests to avoid duplication:

```rust
async fn setup_test_db() -> Result<(Arc<PrimusDB>, TempDir)> { ... }
```

### Assertion Patterns

- `matches!(result, QueryResult::Insert(n))` — variant checking
- `assert!(result.contains(...))` — string content verification
- `assert_eq!(result, expected)` — exact value comparison
- `result.unwrap()` / `assert!(result.is_ok())` — success assertion

## Writing New Tests

1. **Unit tests**: Add a `#[cfg(test)] mod tests` block at the bottom
   of the file you're testing. Test the public API of the module.

2. **Integration tests**: Add to `tests/integration_tests.rs` for
   cross-module scenarios. Use the `setup_test_db()` pattern.

3. **Doc tests**: Use ` ```ignore ` for examples that need complex
   setup. Use ` ``` ``` for simple, self-contained examples that
   should compile and run.

4. **Benchmarks**: Add a new file in `benches/` following the Criterion
   pattern. Register it in `Cargo.toml` under `[[bench]]`.

5. **E2E tests**: Add to `tests/e2e_*.rs` for full-system tests that
   start the server and make HTTP requests.
