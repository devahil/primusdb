# PrimusDB Build and Compilation Guide
=====================================

This guide covers building PrimusDB v1.2.0-alpha from source, including all dependencies, compilation options, and troubleshooting.

## Prerequisites

### System Requirements
- **OS**: Linux (Ubuntu 20.04+, CentOS 7+, Arch Linux), macOS 10.15+, Windows 10+
- **CPU**: x86_64, ARM64
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 2GB free space for build artifacts
- **Network**: Internet connection for dependencies

### Required Tools
```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version  # 1.70.0 or later
cargo --version  # 1.70.0 or later
```

### Optional Tools
```bash
# For development
sudo apt-get install build-essential pkg-config libssl-dev  # Ubuntu/Debian
sudo dnf install gcc openssl-devel                        # Fedora/CentOS
sudo pacman -S base-devel openssl                         # Arch Linux

# For cross-compilation
rustup target add x86_64-unknown-linux-musl
rustup target add aarch64-unknown-linux-gnu

# For benchmarking
cargo install cargo-criterion
```

## Quick Start Build

### Standard Release Build
```bash
# Clone repository
git clone https://github.com/devahil/primusdb.git
cd primusdb

# Build release binaries
cargo build --release

# Verify build
ls -la target/release/
# primusdb-server, primusdb-cli, primusdb
```

### Development Build
```bash
# Debug build with debug symbols
cargo build

# Run tests
cargo test

# Run specific test
cargo test test_crud_operations
```

### Optimized Build
```bash
# Maximum optimization
RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat" cargo build --release

# Size optimized
cargo build --release --config 'profile.release.opt-level = "z"'

# With debug info for profiling
cargo build --release --config 'profile.release.debug = true'
```

## Build Configuration

### Cargo Profiles
```toml
# .cargo/config.toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 0
debug = true
overflow-checks = true
```

### Feature Flags
```bash
# Build with all features
cargo build --release --all-features

# Build with specific features
cargo build --release --features "ai_engine,advanced_analytics"

# List available features
cargo build --help | grep -A 20 "Available features"
```

### Cross-Compilation

#### Linux (musl static)
```bash
# Install target
rustup target add x86_64-unknown-linux-musl

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl

# Verify static linking
ldd target/x86_64-unknown-linux-musl/release/primusdb-server
# Should show "not a dynamic executable"
```

#### ARM64
```bash
# Install target
rustup target add aarch64-unknown-linux-gnu

# Install linker
sudo apt-get install gcc-aarch64-linux-gnu

# Build
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
cargo build --release --target aarch64-unknown-linux-gnu
```

#### macOS Universal Binary
```bash
# Build for both architectures
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

# Combine with lipo (requires Xcode)
lipo -create \
  target/x86_64-apple-darwin/release/primusdb-server \
  target/aarch64-apple-darwin/release/primusdb-server \
  -output primusdb-server-universal
```

## Dependency Management

### Cargo Dependencies
```toml
# Cargo.toml key dependencies
[dependencies]
tokio = { version = "1.0", features = ["full"] }
axum = "0.7"
sled = "0.34"
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
sha2 = "0.10"
aes-gcm = "0.10"
lz4 = "1.28"
```

### System Dependencies

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    liblz4-dev \
    zlib1g-dev \
    libzstd-dev
```

#### CentOS/RHEL/Fedora
```bash
# CentOS/RHEL
sudo yum groupinstall "Development Tools"
sudo yum install openssl-devel lz4-devel zlib-devel libzstd-devel

# Fedora
sudo dnf groupinstall "Development Tools"
sudo dnf install openssl-devel lz4-devel zlib-devel libzstd-devel
```

#### Arch Linux
```bash
sudo pacman -S base-devel openssl lz4 zlib zstd
```

#### macOS
```bash
# Using Homebrew
brew install openssl lz4 zlib zstd

# Or using MacPorts
sudo port install openssl lz4 zlib zstd
```

## Build Troubleshooting

### Common Issues

#### Out of Memory
```bash
# Reduce codegen units
cargo build --release --config 'profile.release.codegen-units = 1'

# Use swap file
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

