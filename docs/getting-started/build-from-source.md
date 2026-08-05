# Building PrimusDB from Source

This guide covers compiling PrimusDB v1.3.2-alpha from source on Linux, macOS, and Windows.

## Prerequisites

### Rust Toolchain

PrimusDB requires **Rust 1.70 or later**.

```bash
# Install rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Load the Rust environment
source "$HOME/.cargo/env"

# Verify the toolchain version
rustc --version   # Must be >= 1.70.0
cargo --version   # Must be >= 1.70.0

# Keep your toolchain updated
rustup update stable
```

### System Dependencies

**Ubuntu / Debian:**
```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev
```

**Fedora / CentOS / RHEL:**
```bash
# Fedora
sudo dnf groupinstall "Development Tools"
sudo dnf install openssl-devel

# CentOS / RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel
```

**Arch Linux:**
```bash
sudo pacman -S base-devel openssl
```

**macOS:**
```bash
# Using Homebrew
brew install openssl

# Using MacPorts
sudo port install openssl
```

**Windows:**

Install [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) and ensure the "Desktop development with C++" workload is selected. OpenSSL is not required on Windows — the `ring` crate provides cryptography natively.

## Clone the Repository

```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
```

## Build

### Debug Build

Build with debug symbols and no optimizations. Suitable for development and testing.

```bash
cargo build
```

The binaries are written to `target/debug/`:

| Binary | Path |
|--------|------|
| `primusdb` | `target/debug/primusdb` |

### Release Build

Build with full optimisations, LTO, and stripped symbols. Use for production.

```bash
cargo build --release
```

Binaries are written to `target/release/`:

```bash
ls -la target/release/primusdb*
```

The release profile is configured in `Cargo.toml`:

```toml
[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

### Build the Entire Workspace

The project uses a Cargo workspace that includes library crates and the Rust driver:

```bash
# Build everything (workspace members)
cargo build --release --workspace

# Build a specific member
cargo build --release -p primusdb-core
cargo build --release -p primusdb-storage
```

Workspace members:

| Crate | Description |
|-------|-------------|
| `primusdb-core` | Core engine types and traits |
| `primusdb-storage` | Storage engine implementations |
| `primusdb-crypto` | Encryption and hashing |
| `primusdb-consensus` | Consensus mechanism |
| `primusdb-transaction` | Transaction management |
| `primusdb-ai` | AI/ML inference and training |
| `primusdb-cluster` | Cluster coordination |
| `primusdb-drivers` | Driver protocol layer |
| `primusdb-api` | REST API and routing |
| `primusdb-error` | Error types |
| `drivers/rust` | Rust language driver |

### Feature Flags

Currently no Cargo feature flags are defined, but the build system supports the standard Cargo flag mechanism:

```bash
# Example (flags are reserved for future use)
cargo build --release --features "some-feature"
cargo build --release --all-features
cargo build --release --no-default-features
```

Run `cargo build --release --help` and look for the "Available features" section for an up-to-date list.

## Building Language Drivers

All language drivers are located in the `drivers/` directory and must be built from source.

### Python Driver

```bash
cd drivers/python
pip install setuptools-rust
pip install -e .
```

### Node.js Driver

```bash
cd drivers/node
npm install
npm run build
```

### Java Driver

```bash
cd drivers/java
mvn clean compile
```

### Ruby Driver

```bash
cd drivers/ruby
gem build primusdb.gemspec
gem install ./primusdb-*.gem
```

### Rust Driver

```bash
cd drivers/rust
cargo build
```

The Rust driver is also a workspace member and can be built from the root:

```bash
cargo build --release -p primusdb-rust-driver
```

## Common Build Errors

### `error: linker 'cc' not found`

Install a C compiler:

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora
sudo dnf groupinstall "Development Tools"

# macOS
xcode-select --install
```

### `error: failed to run custom build command for 'openssl-sys'`

OpenSSL development headers are missing:

```bash
# Ubuntu/Debian
sudo apt-get install pkg-config libssl-dev

# Fedora
sudo dnf install openssl-devel

# macOS
brew install openssl

# Windows (not typically needed — ring is used instead)
```

### `error: No such file or directory (os error 2)` during `ring` build

On some Linux distributions, `ring` requires `clang`:

```bash
sudo apt-get install clang
```

### Out of memory during compilation

The release build can consume several GB of RAM. Reduce parallelism:

```bash
# Limit to 4 concurrent jobs
CARGO_BUILD_JOBS=4 cargo build --release

# Or use a lower-codegen-unit profile
cargo build --release --config 'profile.release.codegen-units = 16'
```

### Rust version too old

Update your Rust toolchain:

```bash
rustup update stable
rustc --version   # Must be >= 1.70.0
```

### `could not find Cargo.toml`

Make sure you are in the project root directory:

```bash
ls Cargo.toml   # Should exist in the primusdb root
```

### Slow compilation

- Use `cargo build` (debug) for development iterations
- Install [mold](https://github.com/rui314/mold) or [lld](https://lld.llvm.org/) for faster linking
- Use [sccache](https://github.com/mozilla/sccache) for build caching

```bash
# Install mold (Linux)
sudo apt-get install mold   # or your distro's equivalent
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Or use sccache
cargo install sccache
export RUSTC_WRAPPER=sccache

cargo build --release
```

## Verifying the Build

```bash
# Check version
./target/release/primusdb --version

# Run the test suite
cargo test

# Run benchmarks
cargo bench
```
