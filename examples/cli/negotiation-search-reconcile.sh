#!/usr/bin/env bash
#
# PrimusDB Capability Negotiation + Unified Search + Integrity Reconciliation
#
# Demonstrates the v1.3.2+ control plane APIs:
#   1. Capability negotiation: GET /api/v1/capabilities
#   2. Unified search (full-text via the persistent inverted index):
#      GET /api/v1/search?q=...
#   3. Integrity-first reconciliation handshake:
#      GET  /api/v1/databases/:db/integrity/reconcile/evidence
#      POST /api/v1/databases/:db/integrity/reconcile  { "peer_records": [...] }
#
# Prerequisites: a PrimusDB server must be running on localhost:8080
#
# Run: bash examples/cli/negotiation-search-reconcile.sh
#
set -euo pipefail

BINARY="./target/debug/primusdb"
[[ -f "$BINARY" ]] || BINARY="./target/release/primusdb"
[[ -f "$BINARY" ]] || BINARY="primusdb"
SERVER="${SERVER:-http://localhost:8080}"
DB="${DB:-default}"

echo "=== PrimusDB Capability Negotiation + Search + Reconcile ==="
echo "Server: $SERVER   DB: $DB"
echo ""

if ! curl -sf "$SERVER/health" > /dev/null 2>&1; then
    echo "ERROR: Server not reachable at $SERVER"
    exit 1
fi

echo "--- 1. Capability negotiation (client asks: do you support these?) ---"
curl -sf "$SERVER/api/v1/capabilities" | "$BINARY" --format json-inline 2>/dev/null \
    || curl -sf "$SERVER/api/v1/capabilities"
echo ""
echo ""

echo "--- 2. Insert two documents to exercise the persistent search index ---"
curl -sf -X POST "$SERVER/api/v1/crud/document/articles" \
    -H 'Content-Type: application/json' \
    -d '{"id":"neg1","title":"cargo borrow checker internals","body":"rust ownership"}' > /dev/null
curl -sf -X POST "$SERVER/api/v1/crud/document/articles" \
    -H 'Content-Type: application/json' \
    -d '{"id":"neg2","title":"gardening","body":"tomatoes in spring"}' > /dev/null
echo "inserted"
echo ""

echo "--- 3. Unified search: full-text via the persistent index ---"
curl -sf "$SERVER/api/v1/search?q=rust&tables=articles"
echo ""
echo ""

echo "--- 4. Vector routing: search by similarity ---"
curl -sf "$SERVER/api/v1/search?query_vector=%5B1.0,0.0%5D&storage_types=vector&tables=embeddings" \
    || echo "(no vector table seeded yet — insert one with a 'vector' field first)"
echo ""
echo ""

echo "--- 5. Integrity evidence handshake (integrity-first reconciliation) ---"
EVIDENCE="$(curl -sf "$SERVER/api/v1/databases/$DB/integrity/reconcile/evidence")"
echo "$EVIDENCE"
echo ""
PEER_RECORDS="$(curl -sf "$SERVER/api/v1/databases/$DB/integrity/records" | jq -c '.data // []' 2>/dev/null || echo '[]')"

echo "--- 6. Reconcile local chain against peer records (read-only report) ---"
if [[ "$PEER_RECORDS" != "[]" ]]; then
    curl -sf -X POST "$SERVER/api/v1/databases/$DB/integrity/reconcile" \
        -H 'Content-Type: application/json' \
        -d "{\"peer_records\": $PEER_RECORDS}"
else
    echo "(no records endpoint available or empty — send a hand-built peer_records array instead)"
fi
echo ""
echo ""
echo "Done."
