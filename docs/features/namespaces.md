# Namespaces

PrimusDB implements a **hierarchical namespace model** for multi-tenancy.
Namespaces form a tree where each node is a labelled path segment (e.g.
`tenant1.project2.analytics`).  Databases, tables, users, and policies are
scoped to a namespace, providing logical and (eventually) resource isolation.

---

## Concept

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

Each component must match `^[a-zA-Z_][a-zA-Z0-9_]{0,63}$` and the full path is
limited to 1024 characters.

When a database or table is created under a namespace, its physical name is
derived from a SHA-256 hash of the namespace path, preventing name collisions
between tenants:

```
ns_a1b2c3__users
```

---

## Multi-Tenancy

Namespaces are the foundation of PrimusDB's multi-tenant architecture:

- **Data isolation** — a `NamespacedStorageEngine` wrapper transparently
  prefixes every table operation with the namespace hash.  Tenants in different
  namespaces cannot see each other's tables.
- **Authentication scoping** — API tokens and user roles can be restricted to a
  subtree of the namespace tree.
- **Policy inheritance** — child namespaces inherit policies from their
  parents; explicit settings override inherited ones.

---

## Resource Isolation

> **Alpha status.**  Resource quotas and usage tracking are partially
> implemented.

The `namespace policy` command can set per-namespace quotas:

| Policy Key         | Example Value | Description                      |
|--------------------|---------------|----------------------------------|
| `max_databases`    | `10`          | Maximum databases in namespace   |
| `max_storage_mb`   | `1024`        | Maximum storage (MB)             |
| `max_connections`  | `50`          | Concurrent connection limit      |
| `allowed_engines`  | `columnar,vector` | Comma-separated engine whitelist |

Quotas are enforced at the `NamespaceController` level.  If a policy is not
set, the parent's value (or an unlimited default) applies.

---

## CLI Commands

### `primusdb namespace list`

List namespaces, optionally filtered by parent:

```bash
primusdb namespace list
primusdb namespace list --parent tenant1
primusdb namespace list --full-paths
```

### `primusdb namespace create`

Create a new namespace:

```bash
primusdb namespace create tenant1 --description "Tenant 1"
primusdb namespace create tenant1/project2 --parent tenant1 --quota storage=1GB
```

### `primusdb namespace drop`

Remove a namespace.  `--recursive` drops child namespaces; `--force` skips
confirmation.

```bash
primusdb namespace drop tenant1/unused --force
primusdb namespace drop tenant1 --recursive --force
```

### `primusdb namespace describe`

Show namespace metadata, including policies and (with `--resources`) the
databases attached to it:

```bash
primusdb namespace describe tenant1
primusdb namespace describe tenant1 --resources
```

### `primusdb namespace policy`

View or modify a namespace's policy:

```bash
primusdb namespace policy tenant1 --list
primusdb namespace policy tenant1 --set max_databases=20
primusdb namespace policy tenant1 --unset max_databases
```

---

## Status

**Alpha.**  The hierarchical namespace model and data isolation via physical
name hashing are implemented and tested.  Resource quota enforcement and usage
tracking are partial — some limits are validated, others are not yet wired.

Known gaps:

- Cross-namespace queries are disabled by default but the `allow_cross_namespace_queries`
  config flag exists (not fully gated in all code paths).
- No per-namespace storage accounting (disk usage per tenant).
- Namespace deletion does not cascade to physical table cleanup in all engines.
