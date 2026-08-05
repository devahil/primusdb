# Running PrimusDB with Docker

PrimusDB ships with an Arch Linux-based Dockerfile for containerised deployments.

## Dockerfile Reference

The `Dockerfile` at the project root performs these steps:

1. Starts from `archlinux:latest`
2. Installs build dependencies (Rust toolchain, compilers, libraries)
3. Creates a `primusdb` user
4. Copies the source code
5. Builds the project with `cargo build --release`
6. Installs the unified `primusdb` binary to `/usr/local/bin/`
7. Generates a default configuration at `/etc/primusdb/primusdb.toml`
8. Installs helper scripts (`primusdb-init`, `primusdb-health`, `primusdb-backup`)
9. Exposes ports `8080` (API) and `9090` (metrics)
10. Sets the entrypoint and default command

## Building the Docker Image

```bash
# Build from the project root
docker build -t primusdb:latest .

# Build with a specific tag
docker build -t primusdb:1.3.2-alpha .

# Build without cache (for a clean build)
docker build --no-cache -t primusdb:latest .
```

> **Note:** The initial build compiles all Rust code from source and can take 10-30 minutes depending on your machine.

## Running a Container

### Basic Usage

```bash
docker run -d \
  --name primusdb \
  -p 8080:8080 \
  primusdb:latest
```

This starts the server in the background, listening on port 8080.

```bash
# Verify it's running
curl http://localhost:8080/health
```

### Custom Host and Port

```bash
docker run -d \
  --name primusdb \
  -p 9090:8080 \
  primusdb:latest
```

Maps host port 9090 to container port 8080.

### With Persistent Volumes

```bash
docker run -d \
  --name primusdb \
  -p 8080:8080 \
  -v primusdb_data:/var/lib/primusdb \
  primusdb:latest
```

This persists the data directory across container restarts. Create the volume first if needed:

```bash
docker volume create primusdb_data
```

### With a Custom Config File

```bash
docker run -d \
  --name primusdb \
  -p 8080:8080 \
  -v /host/path/primusdb.toml:/etc/primusdb/primusdb.toml:ro \
  -v primusdb_data:/var/lib/primusdb \
  primusdb:latest
```

### With Environment Variables

```bash
docker run -d \
  --name primusdb \
  -p 8080:8080 \
  -e RUST_LOG=debug \
  -e PRIMUSDB_DATA_DIR=/var/lib/primusdb \
  -v primusdb_data:/var/lib/primusdb \
  primusdb:latest
```

## Docker Compose

A Docker Compose file is provided at `examples/docker/docker-compose.yml`:

```yaml
version: "3.8"

services:
  primusdb:
    build:
      context: ../..
      dockerfile: Dockerfile
    image: primusdb:latest
    ports:
      - "8080:8080"
    volumes:
      - primusdb_data:/var/lib/primusdb
      - ./primusdb.toml:/etc/primusdb/config.toml
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:8080/health || exit 1"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s

  prometheus:
    image: prom/prometheus:latest
    profiles:
      - monitoring
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    command:
      - "--config.file=/etc/prometheus/prometheus.yml"

volumes:
  primusdb_data:
```

Run with:

```bash
cd examples/docker
docker compose up -d

# With Prometheus monitoring
docker compose --profile monitoring up -d

# View logs
docker compose logs -f primusdb

# Stop
docker compose down
```

## Persistent Volumes

The following directories in the container should be persisted for production use:

| Container Path | Purpose | Recommended Volume |
|----------------|---------|-------------------|
| `/var/lib/primusdb` | Database data files | Named volume or bind mount |
| `/etc/primusdb` | Configuration files | Bind mount (read-only in production) |

```bash
# Named volume
docker volume create primusdb_data
docker run -v primusdb_data:/var/lib/primusdb [...] primusdb:latest

# Bind mount
docker run -v /mnt/ssd/primusdb-data:/var/lib/primusdb [...] primusdb:latest
```

## Health Check Configuration

The Dockerfile includes a built-in `HEALTHCHECK` instruction:

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD primusdb-health 127.0.0.1 8080
```

The `primusdb-health` script runs `curl` against the `/health` endpoint. To use a custom health check in Docker Compose:

```yaml
healthcheck:
  test: ["CMD-SHELL", "curl -sf http://localhost:8080/health || exit 1"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 10s
```

## Environment Variables Reference

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level (trace/debug/info/warn/error) |
| `PRIMUSDB_DATA_DIR` | `/var/lib/primusdb` | Data directory inside the container |

## Helper Scripts (inside the container)

| Script | Purpose |
|--------|---------|
| `primusdb-init` | Initialise data directories and set permissions |
| `primusdb-health` | Run a health check against the local server |
| `primusdb-backup` | Create a compressed backup of the data directory |

## Troubleshooting Docker

### Container exits immediately

Check the logs:

```bash
docker logs primusdb
```

### Port already in use

Change the host-side port mapping:

```bash
docker run -p 8081:8080 primusdb:latest
```

### Build takes very long

The first build compiles all Rust dependencies and the PrimusDB source. Subsequent builds reuse Docker's layer cache. Use `--no-cache` only when you need a clean rebuild.

### Permission errors on mounted volumes

Ensure the host directory is writable. The container runs as the `primusdb` user (UID may vary):

```bash
chmod 755 /host/path/primusdb-data
```
