#!/bin/bash
# PrimusDB Cluster Demo — Federation Example
# Run this after seed.sh to set up cross-cluster DataDomains.
set -e

COORDINATOR="http://coordinator:8081"
FED_PEER="http://fed-peer:8084"

echo "⏳ Waiting for federation peers..."
sleep 15

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║  Setting up Federation DataDomains                  ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# ─────────────────────────────────────────────────────────
# Create a DataDomain that spans both cluster-a and itself
# ─────────────────────────────────────────────────────────
echo "[1/2]  Creating DataDomain → global-users (Quorum mode)"
curl -sf -X POST "$COORDINATOR/api/v1/federation/domains" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "global-users",
    "description": "User profiles replicated across federated clusters",
    "replication_mode": "Quorum",
    "storage_types": ["document"],
    "collections": ["users"],
    "tables": [],
    "member_clusters": ["cluster-a"]
  }' | python3 -m json.tool
echo ""

echo "[2/2]  Checking federation status"
curl -sf "$COORDINATOR/api/v1/federation/status" | python3 -m json.tool
echo ""

echo "╔══════════════════════════════════════════════════════╗"
echo "║  Federation active!                                 ║"
echo "║                                                     ║"
echo "║   curl localhost:8080/api/v1/federation/clusters    ║"
echo "║   curl localhost:8080/api/v1/federation/domains     ║"
echo "╚══════════════════════════════════════════════════════╝"
