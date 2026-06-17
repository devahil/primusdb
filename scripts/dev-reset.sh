#!/usr/bin/env bash
#
# dev-reset.sh — Reset local PrimusDB development environment
#
# Stops the server, removes data directories, and regenerates config.
#
# Usage:
#   ./scripts/dev-reset.sh
#   ./scripts/dev-reset.sh --data-dir ./data
#
set -euo pipefail

DATA_DIR="${DATA_DIR:-./data}"
HELP=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --data-dir) DATA_DIR="$2"; shift 2 ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [--data-dir DIR]"
    echo "  --data-dir DIR    Data directory to reset (default: ./data)"
    exit 0
fi

echo "Resetting PrimusDB development environment..."
echo ""

# Stop any running server
PORT="${PORT:-8080}"
PID=$(lsof -ti "tcp:${PORT}" 2>/dev/null || true)
if [[ -n "$PID" ]]; then
    echo "Stopping running server (PID: ${PID})..."
    kill "$PID" 2>/dev/null || true
    sleep 1
fi

if [[ -z "${SKIP_DATA:-}" ]]; then
    # Remove data directory
    if [[ -d "$DATA_DIR" ]]; then
        echo "Removing data directory: ${DATA_DIR}"
        rm -rf "$DATA_DIR"
    fi

    # Remove default data dir too
    if [[ -d "./data" && "$DATA_DIR" != "./data" ]]; then
        echo "Removing default data directory: ./data"
        rm -rf "./data"
    fi
fi

# Regenerate config if primusdb.toml exists
if [[ -f "primusdb.toml" ]]; then
    echo "Removing existing config file"
    rm -f "primusdb.toml"
fi

echo ""
echo "Reset complete. Start a fresh server with:"
echo "  cargo run -- server start"
