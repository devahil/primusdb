# Driver Usage Guide

PrimusDB provides language drivers for Rust, Python, Node.js, Java, and Ruby. As of v1.3.1-alpha, **no driver is published to a package registry** — all must be compiled from source.

All drivers communicate with a running PrimusDB server via its HTTP REST API. The native Rust driver additionally provides an in-process embedded mode.

---

## Rust Driver

### Location

```
drivers/rust/
```

### Building

The Rust driver is a workspace member. Build it from the project root:

```bash
cargo build --release -p primusdb-rust-driver
```

Or build the entire workspace:

```bash
cargo build --release --workspace
```

### Add to Your Project

To use the driver in your own Rust project, reference it by path:

```toml
[dependencies]
primusdb-rust-driver = { path = "/path/to/primusdb/drivers/rust" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Connection

For **embedded/in-process** mode, configure and create a `NativeDriver` directly:

```rust
use primusdb_rust_driver::{NativeDriver, PrimusDBConfig, StorageType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PrimusDBConfig::default();
    let driver = NativeDriver::new(config)?;
    // driver is now connected to an embedded PrimusDB instance
    Ok(())
}
```

For **remote** mode, use the HTTP client (via reqwest):

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let resp = client
        .post("http://localhost:8080/api/v1/query")
        .json(&json!({"query": "SELECT * FROM users"}))
        .send()
        .await?;
    println!("{}", resp.text().await?);
    Ok(())
}
```

### Basic CRUD

```rust
use primusdb_rust_driver::{NativeDriver, PrimusDBConfig, StorageType};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let driver = Native::new(PrimusDBConfig::default())?;

    // Create table
    driver.create_table(
        StorageType::Document,
        "users",
        json!({"name": "string", "email": "string"})
    ).await?;

    // Insert
    driver.insert(
        StorageType::Document,
        "users",
        json!({"name": "Alice", "email": "alice@example.com"})
    ).await?;

    // Select
    let results = driver.select(
        StorageType::Document,
        "users",
        Some(json!({"name": "Alice"})),
        Some(10),
        Some(0)
    ).await?;

    // Update
    driver.update(
        StorageType::Document,
        "users",
        Some(json!({"name": "Alice"})),
        json!({"age": 31})
    ).await?;

    // Delete
    driver.delete(
        StorageType::Document,
        "users",
        Some(json!({"name": "Alice"}))
    ).await?;

    Ok(())
}
```

### Transactions

```rust
driver.transaction_scope(async {
    driver.insert(StorageType::Document, "accounts", account_data).await?;
    driver.insert(StorageType::Relational, "ledger", ledger_entry).await?;
    Ok(())
}).await;
```

---

## Python Driver

### Location

```
drivers/python/
```

### Building

```bash
cd drivers/python
pip install setuptools-rust aiohttp pydantic typing-extensions
python setup.py build_ext --inplace
pip install -e .
```

### Connection

```python
import asyncio
import primusdb

async def main():
    driver = primusdb.Driver()
    await driver.connect("localhost", 8080)
    # ... operations ...
    await driver.close()

asyncio.run(main())
```

### Basic CRUD

```python
import asyncio
import primusdb

async def main():
    async with primusdb.Driver() as driver:
        await driver.connect("localhost", 8080)

        # Create table
        await driver.create_table(
            storage_type="document",
            table="users",
            schema='{"name": "string", "email": "string"}'
        )

        # Insert
        await driver.insert(
            storage_type="document",
            table="users",
            data='{"name": "Alice", "email": "alice@example.com"}'
        )

        # Query
        results = await driver.select(
            storage_type="document",
            table="users",
            conditions='{"name": "Alice"}'
        )
        print(results)

asyncio.run(main())
```

### REST API (alternative — no native driver needed)

```python
import requests

resp = requests.post(
    "http://localhost:8080/api/v1/query",
    json={"query": "SELECT * FROM users"}
)
print(resp.json())
```

---

## Node.js Driver

### Location

```
drivers/node/
```

### Building

```bash
cd drivers/node
npm install
npm run build
```

### Connection

```typescript
import { PrimusDB } from 'primusdb';

const db = new PrimusDB('localhost', 8080);
await db.connect();
```

### Basic CRUD

```typescript
import { PrimusDB } from 'primusdb';

async function main() {
    const db = new PrimusDB('localhost', 8080);
    await db.connect();

    // Create table
    await db.createTable('document', 'users', {
        name: 'string',
        email: 'string'
    });

    // Insert
    await db.insert('document', 'users', {
        name: 'Alice',
        email: 'alice@example.com'
    });

    // Query
    const users = await db.select('document', 'users', {
        name: 'Alice'
    });
    console.log(users);

    await db.disconnect();
}

main().catch(console.error);
```

### REST API (alternative)

```typescript
// Using fetch
const resp = await fetch('http://localhost:8080/api/v1/query', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ query: 'SELECT * FROM users' })
});
const data = await resp.json();
console.log(data);

// Using axios
import axios from 'axios';
const resp = await axios.post('http://localhost:8080/api/v1/query', {
    query: 'SELECT * FROM users'
});
console.log(resp.data);
```

---

## Java Driver

### Location

```
drivers/java/
```

### Building

```bash
cd drivers/java
mvn clean compile
mvn package  # creates JAR
```

