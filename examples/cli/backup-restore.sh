#!/usr/bin/env bash
#
# PrimusDB Backup & Restore Example
#
# Prerequisites: PrimusDB server must be running on localhost:8080
#
# Run: bash examples/cli/backup-restore.sh
#
set -euo pipefail

BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
[[ -f "$BINARY" ]] || { echo "Build PrimusDB first: cargo build"; exit 1; }

SERVER="${SERVER:-http://localhost:8080}"
BACKUP_DIR="./backups/example-$(date +%Y%m%d-%H%M%S)"

echo "=== PrimusDB Backup & Restore Example ==="
echo "Server: $SERVER"
echo "Backup destination: $BACKUP_DIR"
echo ""

if ! curl -sf "$SERVER/health" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $SERVER"
    exit 1
fi

echo "--- Creating backup ---"
"$BINARY" backup create --destination "$BACKUP_DIR" --description "Example backup"

echo ""
echo "--- Listing backups ---"
"$BINARY" backup list --directory ./backups

echo ""
echo "Done."
