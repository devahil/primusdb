# Query Execution Guide

PrimusDB provides multiple ways to execute SQL queries through the CLI. The query engine supports cross-engine queries — joining tables from different storage engines (relational, document, columnar, vector) in a single statement.

---

## Running Queries

### `primusdb query`

Executes a single SQL query against the connected server.

```bash
primusdb query "SELECT 1"
primusdb query "SELECT * FROM users LIMIT 10"
primusdb query "INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')"
primusdb query "DELETE FROM users WHERE id = 5"
```

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --database <NAME>` | Target database name |

**Target a specific database:**
```bash
primusdb query "SELECT * FROM orders" --database mydb
```

### `primusdb sql`

Executes SQL from a string or file.

```bash
primusdb sql "SELECT * FROM users"
primusdb sql "CREATE TABLE users (id INT PRIMARY KEY, name TEXT)"
primusdb sql -f migrations/001_create_users.sql
primusdb sql "SELECT * FROM logs" --database analytics
```

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --database <NAME>` | Target database name |
| `-f, --file <PATH>` | Read SQL from file |

### `primusdb explain`

Shows the query execution plan without running the query.

```bash
primusdb explain "SELECT * FROM users WHERE age > 30"
primusdb explain "SELECT u.name, COUNT(o.id) FROM users u JOIN orders o ON u.id = o.user_id GROUP BY u.name"
```

Output is a human-readable plan with execution stages, engine routing, and estimated row counts.

---

## SQL Support

PrimusDB's SQL layer is built on `sqlparser-rs` (v0.51) and supports the following statements and clauses.

### Data Manipulation Language (DML)

```sql
SELECT * FROM users WHERE age > 30
INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')
UPDATE users SET age = 31 WHERE id = 1
DELETE FROM users WHERE age < 18
INSERT INTO users (name) VALUES ('X') RETURNING id
UPDATE users SET age = 30 RETURNING id, name
DELETE FROM users WHERE id = 5 RETURNING *
```

### Data Definition Language (DDL)

```sql
CREATE TABLE users (id INT PRIMARY KEY, name TEXT)
ALTER TABLE users ADD COLUMN email VARCHAR(255)
DROP TABLE users
TRUNCATE users CASCADE
CREATE SEQUENCE user_id_seq START 1000
DROP SEQUENCE user_id_seq
CREATE VIEW active_users AS SELECT * FROM users WHERE active = true
DROP VIEW active_users
CREATE TRIGGER log_insert AFTER INSERT ON users ...
DROP TRIGGER log_insert
```

### Transaction Control

```sql
BEGIN
COMMIT
ROLLBACK
```

### Supported Clauses

| Clause | Support |
|--------|---------|
| `WHERE` | `=`, `<>`, `<`, `>`, `IN`, `LIKE`, `BETWEEN`, `IS NULL`, `AND`/`OR`/`NOT` |
| `ORDER BY` | Ascending / descending, multiple columns |
| `LIMIT` / `OFFSET` | Standard SQL syntax |
| `GROUP BY` | Single and multi-column grouping |
| `HAVING` | Post-aggregation filtering |
| `JOIN` | `INNER`, `LEFT`, `RIGHT`, `CROSS` (hash and nested-loop) |
| `DISTINCT` | `SELECT DISTINCT ...` |
| `UNION` / `INTERSECT` / `EXCEPT` | Basic set operations |

### Aggregate Functions

`COUNT`, `SUM`, `AVG`, `MIN`, `MAX` — usable in `SELECT` and `HAVING`.

---

## Output Formats

Queries support all global output formats:

```bash
primusdb query "SELECT id, name FROM users" --format table
primusdb query "SELECT id, name FROM users" --format json
primusdb query "SELECT id, name FROM users" --format csv
primusdb query "SELECT id, name FROM users" --format yaml
primusdb query "SELECT id, name FROM users" --format plain
```

### Table (Default)

```
 id | name
----+-------
  1 | Alice
  2 | Bob
  3 | Carol
```

### JSON

