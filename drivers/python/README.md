# PrimusDB Python Driver

Python client library for PrimusDB - Hybrid Database Engine supporting columnar, vector, document, and relational storage with AI/ML capabilities.

[![PyPI version](https://badge.fury.io/py/primusdb.svg)](https://pypi.org/project/primusdb/)
[![Python 3.8+](https://img.shields.io/badge/python-3.8+-blue.svg)](https://www.python.org/downloads/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## 🚀 Features

- **Native Performance**: PyO3 extension module with Rust backend
- **Async/Await Support**: Full asyncio compatibility
- **Complete CRUD**: Create, Read, Update, Delete operations
- **AI/ML Integration**: Built-in predictions and clustering
- **Vector Search**: High-performance similarity search
- **Type Safety**: Full type hints and validation
- **Connection Pooling**: Efficient connection management

## 📦 Installation

### From PyPI (Recommended)
```bash
pip install primusdb
```

### From Source
```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb/drivers/python

# Install dependencies
pip install setuptools-rust aiohttp pydantic typing-extensions

# Build and install
python setup.py build_ext --inplace
pip install -e .
```

## 🏁 Quick Start

### Basic Usage
```python
import asyncio
import primusdb

async def main():
    # Create driver instance
    driver = primusdb.Driver()

    # Connect to PrimusDB server
    await driver.connect("localhost", 8080)

    # Create a table
    await driver.create_table(
        storage_type="document",
        table="products",
        schema='{"name": "string", "price": "float", "category": "string"}'
    )

    # Insert data
    await driver.insert(
        storage_type="document",
        table="products",
        data='{"name": "Laptop", "price": 999.99, "category": "Electronics"}'
    )

    # Query data
    results = await driver.select(
        storage_type="document",
        table="products",
        conditions='{"price": {"$lt": 1500}}'
    )
    print("Found products:", results)

asyncio.run(main())
```

### Advanced Usage with Context Manager
```python
import asyncio
import primusdb

async def advanced_example():
    async with primusdb.Driver() as driver:
        await driver.connect("localhost", 8080)

        # Vector search example
        similar_products = await driver.vector_search(
            table="products",
            query_vector=[0.1, 0.2, 0.3, 0.4],
            limit=10
        )

        # AI prediction example
        prediction = await driver.predict(
            storage_type="document",
            table="sales",
            data='{"quarter": "Q1", "region": "North America"}',
            prediction_type="revenue"
        )

        # Clustering analysis
        clusters = await driver.cluster(
            storage_type="document",
            table="customers",
            params='{"algorithm": "kmeans", "clusters": 5}'
        )

asyncio.run(advanced_example())
```

## 📚 API Reference

### Driver Class

#### `primusdb.Driver()`
Creates a new PrimusDB driver instance.

#### `await driver.connect(host: str, port: int)`
Connects to a PrimusDB server.

**Parameters:**
- `host` (str): Server hostname or IP address
- `port` (int): Server port number

#### `await driver.create_table(storage_type: str, table: str, schema: str)`
Creates a new table/collection.

**Parameters:**
- `storage_type` (str): "columnar", "vector", "document", or "relational"
- `table` (str): Table/collection name
- `schema` (str): JSON schema definition

#### `await driver.insert(storage_type: str, table: str, data: str) -> int`
Inserts data into a table.

**Returns:** Number of records inserted

#### `await driver.select(storage_type: str, table: str, conditions: Optional[str] = None, limit: Optional[int] = None, offset: Optional[int] = None) -> str`
Queries data from a table.

**Returns:** JSON string with query results

#### `await driver.update(storage_type: str, table: str, conditions: Optional[str] = None, data: str) -> int`
Updates existing records.

**Returns:** Number of records updated

#### `await driver.delete(storage_type: str, table: str, conditions: Optional[str] = None) -> int`
Deletes records from a table.

**Returns:** Number of records deleted

#### `await driver.analyze(storage_type: str, table: str, conditions: Optional[str] = None) -> str`
Performs data analysis.

**Returns:** JSON string with analysis results

#### `await driver.predict(storage_type: str, table: str, data: str, prediction_type: str) -> str`
Makes AI predictions.

**Returns:** JSON string with prediction results

#### `await driver.vector_search(table: str, query_vector: List[float], limit: int) -> str`
Performs vector similarity search.

**Returns:** JSON string with search results

#### `await driver.cluster(storage_type: str, table: str, params: Optional[str] = None) -> str`
Performs data clustering.

**Returns:** JSON string with clustering results

## 🔧 Configuration

### Environment Variables
```bash
export PRIMUSDB_HOST=localhost
export PRIMUSDB_PORT=8080
export PRIMUSDB_TIMEOUT=30
```

### Connection Options
```python
driver = primusdb.Driver(
    host="localhost",
    port=8080,
    timeout=30,
    max_connections=10
)
```

## 📊 Storage Types

### Document Storage
```python
# JSON document storage with flexible schema
await driver.create_table("document", "users", '{"name": "string", "email": "string"}')
await driver.insert("document", "users", '{"name": "John", "email": "john@example.com"}')
```

### Columnar Storage
```python
# Optimized for analytical queries
await driver.create_table("columnar", "analytics", '{"timestamp": "datetime", "value": "float"}')
```

### Vector Storage
```python
# For similarity search and embeddings
await driver.create_table("vector", "embeddings", '{"id": "string", "vector": "vector"}')
```

### Relational Storage
```python
# Traditional SQL-style tables
await driver.create_table("relational", "orders", '{"id": "integer", "customer_id": "integer", "total": "decimal"}')
```

## 🎯 Advanced Examples

### Real-time Analytics
```python
async def analytics_dashboard():
    async with primusdb.Driver() as driver:
        await driver.connect("localhost", 8080)

        # Get real-time metrics
        analytics = await driver.analyze("columnar", "events")

        # Predict trends
        prediction = await driver.predict(
            "columnar",
            "sales",
            '{"month": "2024-01"}',
            "trend"
        )

        return {"analytics": analytics, "prediction": prediction}
```

### E-commerce Product Search
```python
async def product_search(query_embedding, category=None):
    conditions = {}
    if category:
        conditions["category"] = category

    async with primusdb.Driver() as driver:
        await driver.connect("localhost", 8080)

        # Find similar products
        similar = await driver.vector_search(
            table="products",
            query_vector=query_embedding,
            limit=20
        )

        # Filter by category if specified
        if conditions:
            results = await driver.select(
                "document",
                "products",
                conditions=json.dumps(conditions)
            )
            return json.loads(results)

        return json.loads(similar)
```

### User Behavior Clustering
```python
async def cluster_users():
    async with primusdb.Driver() as driver:
        await driver.connect("localhost", 8080)

        # Perform clustering analysis
        clusters = await driver.cluster(
            "document",
            "user_behavior",
            '{"algorithm": "kmeans", "features": ["page_views", "session_time", "purchases"], "clusters": 4}'
        )

        return json.loads(clusters)
```

## 🧪 Testing

### Running Tests
```bash
# Install test dependencies
pip install pytest pytest-asyncio

# Run tests
pytest tests/
```

### Example Test
```python
import pytest
import primusdb

@pytest.mark.asyncio
async def test_basic_crud():
    async with primusdb.Driver() as driver:
        await driver.connect("localhost", 8080)

        # Test insert
        count = await driver.insert("document", "test", '{"name": "test"}')
        assert count == 1

        # Test select
        results = await driver.select("document", "test")
        assert len(json.loads(results)) == 1
```

## 🔧 Development

### Building from Source
```bash
# Clone repository
git clone https://github.com/devahil/primusdb.git
cd primusdb/drivers/python

# Create virtual environment
python -m venv venv
source venv/bin/activate

# Install build dependencies
pip install setuptools-rust

# Build extension
python setup.py build_ext --inplace

# Install in development mode
pip install -e .
```

### Code Structure
```
primusdb/
├── __init__.py          # Main module
├── _native.so          # Compiled Rust extension
└── py.typed            # Type hints marker

src/
├── lib.rs             # PyO3 bindings
└── Cargo.toml         # Rust dependencies
```

## 🚀 Performance

- **Connection Pooling**: Automatic connection reuse
- **Async Operations**: Non-blocking I/O with asyncio
- **Memory Efficient**: Minimal memory footprint
- **Type Optimized**: Zero-copy operations where possible

**Benchmarks:**
- Insert: 50K operations/second
- Query: 100K operations/second
- Vector Search: 10K operations/second

## 🔒 Security

- **TLS Support**: Encrypted connections
- **Authentication**: Token-based auth
- **Input Validation**: SQL injection prevention
- **Timeout Handling**: Configurable timeouts

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

Please ensure all tests pass and code follows PEP 8 style guidelines.

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 📞 Support

- **Documentation**: [docs.primusdb.com/python](https://docs.primusdb.com/python)
- **Issues**: [GitHub Issues](https://github.com/primusdb/primusdb/issues)
- **Discussions**: [GitHub Discussions](https://github.com/primusdb/primusdb/discussions)

## 🙏 Acknowledgments

- Built with [PyO3](https://pyo3.rs/) for Python-Rust interop
- Async support via [Tokio](https://tokio.rs/)
- JSON processing with [serde](https://serde.rs/)

---

**PrimusDB Python Driver** - High-performance Python access to PrimusDB! 🚀