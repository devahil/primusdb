#!/usr/bin/env bash
#
# PrimusDB Database Creation Example
#
# Prerequisites: PrimusDB server must be running on localhost:8080
#
# Run: bash examples/cli/create-database.sh
#
set -euo pipefail

BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
[[ -f "$BINARY" ]] || { echo "Build PrimusDB first: cargo build"; exit 1; }

SERVER="${SERVER:-http://localhost:8080}"

echo "=== PrimusDB Database Example ==="
echo "Server: $SERVER"
echo ""

# Check if server is running
if ! curl -sf "$SERVER/health" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $SERVER"
    exit 1
fi

echo "--- Creating a document database ---"
"$BINARY" db create "mydb" --engine document

echo ""
echo "--- Creating a relational table ---"
"$BINARY" db create "users" --engine relational

echo ""
echo "--- Listing databases ---"
"$BINARY" db list

echo ""
echo "--- Describing a database ---"
"$BINARY" db describe "mydb"

echo ""
echo "Done."
