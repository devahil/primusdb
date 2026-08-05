# RAG & Analytics (v1.3.2-alpha)

## Overview

PrimusDB provides retrieval-augmented generation (RAG) search and analytics capabilities through a dedicated AI/ML engine (`primusdb-ai` crate), a notebook interface for multi-cell execution, and a report builder for structured data analysis.

## Architecture

```
+-------------------------------------------------------+
|                    User Interface                      |
|  CLI / REST API                                       |
+-------------------------------------------------------+
                          |
                          v
+-------------------------------------------------------+
|                 primusdb-api (axum)                    |
|  /api/v1/rag/search  /api/v1/notebooks  /api/v1/reports|
+-------------------------------------------------------+
                          |
                          v
+-------------------------------------------------------+
|                  primusdb-ai crate                     |
|  +-----------+  +----------+  +-------------------+   |
|  | Embedding |  | Vector   |  | Analytics Engine  |   |
|  | Service   |  | Search   |  | (Statistical,     |   |
|  |           |  | (ANN)    |  |  Predictive)      |   |
|  +-----------+  +----------+  +-------------------+   |
+-------------------------------------------------------+
                          |
                          v
+-------------------------------------------------------+
|            Storage Engines (vector store)              |
+-------------------------------------------------------+
```

## RAG Search

### Endpoint: `GET /api/v1/rag/search`

Performs similarity search against a named vector collection.

**Parameters:**

| Parameter    | Type   | Required | Default | Description                         |
|-------------|--------|----------|---------|-------------------------------------|
| `collection` | string | Yes      | —       | Vector collection name              |
| `query`      | string | Yes      | —       | Natural language query text         |
| `limit`      | int    | No       | 10      | Max results to return               |
| `threshold`  | float  | No       | 0.0     | Minimum similarity score filter     |

**Response:**

```json
{
    "success": true,
    "data": {
        "query": "sample query",
        "results": [
            {
                "id": "...",
                "score": 0.92,
                "metadata": { "source": "doc1", "timestamp": "..." },
                "content": "..."
            }
        ],
        "total_results": 5
    }
}
```

### Use Cases

- Semantic document search
- Knowledge base retrieval
- Context augmentation for LLM prompts
- Similarity-based data exploration

## Notebook

### Endpoint: `GET/POST /api/v1/notebooks`

Multi-cell notebook supporting four cell types:

| Cell Type  | Description                         | Execution                     |
|------------|-------------------------------------|-------------------------------|
| SQL        | Execute SQL queries                 | Against relational engine     |
| Analysis   | Run statistical analysis            | AI engine analytics           |
| RAG        | Retrieval-augmented generation      | Vector search + AI engine     |
| Markdown   | Documentation and notes             | Rendered inline               |

### Endpoint: `POST /api/v1/notebooks/:id/execute`

Executes a specific cell by index.

**Request body:**

```json
{
    "cell_index": 0
}
```

**Response:**

```json
{
    "success": true,
    "data": {
        "cell_index": 0,
        "cell_type": "sql",
        "output": {
            "columns": ["id", "name", "value"],
            "rows": [...],
            "row_count": 42,
            "execution_time_ms": 15
        }
    }
}
```

### Cell Lifecycle

```
     Created
        |
        v
     Edited (content/cell type changes)
        |
        v
     Executed (POST /execute)
        |
        +--> SQL: query result set
        +--> Analysis: computed statistics / predictions
        +--> RAG: ranked similarity results
        +--> Markdown: rendered doc
        |
        v
     Deleted (DELETE notebook)
```

## Report Builder

### Endpoint: `GET/POST /api/v1/reports`

Define structured reports with a storage type, table, query, and output format.

**Report definition:**

```json
{
    "name": "Daily Sales Summary",
    "storage_type": "relational",
    "table": "orders",
    "query": "SELECT date, COUNT(*) as orders, SUM(total) as revenue FROM orders GROUP BY date ORDER BY date",
    "format": "table"
}
```

**Supported formats:** `table`, `json`, `csv`

### Endpoint: `POST /api/v1/reports/:id/execute`

Execute a report and retrieve results.

**Response:**

```json
{
    "success": true,
    "data": {
        "report_id": "uuid",
        "name": "Daily Sales Summary",
        "executed_at": "2026-06-27T12:00:00Z",
        "format": "table",
        "results": {
            "columns": ["date", "orders", "revenue"],
            "rows": [...],
            "row_count": 30
        },
        "execution_time_ms": 45
    }
}
```

## AI Engine (`primusdb-ai`)

The `primusdb-ai` crate provides:

- **EmbeddingService** — Generate vector embeddings for text
- **VectorSearch** — Approximate nearest neighbor (ANN) search
- **AnalyticsEngine** — Statistical and predictive analysis on table data

## Configuration

RAG and analytics settings are stored in the system database via `ConfigStore`:

| Key                          | Default  | Description                         |
|------------------------------|----------|-------------------------------------|
| `rag.default_limit`          | `10`     | Default RAG search result limit     |
| `rag.min_score_threshold`    | `0.0`    | Minimum similarity threshold         |
| `notebooks.cell_result_max`  | `1000`   | Max rows returned per cell execution |
| `reports.default_format`     | `table`  | Default report output format         |

## Limitations

- RAG search requires a vector collection to be pre-populated via the vector storage engine
- The analytics engine is experimental in v1.3.2-alpha
- Notebook SQL cells only support SELECT queries
- Report queries must be valid for the target storage engine's SQL dialect
