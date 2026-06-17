#!/usr/bin/env bash
# dev-restart.sh — Restart local PrimusDB development server
set -euo pipefail
PORT="${PORT:-8080}"
bash scripts/dev-stop.sh --port "$PORT"
sleep 1
bash scripts/dev-start.sh --port "$PORT"
