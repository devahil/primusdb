# Welcome to PrimusDB

Welcome to PrimusDB, a high-performance hybrid database engine designed for modern applications. This guide will help you get started quickly.

## What is PrimusDB?

PrimusDB is a database engine that combines four different storage paradigms into a unified system: columnar, vector, document, and relational. This means you can use a single database for diverse workloads without the complexity of managing multiple database systems.

## Getting Started

### Get the Code

**IMPORTANT:** There are no pre-compiled binaries or published packages. You must clone the repository and compile from source code.

```bash
# Clone the repository
git clone https://github.com/devahil/primusdb.git
cd primusdb
```

### Compile

```bash
# Compile in release mode
cargo build --release

# Binaries are at:
# target/release/primusdb-server
# target/release/primusdb-cli
```

### Start the Server

```bash
# Start server
./target/release/primusdb-server --port 8080
```

### Verify It Works

```bash
# Verify health
curl http://localhost:8080/health

# Should respond:
# {"success":true,"data":{"status":"healthy",...}}
```

## Your First Operations

```bash
# Create a record
curl -X POST http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"data": {"name": "Alice", "email": "alice@example.com", "age": 30}}'

# Read records
curl http://localhost:8080/api/v1/crud/document/users

# Clean up
curl -X DELETE http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"conditions": {}}'
```

## Language Drivers

**NOTE:** Drivers are NOT published. Must be compiled locally.

### Python

```bash
cd drivers/python
pip install setuptools-rust aiohttp pydantic typing-extensions
python setup.py build_ext --inplace
pip install -e .
```

### Node.js

```bash
cd drivers/node
npm install
npm run build
```

### Java

```bash
cd drivers/java
mvn clean compile
```

### Ruby

```bash
cd drivers/ruby
gem build primusdb.gemspec
gem install ./primusdb-0.1.0.gem
```

## Documentation

This documentation suite is organized to help you at every stage:

**[README.md](README.md)** - General information and key features.

**[ONBOARDING.md](ONBOARDING.md)** - Structured guide for new users.

**[USER.md](USER.md)** - Practical guide for daily operations.

**[API_REFERENCE.md](API_REFERENCE.md)** - Complete REST API documentation.

**[ARCHITECTURE.md](ARCHITECTURE.md)** - Deep technical documentation.

**[BUILD.md](BUILD.md)** - Compilation instructions.

**[ADMIN.md](ADMIN.md)** - Administration tasks.

**[DEPLOYMENT.md](DEPLOYMENT.md)** - Production deployment guide.

**[TROUBLESHOOTING.md](TROUBLESHOOTING.md)** - Common problem solutions.

**[EXPLANATION.md](EXPLANATION.md)** - Detailed feature explanations.

## Next Steps

1. Complete the onboarding guide (ONBOARDING.md)
2. Review the user guide (USER.md)
3. Set up your development environment
4. Plan your production deployment

## Getting Help

- Check the troubleshooting guide (TROUBLESHOOTING.md)
- Document your issue in detail
- Check examples in the `drivers/` directory

## License

PrimusDB is licensed under GNU General Public License v3.0.

---

Welcome to PrimusDB!
