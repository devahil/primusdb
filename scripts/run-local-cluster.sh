#!/usr/bin/env bash
# run-local-cluster.sh — Start a 3-node PrimusDB cluster locally
set -euo pipefail
echo "=== Starting 3-node PrimusDB cluster ==="
echo ""
echo "Starting node 1 on port 8081..."
PRIMUSDB_DATA_DIR="./data/cluster/node1" PRIMUSDB_PORT=8081 PRIMUSDB_NODE_ID="node1" PRIMUSDB_CLUSTER_ENABLED=true \
    cargo run -- server start --bind "127.0.0.1:8081" --data-dir "./data/cluster/node1" --daemon &
sleep 2
echo "Starting node 2 on port 8082..."
PRIMUSDB_DATA_DIR="./data/cluster/node2" PRIMUSDB_PORT=8082 PRIMUSDB_NODE_ID="node2" PRIMUSDB_CLUSTER_ENABLED=true \
    cargo run -- server start --bind "127.0.0.1:8082" --data-dir "./data/cluster/node2" --daemon &
sleep 2
echo "Starting node 3 on port 8083..."
PRIMUSDB_DATA_DIR="./data/cluster/node3" PRIMUSDB_PORT=8083 PRIMUSDB_NODE_ID="node3" PRIMUSDB_CLUSTER_ENABLED=true \
    cargo run -- server start --bind "127.0.0.1:8083" --data-dir "./data/cluster/node3" --daemon &
sleep 2
echo ""
echo "Cluster started. Check status:"
echo "  curl http://127.0.0.1:8081/health"
echo "  curl http://127.0.0.1:8082/health"
echo "  curl http://127.0.0.1:8083/health"
echo ""
echo "To stop:"
echo "  kill $(lsof -ti tcp:8081) $(lsof -ti tcp:8082) $(lsof -ti tcp:8083)"
echo "  or run: bash scripts/stop-local-cluster.sh"
