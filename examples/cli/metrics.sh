#!/usr/bin/env bash
#
# PrimusDB Metrics Example
#
# Prerequisites: PrimusDB server must be running on localhost:8080
#
# Run: bash examples/cli/metrics.sh
#
set -euo pipefail

echo "=== PrimusDB Metrics ==="
echo ""

echo "--- Metrics endpoint ---"
curl -sf http://localhost:8080/metrics || echo "Metrics endpoint not available"

echo ""
echo "--- PrimusDB metrics command ---"
BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
if [[ -f "$BINARY" ]]; then
    "$BINARY" metrics
else
    echo "Build PrimusDB first: cargo build"
fi

echo ""
echo "Done."
