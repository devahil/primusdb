#!/usr/bin/env bash
# dev-status.sh — Show status of local PrimusDB development server
set -euo pipefail
PORT="${PORT:-8080}"
echo "=== PrimusDB Status ==="
echo "Port:     ${PORT}"
PID=$(lsof -ti "tcp:${PORT}" 2>/dev/null || true)
if [[ -n "$PID" ]]; then
    echo "PID:      ${PID}"
    echo "Process:  $(ps -p "$PID" -o comm= 2>/dev/null || echo 'unknown')"
    echo "Uptime:   $(ps -o etime= -p "$PID" 2>/dev/null || echo 'N/A')"
    echo "Status:   Running"
    # Check health
    if curl -sf "http://127.0.0.1:${PORT}/health" > /dev/null 2>&1; then
        echo "Health:   Healthy"
    else
        echo "Health:   Unreachable"
    fi
else
    echo "Status:   Not running"
fi
