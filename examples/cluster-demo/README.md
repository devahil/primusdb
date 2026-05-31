# PrimusDB Cluster Demo

A complete multi-node federated cluster example with seed data covering
all PrimusDB storage engines.

```
┌─────────────────────────────────────────────────┐
│     Nginx Facade :8080 (ingress simulation)      │
└────┬──────────┬──────────┬───────────────────────┘
     │          │          │
     ▼          ▼          ▼
┌─────────┐ ┌─────────┐ ┌─────────┐   ┌──────────┐
│ Coord   │ │ Worker1 │ │ Worker2 │   │ Fed-Peer │
│ :8081   │ │ :8082   │ │ :8083   │   │ :8084    │
│ raft    │ │ raft    │ │ raft    │   │ fed      │
│ leader  │ │ worker  │ │ worker  │   │ cluster  │
└─────────┘ └─────────┘ └─────────┘   └──────────┘
                │
          (federation)
                │
         ┌──────┴──────┐
         │  Cluster B   │  (extends this example)
         │  :8085       │
         └─────────────┘
```

## Table of Contents

1. [Architecture](#1-architecture)
2. [Requirements](#2-requirements)
3. [Quick Start](#3-quick-start)
4. [Seed Data](#4-seed-data)
5. [Available APIs](#5-available-apis)
6. [Extending to Cluster-of-Clusters](#6-extending-to-cluster-of-clusters)
7. [Deploying to Swarm](#7-deploying-to-swarm)
8. [Deploying to K8s](#8-deploying-to-k8s)
9. [Facade / Ingress](#9-facade--ingress)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. Architecture

### Components

| Service      | Port  | Role                         | Storage            |
|--------------|-------|------------------------------|--------------------|
| `coordinator` | 8081 | Raft leader node             | `primusdb_coordinator` (persistent volume) |
| `worker1`     | 8082 | Raft worker node             | `primusdb_worker1` |
| `worker2`     | 8083 | Raft worker node             | `primusdb_worker2` |
| `fed-peer`    | 8084 | Federation peer              | `primusdb_fedpeer` |
| `gateway`     | 8080 | Nginx facade (ingress)       | —                  |
| `seeder`      | —    | Seed data injector (one-shot)| —                  |

### Network

All containers communicate over the bridge network `primusdb-net`.
Each node's internal hostname matches its service name
(`coordinator`, `worker1`, `worker2`, `fed-peer`, `gateway`).

### Persistence

Each node has its own Docker volume for data. Use
`docker compose down -v` to erase the database; omit `-v` to preserve it.

### Exposed Ports

- `8080` → Nginx facade
- `8081` → Coordinator
- `8082` → Worker1
- `8083` → Worker2
- `8084` → Federation peer

---

## 2. Requirements

- Docker Engine ≥ 24.0
- Docker Compose ≥ 2.20 (bundled with Docker Desktop)
- Minimum 4 GB free RAM (8 GB recommended)
- Port 8080 free on the host (or change `ports:` in compose)

---

## 3. Quick Start

```bash
# 1. Build images and start
docker compose build   # ~5-10 min (first time)
docker compose up -d   # start all services

# 2. Watch the seeder (data takes ~30s to populate)
docker logs -f primusdb-seeder

# 3. Test the cluster
curl localhost:8080/health
curl localhost:8080/api/v1/cluster/status | jq
curl localhost:8080/api/v1/cluster/nodes  | jq

# 4. Explore data
curl localhost:8080/api/v1/crud/columnar/sales    | jq
curl localhost:8080/api/v1/crud/document/users    | jq
curl localhost:8080/api/v1/crud/relational/orders | jq
curl localhost:8080/api/v1/kv/sessions/_all_docs  | jq

# 5. Test federation
curl localhost:8080/api/v1/federation/status   | jq
curl localhost:8080/api/v1/federation/domains  | jq

# 6. Stop
docker compose down    # preserves data in volumes
docker compose down -v # DELETES EVERYTHING (DANGER: destroys data)
```

---

## 4. Seed Data

The `seeder` automatically runs `seed/seed.sh` and `seed/federation.sh` once
all nodes report healthy. It populates **127 records** across 6 categories:

### Columnar — Sales (`sales`)

50 sales records with analytical schema:
- `product_id`, `category`, `amount`, `quantity`, `sale_date`, `region`, `customer_id`

Good for aggregate queries and time series analysis.

### Vector — Product Embeddings (`product_embeddings`)

20 eight-dimensional vectors with metadata:
- `id`, `vector` (float[8]), `metadata.product_id`, `metadata.tags`

Good for similarity search (ANN).

### Document — Users (`users`)

15 user profiles with nested structure:
- `name`, `email`, `age`, `tier`, `created_at`, `preferences.notifications`, `preferences.theme`

### Document — Products (`products`)

20 catalog items:
- `product_id`, `name`, `category`, `price`, `stock`, `active`

### Relational — Customers + Orders

**`customers`** (10 rows): `name`, `email`, `credit_limit`

**`orders`** (30 rows): `customer_id` (FK), `total`, `status`, `order_date`

1:N relationship between customers and orders.

### Key-Value — Sessions + Config

**`sessions`** (5 entries): keys `session_<hex>`, value `{user_id, ip, created_at}`

**`app_config`** (1 entry): key `global`, value `{max_users, features, maintenance_mode}`

---

## 5. Available APIs

All routes go through the facade at `localhost:8080`.
Individual nodes are also directly accessible on their own ports.

### Health and Monitoring

```bash
curl localhost:8080/health
curl localhost:8080/status
curl localhost:8080/metrics          # Prometheus-format metrics
```

### Cluster

```bash
curl localhost:8080/api/v1/cluster/status
curl localhost:8080/api/v1/cluster/nodes
curl localhost:8080/api/v1/cluster/metrics
```

### CRUD by Engine

```bash
# Columnar
curl localhost:8080/api/v1/crud/columnar/sales
curl "localhost:8080/api/v1/crud/columnar/sales?category=Electronics"
curl -X POST localhost:8080/api/v1/crud/columnar/sales \
  -H "Content-Type: application/json" \
  -d '{"product_id":99, "category":"Test", "amount":10.0, "quantity":1, "sale_date":"2025-06-01", "region":"Test"}'

# Vector
curl localhost:8080/api/v1/crud/vector/product_embeddings
curl -X POST localhost:8080/api/v1/crud/vector/product_embeddings \
  -H "Content-Type: application/json" \
  -d '{"id":"test_vec","vector":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8],"metadata":{"tags":["test"]}}'

# Document
curl localhost:8080/api/v1/crud/document/users
curl -X POST localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Test User","email":"test@example.com","age":30,"tier":"basic"}'

# Relational
curl localhost:8080/api/v1/crud/relational/customers
curl localhost:8080/api/v1/crud/relational/orders
curl "localhost:8080/api/v1/crud/relational/orders?customer_id=1"

# Key-Value
curl localhost:8080/api/v1/kv/app_config/global
curl localhost:8080/api/v1/kv/sessions/_all_docs
curl -X PUT localhost:8080/api/v1/kv/app_config/global \
  -H "Content-Type: application/json" \
  -d '{"maintenance_mode":true}'
```

### Federation

```bash
curl localhost:8080/api/v1/federation/status
curl localhost:8080/api/v1/federation/clusters
curl localhost:8080/api/v1/federation/domains
curl -X POST localhost:8080/api/v1/federation/domains \
  -H "Content-Type: application/json" \
  -d '{"name":"test-domain","replication_mode":"Async","storage_types":["document"],"collections":["users"]}'
```

### Namespaces

```bash
curl localhost:8080/api/v1/namespace/list
curl -X POST localhost:8080/api/v1/namespace/create \
  -H "Content-Type: application/json" \
  -d '{"name":"myapp"}'
```

### UQL (PrimusDB Query Language)

```bash
curl -X POST localhost:8080/api/v1/uql \
  -H "Content-Type: application/json" \
  -d '{"query": "FROM sales SELECT category, SUM(amount) GROUP BY category ORDER BY SUM(amount) DESC"}'

curl -X POST localhost:8080/api/v1/uql \
  -H "Content-Type: application/json" \
  -d '{"query": "FROM users SELECT * WHERE tier = \"premium\""}'
```

### Transactions

```bash
curl -X POST localhost:8080/api/v1/transaction/begin
# → {"transaction_id": "tx_abc123"}

curl -X POST localhost:8080/api/v1/transaction/tx_abc123/commit
curl -X POST localhost:8080/api/v1/transaction/tx_abc123/rollback
```

---

## 6. Extending to Cluster-of-Clusters

The example includes a federation peer (`fed-peer`) ready to connect
additional clusters. Follow these steps to scale:

### 6.1 Add a Second Cluster (Cluster B)

```yaml
# docker-compose.override.yml
services:
  coordinator-b:
    build: .
    hostname: coordinator-b
    ports: ["8085:8085"]
    command: ["--config", "/etc/primusdb/primusdb-b.toml"]
    networks: [primusdb-net]

  worker-b1:
    build: .
    hostname: worker-b1
    ports: ["8086:8086"]
    command: ["--config", "/etc/primusdb/primusdb-b1.toml"]
    networks: [primusdb-net]
```

Create `config/coordinator-b.toml` with:

```toml
[network]
port = 8085

[cluster]
node_id = "coord-b"
discovery_servers = ["worker-b1:8086"]

[federation]
enabled = true
federation_id = "global-fed"
cluster_id = "cluster-b"
region = "eu-west-1"
discovery = ["fed-peer:8084", "coordinator-b:8085"]
```

### 6.2 The Peer Connects Both Clusters

Once `coordinator-b` joins the federation, both clusters share
DataDomains. You can create multi-cluster domains:

```bash
curl -X POST localhost:8080/api/v1/federation/domains \
  -H "Content-Type: application/json" \
  -d '{
    "name": "cross-region-users",
    "replication_mode": "Quorum",
    "storage_types": ["document"],
    "collections": ["users"],
    "member_clusters": ["cluster-a", "cluster-b"]
  }'
```

### 6.3 Resulting Topology

```
┌──────────┐      ┌──────────┐
│Cluster-A │◄────►│ Cluster-B│
│  us-east │  fed │  eu-west │
│ coord     │      │ coord     │
│ worker1   │      │ worker1   │
│ worker2   │      │           │
└────┬─────┘      └────┬─────┘
     │                  │
     └──────┬───────────┘
            │
     DataDomain: cross-region-users (Quorum)
```

---

## 7. Deploying to Swarm

```bash
# Initialize swarm
docker swarm init

# Label nodes (optional)
docker node update --label-add primusdb.role=coordinator node1
docker node update --label-add primusdb.role=worker node2

# Deploy stack
docker stack deploy -c docker-compose.yml primusdb

# Scale workers
docker service scale primusdb_worker1=3 primusdb_worker2=3

# Check status
docker stack ps primusdb
docker service logs primusdb_seeder
```

**Note:** For Swarm, change `ports:` to use an overlay network. The nginx.conf
uses `least_conn` which works unchanged in Swarm.

---

## 8. Deploying to K8s

Create a namespace and apply manifests:

```bash
kubectl create namespace primusdb
kubectl apply -f k8s/ -n primusdb
```

If the `k8s/` directory does not exist, generate it with:

```bash
mkdir k8s
# StatefulSet for each node
# ClusterIP and NodePort Services
# ConfigMap for nginx.conf
# Job for seeder
```

A minimal StatefulSet template:

```yaml
# k8s/coordinator-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: primusdb-coordinator
spec:
  serviceName: primusdb-coordinator
  replicas: 1
  selector:
    matchLabels:
      app: primusdb
      node: coordinator
  template:
    metadata:
      labels:
        app: primusdb
        node: coordinator
    spec:
      containers:
      - name: primusdb
        image: primusdb:latest
        args: ["--config", "/etc/primusdb/primusdb.toml"]
        ports:
        - containerPort: 8081
        volumeMounts:
        - name: config
          mountPath: /etc/primusdb
        - name: data
          mountPath: /data
      volumes:
      - name: config
        configMap:
          name: primusdb-config-coordinator
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      resources:
        requests:
          storage: 10Gi
```

---

## 9. Facade / Ingress

The `gateway` container runs Nginx and acts as the **single entry point**
(port 8080). It routes each API prefix to the appropriate node:

| Route                              | Target        |
|------------------------------------|---------------|
| `/health`, `/status`, `/metrics`   | Coordinator   |
| `/api/v1/crud/*`                  | Least-conn    |
| `/api/v1/kv/*`                    | Least-conn    |
| `/api/v1/uql`                     | Least-conn    |
| `/api/v1/query`                   | Least-conn    |
| `/api/v1/transaction/*`           | Least-conn    |
| `/api/v1/cluster/*`               | Coordinator   |
| `/api/v1/federation/*`            | Coordinator / fed-peer |
| `/api/v1/namespace/*`             | Coordinator   |

### Plug and Play with any Ingress

The facade listens on `:8080` and can be placed behind:

- **Traefik**: `traefik.http.routers.primusdb.rule=Host(\`db.example.com\`)`
- **Kong / APISIX**: upstream pointing to `gateway:8080`
- **AWS ALB**: target group to port 8080
- **Istio Gateway**: VirtualService routing to `gateway.primusdb.svc.cluster.local:8080`

---

## 10. Troubleshooting

### "Seeder stuck waiting for coordinator"

The coordinator takes ~20s to start and ~10s additional to form a Raft quorum.
Check the logs:

```bash
docker logs primusdb-coordinator
docker logs primusdb-worker1
```

If a worker cannot connect, verify that `discovery_servers` in the TOML
uses the correct hostname (not `localhost`).

### Corrupted Volume

```bash
docker compose down -v   # delete volumes
docker compose up -d     # fresh start
```

### Data Not Appearing

The seeder injects data via REST. Verify all health checks pass:

```bash
curl -sf http://localhost:8081/health && echo "coord ok"
curl -sf http://localhost:8082/health && echo "w1 ok"
```

Then check the seeder logs:

```bash
docker logs primusdb-seeder
```

### Port in Use

Change the mapping in `docker-compose.yml`:

```yaml
services:
  gateway:
    ports: ["9090:8080"]  # change the host port
```

### Rebuild from Scratch

```bash
docker compose down -v
docker compose build --no-cache
docker compose up -d
```
