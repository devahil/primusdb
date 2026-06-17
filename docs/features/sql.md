# SQL Support

PrimusDB includes a SQL parser, planner, and executor built on
[`sqlparser-rs`](https://crates.io/crates/sqlparser) (v0.51).  The SQL layer
can target any storage engine, although **relational** is the primary target
for full SQL semantics.

---

## Supported Statements

### Data Manipulation

| Statement                            | Example                                              |
|--------------------------------------|------------------------------------------------------|
| `SELECT`                             | `SELECT * FROM users WHERE age > 30`                 |
| `INSERT`                             | `INSERT INTO users (name, email) VALUES ('A', 'a@b')`|
| `UPDATE`                             | `UPDATE users SET age = 31 WHERE id = 1`             |
| `DELETE`                             | `DELETE FROM users WHERE age < 18`                   |
| `INSERT ... RETURNING`               | `INSERT INTO users (name) VALUES ('X') RETURNING id` |
| `UPDATE ... RETURNING`               | `UPDATE users SET age=30 RETURNING id, name`         |
| `DELETE ... RETURNING`               | `DELETE FROM users WHERE id=5 RETURNING *`           |

### Data Definition

| Statement              | Example                                              |
|------------------------|------------------------------------------------------|
| `CREATE TABLE`         | `CREATE TABLE users (id INT PRIMARY KEY, name TEXT)` |
| `ALTER TABLE`          | `ALTER TABLE users ADD COLUMN email VARCHAR(255)`    |
| `DROP TABLE`           | `DROP TABLE users`                                   |
| `TRUNCATE`             | `TRUNCATE users CASCADE`                             |
| `CREATE SEQUENCE`      | `CREATE SEQUENCE user_id_seq START 1000`             |
| `DROP SEQUENCE`        | `DROP SEQUENCE user_id_seq`                          |
| `CREATE VIEW`          | `CREATE VIEW active_users AS SELECT * FROM users WHERE active = true` |
| `DROP VIEW`            | `DROP VIEW active_users`                             |
| `CREATE TRIGGER`       | `CREATE TRIGGER log_insert AFTER INSERT ON users ...` |
| `DROP TRIGGER`         | `DROP TRIGGER log_insert`                            |

### Transaction Control

| Statement        | Example                  |
|------------------|--------------------------|
| `BEGIN`          | `BEGIN`                  |
| `COMMIT`         | `COMMIT`                 |
| `ROLLBACK`       | `ROLLBACK`               |

---

## Supported Clauses

| Clause        | Support                         |
|---------------|---------------------------------|
| `WHERE`       | Full: `=`, `<>`, `<`, `>`, `IN`, `LIKE`, `BETWEEN`, `IS NULL`, `AND`/`OR`/`NOT` |
| `ORDER BY`    | Ascending / descending, multiple columns |
| `LIMIT` / `OFFSET` | Standard SQL syntax         |
| `GROUP BY`    | Single and multi-column grouping |
| `HAVING`      | Post-aggregation filtering    |
| `JOIN`        | `INNER`, `LEFT`, `RIGHT`, `CROSS` (hash and nested-loop) |
| `DISTINCT`    | `SELECT DISTINCT ...`         |
| `UNION` / `INTERSECT` / `EXCEPT` | Basic set operations |

### Aggregate Functions

`COUNT`, `SUM`, `AVG`, `MIN`, `MAX` — usable in `SELECT` and `HAVING`.

---

## CLI Commands

### `primusdb query`

Execute an ad-hoc query:

```bash
primusdb query "SELECT * FROM users LIMIT 10"
primusdb query "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')"
primusdb query "SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id"
```

The `query` command accepts a positional SQL string (space-separated
arguments are joined automatically).

### `primusdb sql`

Interprets the first argument as a SQL string or, with `-f`, reads from a file:

```bash
primusdb sql "CREATE TABLE users (id INT PRIMARY KEY, name TEXT)"
primusdb sql -f migrations/001_create_users.sql
primusdb sql "SELECT * FROM logs" --database analytics
```

### `primusdb explain`

Show the query plan without executing:

```bash
primusdb explain "SELECT * FROM users WHERE age > 30"
primusdb explain "SELECT u.name, COUNT(o.id) FROM users u JOIN orders o ON u.id = o.user_id GROUP BY u.name"
```

Output is a human-readable plan with execution stages, engine routing, and
estimated row counts.

---

## Architecture

The SQL pipeline has four phases:

```
SQL string
    │
    ▼
  ┌─────────────┐     sqlparser-rs
  │   Parser    │ ──► AST (Statement enum)
  └─────────────┘
    │
    ▼
  ┌─────────────┐
  │   Planner   │ ──► QueryPlan (stages, engine routing)
  └─────────────┘
    │
    ▼
  ┌─────────────┐
  │  Executor   │ ──► UqlResult (records + metadata)
  └─────────────┘
    │
    ▼
  ┌─────────────┐
  │  Formatter  │ ──► JSON / table output
  └─────────────┘
```

1. **Parser** — converts SQL text to an AST using `sqlparser-rs`.
2. **Planner** — produces an execution plan with optimised stage ordering and
   engine routing (e.g. sends JOIN to the relational engine, pushes filter
   predicates down).
3. **Executor** — walks the plan stages, calling the appropriate storage
   engines, and collects results.
4. **Formatter** — formats results as JSON, table, CSV, or YAML.

Cross-engine queries (e.g. joining a relational table with a document
collection) are supported via the UQL layer and hash-join stages.

---

## Known Limitations (Alpha)

- **No cost-based optimisation** — the planner uses heuristic rules (always
  push filters, prefer hash joins for large tables) but does not estimate
  cardinality or I/O cost.
- **No subqueries** — `SELECT * FROM (SELECT ... ) AS sub` is not yet parsed;
  use CTEs or sequential queries.
- **No window functions** — `ROW_NUMBER()`, `RANK()`, etc. are not supported.
- **No `EXPLAIN ANALYZE`** — `explain` shows the plan but does not execute or
  collect runtime statistics.
- **DDL on non-relational engines** — `CREATE TABLE` on a columnar or document
  engine works, but `ALTER TABLE` is only fully tested on the relational
  engine.
- **Prepared statements** — not yet supported in the SQL parser (the driver
  protocol does support parameterised queries via the Rust/Python/Java
  drivers).
- **`NULL` ordering** — `ORDER BY` sorts `NULLS LAST` by default; `NULLS
  FIRST` / `NULLS LAST` clauses are not parsed.