#### Linker Errors
```bash
# Install missing libraries
sudo apt-get install libssl-dev liblz4-dev

# Check library paths
pkg-config --libs openssl
pkg-config --libs liblz4
```

#### Network Issues
```bash
# Use local registry mirror
# Edit ~/.cargo/config
[source.crates-io]
registry = "https://github.com/rust-lang/crates.io-index"
replace-with = 'mirror'

[source.mirror]
registry = "https://mirrors.tuna.tsinghua.edu.cn/git/crates.io-index.git"
```

#### Compilation Speed
```bash
# Use more threads
export CARGO_BUILD_JOBS=8

# Enable incremental compilation
echo 'export CARGO_INCREMENTAL=1' >> ~/.bashrc

# Use mold linker (Linux)
sudo apt-get install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Debug Build Issues
```bash
# Verbose output
cargo build -v

# Check dependencies
cargo tree

# Clean and rebuild
cargo clean
cargo build

# Check Rust version compatibility
rustup show
rustup update
```

## Advanced Build Options

### Custom Linker
```bash
# Use lld (faster linking)
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"

# Use mold (fastest)
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
```

### Build Caching
```bash
# Install sccache for faster rebuilds
cargo install sccache
export RUSTC_WRAPPER=sccache

# Check cache stats
sccache --show-stats
```

### Docker Build
```dockerfile
FROM rust:1.70-slim as builder

WORKDIR /usr/src/primusdb

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Build actual source
COPY src ./src
RUN touch src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /usr/src/primusdb/target/release/primusdb-server /usr/local/bin/
EXPOSE 8080
CMD ["primusdb-server"]
```

### CI/CD Build
```yaml
# GitHub Actions example
name: Build and Test
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - uses: actions-rs/cargo@v1
      with:
        command: build
        args: --release
    - uses: actions-rs/cargo@v1
      with:
        command: test
```

## Binary Optimization

### Size Optimization
```bash
# Strip debug symbols
strip target/release/primusdb-server

# UPX compression
upx --best target/release/primusdb-server

# Check final size
ls -lh target/release/primusdb-server
```

### Performance Profiling
```bash
# Install perf tools
sudo apt-get install linux-tools-common linux-tools-generic

# Profile build
cargo build --release
perf record target/release/primusdb-server --help
perf report
```

### Benchmarking
```bash
# Install criterion
cargo install cargo-criterion

# Run benchmarks
cargo criterion

# Custom benchmarks
cargo bench --bench my_benchmark
```

## Distribution

### Packaging

#### Debian Package
```bash
# Install cargo-deb
cargo install cargo-deb

# Build .deb package
cargo deb

# Install package
sudo dpkg -i target/debian/primusdb_1.0.0_amd64.deb
```

#### RPM Package
```bash
# Install cargo-rpm
cargo install cargo-rpm

# Build .rpm package
cargo rpm build

# Install package
sudo rpm -i target/release/rpmbuild/RPMS/x86_64/primusdb-1.0.0-1.x86_64.rpm
```

#### Docker Image
```bash
# Build multi-stage image
docker build -t primusdb:latest .

# Build distroless image
docker build -f Dockerfile.distroless -t primusdb:distroless .

# Push to registry
docker tag primusdb:latest registry.example.com/primusdb:latest
docker push registry.example.com/primusdb:latest
```

### Distribution Checklist
- [ ] Binaries stripped of debug symbols
- [ ] Static linking where possible
- [ ] Minimal runtime dependencies
- [ ] Proper file permissions
- [ ] Version information embedded
- [ ] License files included
- [ ] Documentation bundled
- [ ] Configuration examples provided

## Security Considerations

### Build Security
```bash
# Verify checksums
sha256sum target/release/primusdb-server

# Sign binaries
gpg --detach-sign target/release/primusdb-server

# Use reproducible builds
SOURCE_DATE_EPOCH=$(git log -1 --format=%ct) cargo build --release
```

### Supply Chain Security
```bash
# Audit dependencies
cargo install cargo-audit
cargo audit

# Check for vulnerabilities
cargo install cargo-deny
cargo deny check

# License compliance
cargo install cargo-license
cargo license
```

This build guide ensures consistent, secure, and optimized compilation of PrimusDB across all supported platforms.