### Connection

```java
import java.sql.*;

Class.forName("com.primusdb.jdbc.PrimusDBDriver");
String url = "jdbc:primusdb://localhost:8080/default";
Connection conn = DriverManager.getConnection(url);
```

### Basic CRUD

```java
Connection conn = DriverManager.getConnection(
    "jdbc:primusdb://localhost:8080/default"
);
Statement stmt = conn.createStatement();

// Create table
stmt.execute(
    "CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR(255), email VARCHAR(255))"
);

// Insert
PreparedStatement pstmt = conn.prepareStatement(
    "INSERT INTO users (id, name, email) VALUES (?, ?, ?)"
);
pstmt.setInt(1, 1);
pstmt.setString(2, "Alice");
pstmt.setString(3, "alice@example.com");
pstmt.executeUpdate();

// Query
ResultSet rs = stmt.executeQuery("SELECT * FROM users");
while (rs.next()) {
    System.out.println(rs.getString("name"));
}

rs.close();
stmt.close();
conn.close();
```

### REST API (alternative — no native driver needed)

```java
import java.net.http.*;
import java.net.URI;

HttpClient client = HttpClient.newHttpClient();
HttpRequest request = HttpRequest.newBuilder()
    .uri(URI.create("http://localhost:8080/api/v1/query"))
    .header("Content-Type", "application/json")
    .POST(HttpRequest.BodyPublishers.ofString(
        "{\"query\": \"SELECT * FROM users\"}"))
    .build();

HttpResponse<String> resp = client.send(request,
    HttpResponse.BodyHandlers.ofString());
System.out.println(resp.body());
```

---

## Ruby Driver

### Location

```
drivers/ruby/
```

### Building

```bash
cd drivers/ruby
gem build primusdb.gemspec
gem install ./primusdb-*.gem
```

### Connection

```ruby
require 'primusdb'

client = PrimusDB::Client.new(host: 'localhost', port: 8080)
```

### Basic CRUD

```ruby
require 'primusdb'

client = PrimusDB::Client.new(host: 'localhost', port: 8080)

# Get a collection
users = client.collection(:document, 'users')

# Insert
users.insert(name: 'Alice', email: 'alice@example.com')

# Query
results = users.find(name: 'Alice')
results.each { |u| puts u['name'] }

# Update
users.update({ name: 'Alice' }, { age: 31 })

# Delete
users.delete(name: 'Alice')
```

### REST API (alternative)

```ruby
require 'net/http'
require 'json'

uri = URI('http://localhost:8080/api/v1/query')
resp = Net::HTTP.post(uri,
    { query: 'SELECT * FROM users' }.to_json,
    'Content-Type' => 'application/json')
puts resp.body
```

---

## Connection String Formats

| Driver | Connection |
|--------|-----------|
| Rust (embedded) | `PrimusDBConfig::default()` |
| Rust (REST) | `http://localhost:8080` |
| Python | `localhost:8080` |
| Node.js | `localhost:8080` |
| Java (JDBC) | `jdbc:primusdb://localhost:8080/default` |
| Ruby | `localhost:8080` |

All REST API calls use the `/api/v1/query` endpoint with a JSON body:

```json
{"query": "SELECT * FROM users"}
```

---

## Resource Governor

All drivers support the Resource Governor API for tracking and limiting resource usage. Each driver exposes methods under the `governor*` / `governor_*` naming convention:

| Method              | Description                          |
|---------------------|--------------------------------------|
| `governorStartExecution` | Start a tracked execution        |
| `governorFinishExecution`| Finish an execution              |
| `governorCheckLimit`     | Check a resource limit          |
| `governorStatus`        | Get governor status              |
| `governorMetrics`       | Get metrics snapshot             |
| `governorListExecutions`| List active executions           |
| `governorListViolations`| List violations                  |
| `governorPolicies`      | List configured policies         |
| `governorUpdatePolicy`  | Create or update a policy        |

**Examples:**

```python
# Python
handle = await client.governor_start_execution("analytics", "sql", user="alice")
result = await client.governor_check_limit(handle["execution_id"], "max_memory_mb", 1024)
await client.governor_finish_execution(handle["execution_id"])
```

```rust
// Rust (native)
let id = driver.governor_start_execution("analytics".into(), WorkloadType::Sql, Some("alice"), None).await?;
let action = driver.governor_check_limit(id.parse()?, "max_memory_mb", 1024, None).await?;
driver.governor_finish_execution(id.parse()?).await?;
```

```typescript
// Node.js
const result = await db.governorStartExecution('analytics', 'sql', 'alice');
const check = await db.governorCheckLimit(result.execution_id, 'max_memory_mb', 1024);
await db.governorFinishExecution(result.execution_id);
```

## Alpha Limitations

- **No packages published** — drivers are not on crates.io, PyPI, npm, Maven Central, or RubyGems. All must be built from source.
- **Native Rust driver** uses in-process embedded mode — it does not support connecting to a remote server via a native protocol. Use the REST API for remote connections.
- **No prepared statements in SQL parser** — the driver protocol supports parameterized queries, but the SQL parser does not yet parse prepared statement syntax.
- **Driver documentation** is available in each driver's `README.md` under `drivers/<lang>/`.
- **Limited error handling** — drivers may return raw HTTP error responses without consistent error type wrapping.
