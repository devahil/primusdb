#!/usr/bin/env bash
#
# PrimusDB Query Example
#
# Prerequisites: PrimusDB server must be running on localhost:8080
#
# Run: bash examples/cli/query-basic.sh
#
set -euo pipefail

BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
[[ -f "$BINARY" ]] || { echo "Build PrimusDB first: cargo build"; exit 1; }

SERVER="${SERVER:-http://localhost:8080}"

echo "=== PrimusDB Query Example ==="
echo "Server: $SERVER"
echo ""

if ! curl -sf "$SERVER/health" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $SERVER"
    exit 1
fi

echo "--- Running a SQL query ---"
"$BINARY" query "SELECT * FROM information_schema.tables" --format table

echo ""
echo "--- Running a query with JSON output ---"
"$BINARY" query "SELECT 1 AS value" --format json

echo ""
echo "Done."
