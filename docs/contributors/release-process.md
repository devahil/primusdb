# Release Process

This document describes the process for creating a PrimusDB release.

## Versioning

PrimusDB follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **MAJOR**: Incompatible API changes
- **MINOR**: Backwards-compatible feature additions
- **PATCH**: Backwards-compatible bug fixes

Pre-release versions use the `-alpha` suffix (e.g., `1.3.1-alpha`).

## Release Checklist

### 1. Version Bump

Update the version in all `Cargo.toml` files across the workspace:

```bash
# Root Cargo.toml (src/)
# The version field at line 3:
# version = "1.3.1-alpha" → version = "1.3.2-alpha"

# Also update in:
# - crates/primusdb-core/Cargo.toml
# - crates/primusdb-storage/Cargo.toml
# - crates/primusdb-crypto/Cargo.toml
# - crates/primusdb-consensus/Cargo.toml
# - crates/primusdb-transaction/Cargo.toml
# - crates/primusdb-ai/Cargo.toml
# - crates/primusdb-cluster/Cargo.toml
# - crates/primusdb-drivers/Cargo.toml
# - crates/primusdb-api/Cargo.toml
# - crates/primusdb-error/Cargo.toml
# - drivers/rust/Cargo.toml
```

Update the version string in legacy binaries:

```rust
// src/bin/server.rs — #[command(version = "1.3.1-alpha")]
// src/bin/cli.rs — version reference
```

Also update the version in `src/lib.rs` doc header:

```rust
// Version: 1.3.1-alpha
```

**Tip**: Use a script or `sed` to ensure consistency:

```bash
# Check current versions across all Cargo.toml files
grep '^version' crates/*/Cargo.toml Cargo.toml drivers/rust/Cargo.toml

# Update (example: 1.3.1 → 1.4.0)
sed -i 's/^version = "1.3.1-alpha"/version = "1.4.0-alpha"/' \
  Cargo.toml \
  crates/*/Cargo.toml \
  drivers/rust/Cargo.toml
```

### 2. Update CHANGELOG.md

Add a new entry at the top of `CHANGELOG.md` following the existing format:

```markdown
## [1.3.2-alpha] - 2026-06-14

### Added
- New feature description

### Changed
- Breaking or notable change description

### Fixed
- Bug fix description

### Security
- Security improvement description

### Removed
- Deprecated feature removal (if any)
```

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Group changes under: **Added**, **Changed**, **Fixed**, **Removed**, **Security**.

### 3. Run Full Test Suite

Run the complete quality assurance pipeline:

```bash
# Full CI check
./scripts/check-all.sh

# This runs:
# 1. cargo fmt --all --check
# 2. cargo clippy --workspace -- -D warnings
# 3. cargo build --workspace
# 4. cargo build --release --workspace
# 5. cargo test --workspace
# 6. cargo test --doc
# 7. scripts/check-docs.sh (if exists)
```

Verify zero failures across all steps. If any checks fail, fix them
before proceeding.

### 4. Build Release Binaries

```bash
# Clean build for reproducibility
cargo clean

# Build release binaries with optimizations
cargo build --release --workspace
```

The release profile (from `Cargo.toml`) enables:
- **LTO**: Link-time optimization for performance
- **codegen-units = 1**: Maximizes per-function optimization
- **panic = "abort"**: Removes panic unwind infrastructure
- **strip = true**: Removes debug symbols (smaller binary)

### 5. Tag the Commit

```bash
# Ensure you're on the correct branch (main)
git checkout main
git pull origin main

# Create an annotated tag
git tag -a v1.3.1-alpha -m "Release v1.3.1-alpha"

# Push the tag to GitHub
git push origin v1.3.1-alpha
```

Tag format: `v{MAJOR}.{MINOR}.{PATCH}{SUFFIX}`
Examples: `v1.3.1-alpha`, `v1.2.0`, `v1.1.0`

### 6. Package Distribution

Create the Linux distribution tarball:

```bash
# Standard packaging
./scripts/package-linux.sh

# With explicit version
./scripts/package-linux.sh --version 1.3.1-alpha

# To a custom output directory
./scripts/package-linux.sh --output ./dist --version 1.3.1-alpha
```

The packaging script:
1. Verifies release binaries exist (builds if missing)
2. Creates a directory: `dist/primusdb-{version}-linux-{arch}/`
3. Copies binaries: `primusdb`, `primusdb-server`, `primusdb-cli`
4. Copies documentation and license files
5. Copies example configurations
6. Creates a compressed tarball: `dist/primusdb-{version}-linux-{arch}.tar.gz`

### 7. Publish to crates.io (Future)

When ready for public release, publish crates to crates.io:

```bash
# Verify each crate builds independently
cd crates/primusdb-core && cargo publish --dry-run
cd crates/primusdb-error && cargo publish --dry-run
# ... repeat for all crates

# Publish in dependency order (leaf crates first):
cargo publish -p primusdb-error
cargo publish -p primusdb-core
cargo publish -p primusdb-storage
cargo publish -p primusdb-crypto
cargo publish -p primusdb-consensus
cargo publish -p primusdb-transaction
cargo publish -p primusdb-ai
cargo publish -p primusdb-cluster
cargo publish -p primusdb-drivers
cargo publish -p primusdb-api
cargo publish -p primusdb

# Publish native driver
cd drivers/rust && cargo publish
```

**Note**: The `primusdb` root crate depends on all sub-crates, so it
must be published last.

### 8. GitHub Release

Create a release on GitHub:

```bash
# Using gh CLI (requires authentication)
gh release create v1.3.1-alpha \
  --title "PrimusDB v1.3.1-alpha" \
  --notes "Release notes here" \
  "dist/primusdb-1.3.1-alpha-linux-x86_64.tar.gz"
```

Or create the release manually:
1. Go to https://github.com/devahil/primusdb/releases
2. Click "Draft a new release"
3. Select the tag (`v1.3.1-alpha`)
4. Write release notes (summarize changes from CHANGELOG.md)
5. Attach the distribution tarball
6. Publish release

## Release Summary Checklist

```
□  1. Version bump in all Cargo.toml files
□  2. CHANGELOG.md updated with new entry
□  3. Full test suite passes (./scripts/check-all.sh)
□  4. Release build succeeds (cargo build --release --workspace)
□  5. Git tag created and pushed (git tag -a vX.Y.Z -m "...")
□  6. Distribution package created (./scripts/package-linux.sh)
□  7. crates.io publish (if applicable)
□  8. GitHub release published
□  9. Driver versions bumped and published (if applicable)
□  10. Documentation updated (docs/)
```

## Post-Release

After a release:

1. **Bump version to next development iteration**: Update the version
   in all `Cargo.toml` files to the next alpha version (e.g., `1.4.0-alpha`)
2. **Update driver versions**: Sync version strings in all language drivers
   (Python `setup.py`, Node `package.json`, Ruby gemspec, Java POM)
3. **Deploy documentation**: If using a documentation site, trigger a rebuild

## Emergency Releases

For critical bug fixes or security patches:

1. Create a fix branch from the release tag:
   ```bash
   git checkout -b fix/v1.3.1 v1.3.1-alpha
   ```
2. Apply the fix
3. Bump the patch version: `1.3.2-alpha`
4. Follow the standard release process from step 2 onward
5. Merge the fix branch back to main
