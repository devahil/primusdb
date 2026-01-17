#!/bin/bash

# PrimusDB Drivers Build Script
# Builds all language drivers for PrimusDB

set -e

echo "🔨 Building PrimusDB Drivers..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Function to print status messages
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    print_error "Cargo (Rust) is not installed. Please install Rust first."
    exit 1
fi

# Build core PrimusDB first
print_status "Building core PrimusDB..."
cd "$PROJECT_ROOT"
cargo build --release

# Build Rust driver
print_status "Building Rust driver..."
cd "$PROJECT_ROOT/drivers/rust"
cargo build --release

# Build Python driver (if Python is available)
if command -v python3 &> /dev/null; then
    print_status "Building Python driver..."
    cd "$PROJECT_ROOT/drivers/python"

    # Check if setuptools-rust is available
    if python3 -c "import setuptools_rust" 2>/dev/null; then
        python3 setup.py build_ext --inplace
        print_status "Python driver built successfully"
    else
        print_warning "setuptools-rust not available. Install with: pip install setuptools-rust"
        print_warning "Skipping Python driver build"
    fi
else
    print_warning "Python3 not found. Skipping Python driver build"
fi

# Build Java driver (if Maven is available)
if command -v mvn &> /dev/null; then
    print_status "Building Java JDBC driver..."
    cd "$PROJECT_ROOT/drivers/java"

    if [ -f "pom.xml" ]; then
        mvn clean compile package -DskipTests
        print_status "Java JDBC driver built successfully"
    else
        print_warning "Java driver pom.xml not found"
    fi
else
    print_warning "Maven not found. Skipping Java driver build"
fi

# Build Ruby driver (if Ruby is available)
if command -v ruby &> /dev/null && command -v gem &> /dev/null; then
    print_status "Building Ruby driver..."
    cd "$PROJECT_ROOT/drivers/ruby"

    if [ -f "primusdb.gemspec" ]; then
        gem build primusdb.gemspec
        print_status "Ruby driver gem built successfully"
    else
        print_warning "Ruby driver gemspec not found"
    fi
else
    print_warning "Ruby/Gem not found. Skipping Ruby driver build"
fi

# Build Rust Python bindings (if PyO3 feature is enabled)
print_status "Building Rust Python bindings..."
cd "$PROJECT_ROOT/crates/primusdb-drivers"
cargo build --release --features python

# Build Rust Java bindings (if JNI feature is enabled)
print_status "Building Rust Java bindings..."
cargo build --release --features java

# Build Rust Ruby bindings (if Rutie feature is enabled)
print_status "Building Rust Ruby bindings..."
cargo build --release --features ruby

print_status "🎉 All drivers built successfully!"
print_status ""
print_status "Driver locations:"
print_status "  Rust: $PROJECT_ROOT/drivers/rust/target/release/libprimusdb_rust_driver.rlib"
print_status "  Python: $PROJECT_ROOT/drivers/python/"
print_status "  Java: $PROJECT_ROOT/drivers/java/target/"
print_status "  Ruby: $PROJECT_ROOT/drivers/ruby/primusdb-0.1.0.gem"
print_status "  Rust Bindings: $PROJECT_ROOT/crates/primusdb-drivers/target/release/"
print_status ""
print_status "To test drivers:"
print_status "  Rust: cd drivers/rust && cargo test"
print_status "  Python: cd drivers/python && python -c \"import primusdb; print('Import successful')\""
print_status "  Java: cd drivers/java && mvn test"
print_status "  Ruby: gem install drivers/ruby/primusdb-0.1.0.gem && ruby -e \"require 'primusdb'; puts 'Import successful'\""