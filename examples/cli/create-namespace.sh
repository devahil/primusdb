#!/usr/bin/env bash
#
# PrimusDB Namespace Example
#
# Prerequisites: PrimusDB server must be running on localhost:8080
#
# Run: bash examples/cli/create-namespace.sh
#
set -euo pipefail

BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
[[ -f "$BINARY" ]] || { echo "Build PrimusDB first: cargo build"; exit 1; }

SERVER="${SERVER:-http://localhost:8080}"

echo "=== PrimusDB Namespace Example ==="
echo "Server: $SERVER"
echo ""

# Check if server is running
if ! curl -sf "$SERVER/health" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $SERVER"
    echo "Start the server: $BINARY server start"
    exit 1
fi

echo "--- Creating namespace ---"
"$BINARY" namespace create "myapp.production" \
    --description "Production namespace for myapp"

echo ""
echo "--- Listing namespaces ---"
"$BINARY" namespace list

echo ""
echo "--- Describing namespace ---"
"$BINARY" namespace describe "myapp.production"

echo ""
echo "Done."
