#!/usr/bin/env bash
#
# install.sh — Install PrimusDB from source
#
# Usage:
#   ./scripts/install.sh              # Build release and install to ~/.cargo/bin
#   ./scripts/install.sh --prefix /usr/local
#   ./scripts/install.sh --no-install  # Build only
#
set -euo pipefail

PREFIX="${PREFIX:-${HOME}/.cargo/bin}"
NO_INSTALL=0
HELP=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix) PREFIX="$2"; shift 2 ;;
        --no-install) NO_INSTALL=1; shift ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [--prefix PATH] [--no-install]"
    echo "  --prefix PATH    Install binaries to PATH (default: ~/.cargo/bin)"
    echo "  --no-install     Build only, do not install"
    exit 0
fi

# Detect Rust toolchain
if ! command -v rustc &>/dev/null; then
    echo "Rust toolchain not found."
    echo "Install it with: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo "Rust version: $(rustc --version)"
echo "Cargo version: $(cargo --version)"
echo ""

# Build release
echo "Building PrimusDB release binary..."
cargo build --release --workspace

echo ""
echo "Build complete."

if [[ "$NO_INSTALL" -eq 0 ]]; then
    mkdir -p "$PREFIX"
    install -m 755 target/release/primusdb "$PREFIX/primusdb"
    echo "Installed 'primusdb' to $PREFIX"

    echo ""
    echo "Installation complete."
    echo "Make sure $PREFIX is in your PATH."
    echo ""
    echo "Quick start:"
    echo "  primusdb config init                  # Generate config"
    echo "  primusdb server start                 # Start server"
    echo "  primusdb version                      # Verify installation"
fi
