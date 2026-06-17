#!/usr/bin/env bash
#
# dev-stop.sh — Stop a local PrimusDB development server
#
# Usage:
#   ./scripts/dev-stop.sh
#   ./scripts/dev-stop.sh --port 8081
#
set -euo pipefail

PORT="${PORT:-8080}"
HELP=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --port) PORT="$2"; shift 2 ;;
        --help|-h) HELP=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ "$HELP" -eq 1 ]]; then
    echo "Usage: $0 [--port PORT]"
    echo "  --port PORT    Server port (default: 8080)"
    exit 0
fi

# Find PID listening on the port
PID=$(lsof -ti "tcp:${PORT}" 2>/dev/null || true)

if [[ -z "$PID" ]]; then
    echo "No process found listening on port ${PORT}."
    exit 0
fi

echo "Stopping PrimusDB server (PID: ${PID}) on port ${PORT}..."
kill "$PID" 2>/dev/null || true

# Wait for process to exit
for _ in $(seq 1 10); do
    if ! kill -0 "$PID" 2>/dev/null; then
        echo "Server stopped."
        exit 0
    fi
    sleep 0.5
done

echo "Server did not stop gracefully. Forcing..."
kill -9 "$PID" 2>/dev/null || true
echo "Server force-stopped."