```json
[
  {"id": 1, "name": "Alice"},
  {"id": 2, "name": "Bob"},
  {"id": 3, "name": "Carol"}
]
```

### CSV

```
id,name
1,Alice
2,Bob
3,Carol
```

---

## Connection Handling

Queries are sent to the server specified by `--server-url` (default: `http://localhost:8080`). The CLI connects via HTTP POST to `/api/v1/query`.

### Connection Check Before Query

If the server is unreachable:

```bash
$ primusdb query "SELECT 1"
Error: Connection failed: error trying to connect: tcp connect error: connection refused (os error 111)
```

Check server status first:

```bash
primusdb health
primusdb server status
```

---

## Error Handling

### Syntax Errors

```bash
$ primusdb query "SELCT * FORM users"
Error: Query error: ParserError("Expected SELECT, got SELCT")
```

### Table Not Found

```bash
$ primusdb query "SELECT * FROM nonexistent"
Error: Query error: Table 'nonexistent' not found
```

### Connection Errors

```bash
$ primusdb query "SELECT 1"
Error: Connection failed: error trying to connect: tcp connect error: connection refused (os error 111)
```

### Timeout

```bash
$ primusdb --timeout 5000 query "SELECT * FROM big_table"
Error: Timeout after 5000ms
```

---

## Examples

### Basic CRUD Workflow

```bash
# Create a table
primusdb sql "CREATE TABLE users (id INT PRIMARY KEY, name TEXT, email TEXT, age INT)"

# Insert data
primusdb query "INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 30)"
primusdb query "INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 25)"
primusdb query "INSERT INTO users VALUES (3, 'Carol', 'carol@example.com', 35)"

# Query with filter
primusdb query "SELECT name, email FROM users WHERE age > 28 ORDER BY name"

# Update
primusdb query "UPDATE users SET age = 31 WHERE name = 'Alice'"

# Delete
primusdb query "DELETE FROM users WHERE age < 18"
```

### Joining Across Engines

```bash
# Create a document collection for user profiles
primusdb db create profiles --engine document

# Create a relational orders table
primusdb db create orders_db --engine relational

# Query across engines
primusdb query "SELECT u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id WHERE o.total > 100"
```

### Using `sql` with File Input

```bash
# Create a migration file
cat > migrate.sql << 'EOF'
CREATE TABLE users (id INT PRIMARY KEY, name TEXT);
CREATE TABLE orders (id INT PRIMARY KEY, user_id INT, total DECIMAL);
ALTER TABLE orders ADD FOREIGN KEY (user_id) REFERENCES users(id);
EOF

# Execute the migration
primusdb sql -f migrate.sql
```

### Explaining a Query Plan

```bash
$ primusdb explain "SELECT u.name, COUNT(o.id) FROM users u LEFT JOIN orders o ON u.id = o.user_id GROUP BY u.name HAVING COUNT(o.id) > 5"

Query Plan:
  Stage 1: Scan users (relational) → 1000 rows
  Stage 2: Scan orders (relational) → 5000 rows
  Stage 3: HashJoin (LEFT, on user_id) → 1000 rows
  Stage 4: GroupBy (name, COUNT(id)) → 200 rows
  Stage 5: Having (COUNT(id) > 5) → 50 rows
  Stage 6: Project (name, COUNT(id)) → 50 rows
```

---

## Alpha Limitations

- **No subqueries** — `SELECT * FROM (SELECT ...) AS sub` is not supported. Use CTEs or sequential queries.
- **No window functions** — `ROW_NUMBER()`, `RANK()`, etc. are not available.
- **No `EXPLAIN ANALYZE`** — `explain` shows the plan but does not execute or collect runtime statistics.
- **No cost-based optimization** — the planner uses heuristic rules only.
- **DDL on non-relational engines** — `ALTER TABLE` is only fully tested on the relational engine.
- **Prepared statements** — not yet supported in the SQL parser (driver protocol supports parameterised queries).
- **`NULL` ordering** — `ORDER BY` sorts `NULLS LAST` by default; `NULLS FIRST` / `NULLS LAST` clauses are not parsed.
