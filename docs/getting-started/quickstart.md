# PrimusDB Onboarding Guide

This guide provides a structured path for new PrimusDB users.

## Table of Contents

1. Introduction
2. Getting the Code
3. Compilation
4. Your First Instance
5. Basic Operations
6. Language Drivers
7. Next Steps

## 1. Introduction

Welcome to PrimusDB. This guide will take you from installation to your first application.

**Important note:** Everything must be compiled from source code. There are no pre-compiled binaries or published packages.

## 2. Getting the Code

### Prerequisites

```bash
# Rust 1.70+
rustc --version

# Git
git --version

# Build tools
# Ubuntu/Debian:
sudo apt-get install build-essential pkg-config libssl-dev

# Arch Linux:
sudo pacman -S base-devel openssl
```

### Clone Repository

```bash
git clone https://github.com/devahil/primusdb.git
cd primusdb
```

## 3. Compilation

### Compile the Unified Binary

```bash
cargo build --release

# Verify
./target/release/primusdb --version
```

## 4. Your First Instance

### Start Server

```bash
./target/release/primusdb server start --bind 127.0.0.1:8080
```

### Verify Operation

```bash
curl http://localhost:8080/health
```

## 5. Basic Operations

### Create Collection

```bash
curl -X POST http://localhost:8080/api/v1/table/document/users \
  -H "Content-Type: application/json" \
  -d '{"operation": "create"}'
```

### Insert Data

```bash
curl -X POST http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"data": {"name": "Alice", "email": "alice@example.com"}}'
```

### Query Data

```bash
curl http://localhost:8080/api/v1/crud/document/users
```

### Update Data

```bash
curl -X PUT http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"conditions": {"name": "Alice"}, "data": {"age": 31}}'
```

### Delete Data

```bash
curl -X DELETE http://localhost:8080/api/v1/crud/document/users \
  -H "Content-Type: application/json" \
  -d '{"conditions": {}}'
```

## 6. Language Drivers

**IMPORTANT:** No driver is published. All must be compiled locally.

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

### Rust

```bash
# Included in main compilation
cd drivers/rust
cargo build
```

## 7. Next Steps

| For | See |
|-----|-----|
| Application Developers | USER.md |
| API Documentation | API_REFERENCE.md |
| DevOps | ADMIN.md, DEPLOYMENT.md |
| Architects | ARCHITECTURE.md, EXPLANATION.md |

## Summary

You have completed the onboarding guide. Now you can:

- [x] Clone and compile PrimusDB
- [x] Start an instance
- [x] Perform CRUD operations
- [x] Compile drivers locally

Thank you for choosing PrimusDB.
