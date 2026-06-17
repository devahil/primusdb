#!/usr/bin/env bash
# stop-local-cluster.sh — Stop a local 3-node PrimusDB cluster
set -euo pipefail
echo "=== Stopping PrimusDB cluster ==="
for port in 8081 8082 8083; do
    PID=$(lsof -ti "tcp:${port}" 2>/dev/null || true)
    if [[ -n "$PID" ]]; then
        echo "Stopping node on port ${port} (PID: ${PID})..."
        kill "$PID" 2>/dev/null || true
        sleep 1
        if kill -0 "$PID" 2>/dev/null; then
            echo "Force stopping..."
            kill -9 "$PID" 2>/dev/null || true
        fi
    else
        echo "No process on port ${port}."
    fi
done
echo "Cluster stopped."
