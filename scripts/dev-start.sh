#!/usr/bin/env bash
#
# dev-start.sh — Start a local PrimusDB development server
#
# Usage:
#   ./scripts/dev-start.sh              # Start with defaults
#   ./scripts/dev-start.sh --port 8081  # Custom port
#   ./scripts/dev-start.sh --release    # Use release build
#
set -euo pipefail

PORT="${PORT:-8080}"
HOST="${HOST:-127.0.0.1}"
DATA_DIR="${DATA_DIR:-./data/dev}"
BUILD_MODE="debug"
CONFIG_FILE=""
HELP=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --port) PORT="$2"; shift 2 ;;
        --host) HOST="$2"; shift 2 ;;
        --data-dir) DATA_DIR="$2"; shift 2 ;;
        --config) CONFIG_FILE="$2"; shift 2 ;;
        --release) BUILD_MODE="release"; shift ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [options]"
    echo "  --port PORT        Server port (default: 8080)"
    echo "  --host HOST        Bind address (default: 127.0.0.1)"
    echo "  --data-dir DIR     Data directory (default: ./data/dev)"
    echo "  --config FILE      Config file path"
    echo "  --release          Use release build instead of debug"
    exit 0
fi

# Pick the right binary
if [[ "$BUILD_MODE" == "release" ]]; then
    if [[ ! -f target/release/primusdb ]]; then
        echo "Release binary not found. Run 'cargo build --release' first."
        exit 1
    fi
    BINARY="target/release/primusdb"
else
    if [[ ! -f target/debug/primusdb ]]; then
        echo "Debug binary not found. Building..."
        cargo build
    fi
    BINARY="target/debug/primusdb"
fi

# Ensure data dir exists
mkdir -p "$DATA_DIR"

echo "Starting PrimusDB development server..."
echo "  Host:      $HOST"
echo "  Port:      $PORT"
echo "  Data dir:  $DATA_DIR"
echo "  Binary:    $BINARY"
echo "  Config:    ${CONFIG_FILE:-default}"
echo ""

if [[ -n "$CONFIG_FILE" ]]; then
    exec "$BINARY" server start --bind "${HOST}:${PORT}" --data-dir "$DATA_DIR" --config "$CONFIG_FILE" --log-level debug
else
    exec "$BINARY" server start --bind "${HOST}:${PORT}" --data-dir "$DATA_DIR" --log-level debug
fi
