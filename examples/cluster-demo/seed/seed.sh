#!/bin/bash
# PrimusDB Cluster Demo — Seed Data Ingest
set -e

COORDINATOR="http://coordinator:8081"
echo "⏳ Waiting for coordinator..."
until curl -sf "$COORDINATOR/health" > /dev/null 2>&1; do sleep 2; done
echo "✅ Coordinator ready."

echo "⏳ Waiting for cluster quorum..."
sleep 10

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║  Seeding  PrimusDB Cluster Demo Data                ║"
echo "╚══════════════════════════════════════════════════════╝"
echo ""

# ─────────────────────────────────────────────────────────
# 1.  Columnar  — Sales Analytics  (50 records)
# ─────────────────────────────────────────────────────────
echo "[1/6]  Seeding  Columnar  → sales  (50 records)"
for i in $(seq 1 50); do
  year=$((2024 + RANDOM % 2))
  month=$((1 + RANDOM % 12))
  day=$((1 + RANDOM % 28))
  cat="$(echo -e "Electronics\nClothing\nFood\nBooks\nHome" | shuf -n1)"
  region="$(echo -e "North America\nEurope\nAsia Pacific\nLatin America" | shuf -n1)"
  amount=$(python3 -c "import random; print(f'{random.uniform(5.0, 1500.0):.2f}')")
  qty=$((1 + RANDOM % 20))
  curl -sf -X POST "$COORDINATOR/api/v1/crud/columnar/sales" \
    -H "Content-Type: application/json" \
    -d "{
      \"product_id\": $i,
      \"category\": \"$cat\",
      \"amount\": $amount,
      \"quantity\": $qty,
      \"sale_date\": \"$year-$(printf '%02d' $month)-$(printf '%02d' $day)\",
      \"region\": \"$region\",
      \"customer_id\": $((1000 + RANDOM % 500))
    }" > /dev/null
done
echo "       ✔  50 rows inserted."

# ─────────────────────────────────────────────────────────
# 2.  Vector  — Product Embeddings  (20 vectors, 8d)
# ─────────────────────────────────────────────────────────
echo "[2/6]  Seeding  Vector     → product_embeddings  (20 vectors)"
for i in $(seq 1 20); do
  v1=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v2=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v3=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v4=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v5=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v6=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v7=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  v8=$(python3 -c "import random; print(round(random.uniform(-1,1), 4))")
  tags="$(echo -e "[\"electronics\"]\n[\"clothing\"]\n[\"book\"]\n[\"home\"]\n[\"food\"]" | shuf -n1)"
  curl -sf -X POST "$COORDINATOR/api/v1/crud/vector/product_embeddings" \
    -H "Content-Type: application/json" \
    -d "{
      \"id\": \"prod_emb_$(printf '%03d' $i)\",
      \"vector\": [$v1,$v2,$v3,$v4,$v5,$v6,$v7,$v8],
      \"metadata\": {\"product_id\": $i, \"tags\": $tags}
    }" > /dev/null
done
echo "       ✔  20 vectors inserted."

# ─────────────────────────────────────────────────────────
# 3.  Document  — Users  (15 profiles)
# ─────────────────────────────────────────────────────────
echo "[3/6]  Seeding  Document   → users  (15 profiles)"
names=("Alice Johnson" "Bob Smith" "Carol White" "David Brown" "Eva Martinez"
       "Frank Lee" "Grace Kim" "Henry Davis" "Iris Chen" "Jack Wilson"
       "Kate Taylor" "Leo Anders" "Mia Thomas" "Noah Garcia" "Olivia Clark")
for i in $(seq 0 14); do
  name="${names[$i]}"
  email="$(echo "$name" | tr '[:upper:]' '[:lower:]' | tr ' ' '.')@example.com"
  age=$((18 + RANDOM % 52))
  tier="$(echo -e "basic\npremium\nenterprise" | shuf -n1)"
  curl -sf -X POST "$COORDINATOR/api/v1/crud/document/users" \
    -H "Content-Type: application/json" \
    -d "{
      \"name\": \"$name\",
      \"email\": \"$email\",
      \"age\": $age,
      \"tier\": \"$tier\",
      \"created_at\": \"2025-01-01T00:00:00Z\",
      \"preferences\": {\"notifications\": true, \"theme\": \"dark\"}
    }" > /dev/null
