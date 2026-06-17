# PrimusDB 3-Node Cluster (Docker Compose)

This example runs a 3-node PrimusDB cluster using Docker Compose.

## Prerequisites

- Docker & Docker Compose

## Usage

```bash
# Start the cluster
docker compose up -d

# Check health
curl http://localhost:8081/health
curl http://localhost:8082/health
curl http://localhost:8083/health

# View logs
docker compose logs -f

# Stop the cluster
docker compose down
```

## Architecture

- `node1` on port `8081`
- `node2` on port `8082`
- `node3` on port `8083`

Each node stores data in a named Docker volume.
