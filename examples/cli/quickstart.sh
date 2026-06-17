#!/usr/bin/env bash
#
# PrimusDB CLI Quickstart
#
# Run: bash examples/cli/quickstart.sh
#
set -euo pipefail

echo "=== PrimusDB CLI Quickstart ==="
echo ""

# Check binary
BINARY="./target/debug/primusdb"
if [[ ! -f "$BINARY" ]]; then
    BINARY="./target/release/primusdb"
fi
if [[ ! -f "$BINARY" ]]; then
    echo "Building PrimusDB..."
    cargo build
    BINARY="./target/debug/primusdb"
fi

echo "Using binary: $BINARY"
echo ""

# Version
echo "--- Version ---"
"$BINARY" version

echo ""
echo "--- Help ---"
"$BINARY" --help

echo ""
echo "--- Doctor (local diagnostics) ---"
"$BINARY" doctor
