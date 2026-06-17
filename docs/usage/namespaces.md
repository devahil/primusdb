# Namespace Usage Guide

Namespaces provide hierarchical multi-tenancy in PrimusDB. They form a tree structure where each node is a labelled path segment (e.g., `tenant1.project2.analytics`), and databases, tables, users, and policies are scoped to a specific namespace.

---

## What Namespaces Are

A namespace path is a dot-separated string of components:

```
root
├── tenant1
│   ├── project2
│   │   ├── analytics
│   │   └── appdb
│   └── project3
├── tenant2
│   └── staging
└── system
```

Each component must match `^[a-zA-Z_][a-zA-Z0-9_]{0,63}$` and the full path is limited to 1024 characters.

When a database or table is created under a namespace, its physical name is derived from a SHA-256 hash of the namespace path, preventing name collisions between tenants:

```
ns_a1b2c3__users
```

---

## CLI Commands

### `primusdb namespace list`

List namespaces, optionally filtered by parent:

```bash
primusdb namespace list
primusdb namespace list --parent tenant1
primusdb namespace list --full-paths
```

**Output example (table):**
```
 Namespace
-----------
 tenant1
 tenant1/project2
 tenant1/project3
 tenant2
```

### `primusdb namespace create`

Create a new namespace:

```bash
primusdb namespace create tenant1 --description "Tenant 1"
primusdb namespace create tenant1/project2 --parent tenant1 --quota storage=1GB
```

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --description <TEXT>` | Namespace description |
| `-p, --parent <PATH>` | Parent namespace path |
| `--quota <STRING>` | Resource quota (e.g. `storage=1GB`) |

### `primusdb namespace describe`

Show namespace metadata, including policies and attached resources:

```bash
primusdb namespace describe tenant1
primusdb namespace describe tenant1 --resources
```

**Output example:**
```
Namespace: tenant1
Description: Tenant 1
Path: tenant1
Policies:
  max_databases: 10
  max_storage_mb: 1024
Resources:
  mydb (database, document)
  analytics (database, columnar)
```

### `primusdb namespace drop`

Remove a namespace:

```bash
primusdb namespace drop tenant1/unused --force
primusdb namespace drop tenant1 --recursive --force
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-r, --recursive` | Recursively drop child namespaces | `false` |
| `-f, --force` | Skip confirmation prompt | `false` |

### `primusdb namespace policy`

View or modify a namespace's policy (quotas and limits):

```bash
primusdb namespace policy tenant1 --list
primusdb namespace policy tenant1 --set max_databases=20
primusdb namespace policy tenant1 --unset max_databases
```

**Options:**

| Option | Description |
|--------|-------------|
| `-l, --list` | List current policy |
| `-s, --set <KEY=VALUE>` | Set a policy value |
| `-u, --unset <KEY>` | Unset a policy value |

**Available policy keys:**

| Key | Example Value | Description |
|-----|---------------|-------------|
| `max_databases` | `10` | Maximum databases in namespace |
| `max_storage_mb` | `1024` | Maximum storage (MB) |
| `max_connections` | `50` | Concurrent connection limit |
| `allowed_engines` | `columnar,vector` | Comma-separated engine whitelist |

Policies are inherited from parent namespaces. Explicit settings override inherited defaults.

---

## Use Cases

### Multi-Team

```bash
# Create team namespaces
primusdb namespace create team-alpha --description "Alpha team"
primusdb namespace create team-beta --description "Beta team"

# Create databases for each team under their namespace
primusdb db create app-db --namespace team-alpha
primusdb db create app-db --namespace team-beta

# Set quotas per team
primusdb namespace policy team-alpha --set max_databases=5
primusdb namespace policy team-alpha --set max_storage_mb=512
```

### Multi-Environment

```bash
# Create environment hierarchy
primusdb namespace create myapp
primusdb namespace create myapp/dev --parent myapp --quota storage=500MB
primusdb namespace create myapp/staging --parent myapp --quota storage=2GB
primusdb namespace create myapp/prod --parent myapp --quota storage=10GB

# Create databases per environment
primusdb db create myapp-db --namespace myapp/dev
primusdb db create myapp-db --namespace myapp/staging
primusdb db create myapp-db --namespace myapp/prod
```

### Data Isolation

Data isolation is enforced at the storage engine layer. A `NamespacedStorageEngine` wrapper transparently prefixes every table operation with the namespace hash:

```bash
# These databases have physically distinct table names
primusdb db create metrics --namespace team-alpha
primusdb db create metrics --namespace team-beta
```

Tables in `team-alpha` and `team-beta` share the same logical name but are stored under different hashed prefixes, preventing any cross-tenant access.

---

## Alpha Limitations

- **Resource quota enforcement is partial** — some limits are validated, others are not yet wired to the storage layer.
- **No per-namespace storage accounting** — disk usage per tenant is not tracked.
- **Namespace deletion does not cascade** to physical table cleanup in all engines.
- **Cross-namespace queries** are disabled by default. The `allow_cross_namespace_queries` config flag exists but is not fully gated in all code paths.
- **Maximum namespace depth** is configurable (`namespaces.max_depth` in config), default is 5 levels.