done
echo "       ✔  15 documents inserted."

# ─────────────────────────────────────────────────────────
# 3b. Document — Products  (20 catalog items)
# ─────────────────────────────────────────────────────────
echo "[4/6]  Seeding  Document   → products  (20 catalog items)"
for i in $(seq 1 20); do
  cat="$(echo -e "Electronics\nClothing\nBooks\nHome & Garden\nSports" | shuf -n1)"
  price=$(python3 -c "import random; print(f'{random.uniform(5.0, 500.0):.2f}')")
  stock=$((0 + RANDOM % 200))
  curl -sf -X POST "$COORDINATOR/api/v1/crud/document/products" \
    -H "Content-Type: application/json" \
    -d "{
      \"product_id\": $i,
      \"name\": \"Product $i\",
      \"category\": \"$cat\",
      \"price\": $price,
      \"stock\": $stock,
      \"active\": true
    }" > /dev/null
done
echo "       ✔  20 documents inserted."

# ─────────────────────────────────────────────────────────
# 4.  Relational  — Customers + Orders  (10/30 rows with FK)
# ─────────────────────────────────────────────────────────
echo "[5/6]  Seeding  Relational → customers  (10) + orders  (30)"
for i in $(seq 1 10); do
  name="Customer $i"
  email="customer$i@example.com"
  curl -sf -X POST "$COORDINATOR/api/v1/crud/relational/customers" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"$name\", \"email\": \"$email\", \"credit_limit\": $((5000 + RANDOM % 15000))}" \
    > /dev/null
done
for i in $(seq 1 30); do
  cid=$((1 + RANDOM % 10))
  total=$(python3 -c "import random; print(f'{random.uniform(10.0, 2000.0):.2f}')")
  status="$(echo -e "pending\nshipped\ndelivered\ncancelled" | shuf -n1)"
  curl -sf -X POST "$COORDINATOR/api/v1/crud/relational/orders" \
    -H "Content-Type: application/json" \
    -d "{
      \"customer_id\": $cid,
      \"total\": $total,
      \"status\": \"$status\",
      \"order_date\": \"2025-$(printf '%02d' $((1+RANDOM%12)))-$(printf '%02d' $((1+RANDOM%28)))\"
    }" > /dev/null
done
echo "       ✔  10 customers + 30 orders inserted."

# ─────────────────────────────────────────────────────────
# 5.  Key-Value  — Sessions + Config
# ─────────────────────────────────────────────────────────
echo "[6/6]  Seeding  Key-Value  → sessions  (5) + app_config"
curl -sf -X PUT "$COORDINATOR/api/v1/kv/sessions" -H "Content-Type: application/json" -d '{}' > /dev/null
curl -sf -X PUT "$COORDINATOR/api/v1/kv/app_config" -H "Content-Type: application/json" -d '{}' > /dev/null
for i in $(seq 1 5); do
  sid="session_$(openssl rand -hex 8)"
  uid=$((1 + RANDOM % 15))
  curl -sf -X PUT "$COORDINATOR/api/v1/kv/sessions/$sid" \
    -H "Content-Type: application/json" \
    -d "{\"user_id\": $uid, \"ip\": \"192.168.1.$((RANDOM%255))\", \"created_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" \
    > /dev/null
done
curl -sf -X PUT "$COORDINATOR/api/v1/kv/app_config/global" \
  -H "Content-Type: application/json" \
  -d '{"max_users": 10000, "features": {"dark_mode": true, "beta": false}, "maintenance_mode": false}' \
  > /dev/null
echo "       ✔  5 sessions + 1 config inserted."

echo ""
echo "╔══════════════════════════════════════════════════════╗"
echo "║  Demo data seeded successfully!                     ║"
echo "║                                                     ║"
echo "║  Try it:                                            ║"
echo "║   curl localhost:8080/api/v1/crud/columnar/sales    ║"
echo "║   curl localhost:8080/api/v1/crud/document/users    ║"
echo "║   curl localhost:8080/api/v1/crud/relational/orders ║"
echo "║   curl localhost:8080/api/v1/kv/sessions/_all_docs  ║"
echo "║   curl localhost:8080/api/v1/cluster/status         ║"
echo "║   curl localhost:8080/api/v1/federation/status      ║"
echo "╚══════════════════════════════════════════════════════╝"
