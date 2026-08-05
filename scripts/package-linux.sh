#!/usr/bin/env bash
#
# package-linux.sh — Create a Linux binary tarball for distribution
#
# Usage:
#   ./scripts/package-linux.sh
#   ./scripts/package-linux.sh --output ./dist
#   ./scripts/package-linux.sh --version 1.3.1-alpha
#
set -euo pipefail

OUTPUT_DIR="./dist"
VERSION=""
HELP=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT_DIR="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [--output DIR] [--version VERSION]"
    echo "  --output DIR      Output directory (default: ./dist)"
    echo "  --version VER     Package version (default: from Cargo.toml)"
    exit 0
fi

# Get version from Cargo.toml if not specified
if [[ -z "$VERSION" ]]; then
    VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
fi

ARCH=$(uname -m)
PKG_NAME="primusdb-${VERSION}-linux-${ARCH}"

echo "Packaging PrimusDB ${VERSION} for Linux (${ARCH})"
echo ""

# Ensure release binaries exist
if [[ ! -f target/release/primusdb ]]; then
    echo "Release binary not found. Building..."
    cargo build --release --workspace
fi

mkdir -p "${OUTPUT_DIR}/${PKG_NAME}"

# Copy binaries
echo "Copying binaries..."
install -m 755 target/release/primusdb "${OUTPUT_DIR}/${PKG_NAME}/primusdb"

# Copy docs and configs
echo "Copying documentation..."
cp -r docs "${OUTPUT_DIR}/${PKG_NAME}/" 2>/dev/null || true
cp README.md "${OUTPUT_DIR}/${PKG_NAME}/" 2>/dev/null || true
cp LICENSE "${OUTPUT_DIR}/${PKG_NAME}/" 2>/dev/null || true

# Copy example configs
if [[ -d config/examples ]]; then
    cp -r config/examples "${OUTPUT_DIR}/${PKG_NAME}/config/" 2>/dev/null || true
fi

# Create tarball
echo "Creating tarball..."
cd "${OUTPUT_DIR}"
tar czf "${PKG_NAME}.tar.gz" "${PKG_NAME}"
rm -rf "${PKG_NAME}"
cd ..

echo ""
echo "Package created: ${OUTPUT_DIR}/${PKG_NAME}.tar.gz"
echo "Size: $(ls -lh "${OUTPUT_DIR}/${PKG_NAME}.tar.gz" | awk '{print $5}')"
