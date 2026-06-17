#!/usr/bin/env bash
#
# PrimusDB Health Check Example
#
# Run: bash examples/cli/health-check.sh
#
set -euo pipefail

echo "=== PrimusDB Health Check ==="
echo ""

# Health via Docker or direct
if command -v docker &>/dev/null && docker ps --format '{{.Names}}' 2>/dev/null | grep -q primusdb; then
    echo "--- Docker container status ---"
    docker ps --filter "name=primusdb" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
fi

echo "--- Server health endpoint ---"
curl -sf http://localhost:8080/health || echo "Server not running on port 8080"

echo ""
echo "--- Status endpoint ---"
curl -sf http://localhost:8080/status || echo "Status endpoint not available"

echo ""
echo "--- Doctor diagnostics ---"
BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
if [[ -f "$BINARY" ]]; then
    "$BINARY" doctor
else
    echo "Build PrimusDB first: cargo build"
fi

echo ""
echo "Done."
