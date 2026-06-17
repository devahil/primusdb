# AI/ML Engine

PrimusDB has a built-in AI/ML engine for training models, making predictions,
detecting anomalies, and analysing data patterns — without external
dependencies.  The engine is implemented in `src/ai/` and exposed via the REST
API and the `primusdb ai` CLI subcommands.

> **Alpha status.**  The engine produces valid results for demonstration and
> prototyping, but the underlying models are simplified (linear regression,
> basic statistics) and are not yet competitive with dedicated ML frameworks.
> See the limitations section below.

---

## Model Types

| Model              | Kind              | Description                                      |
|--------------------|-------------------|--------------------------------------------------|
| Linear Regression  | `regression`      | Continuous value prediction (e.g. sales, price)  |
| Logistic Regression| `classification`  | Binary classification (e.g. churn, spam)         |
| Time Series        | `forecasting`     | Temporal forecasting with configurable horizon   |
| Anomaly Detection  | `anomaly`         | Statistical outlier detection (z-score, IQR, MAD, isolation forest) |
| Clustering         | `clustering`      | Unsupervised k-means clustering                  |

---

## CLI Commands

### `primusdb ai models`

List all trained models:

```bash
primusdb ai models
primusdb ai models --kind regression --verbose
```

Output includes model ID, type, accuracy, training table, and timestamps.

### `primusdb ai train`

Train a new model from a table's data:

```bash
primusdb ai train sales-forecast sales_data --model-type forecasting --target revenue
primusdb ai train classifier user_data --model-type classification --target churned --test-split 0.3
primusdb ai train segmenter customer_data --model-type clustering --params '{"clusters": 5}'
```

Options:

| Option          | Description                               | Default   |
|-----------------|-------------------------------------------|-----------|
| `-t, --model-type` | Model kind (see table above)           | `regression` |
| `-c, --target`  | Target column name                        | —         |
| `--params`      | Hyperparameters as JSON                   | —         |
| `--test-split`  | Fraction of data held out for validation  | `0.2`     |
| `--max-time`    | Maximum training time (seconds)           | `3600`    |

### `primusdb ai predict`

Run inference with a trained model:

```bash
primusdb ai predict sales-forecast '{"month": "2025-06", "region": "US"}'
primusdb ai predict classifier '{"age": 35, "spend": 1200}' --top-k 3
primusdb ai predict anomaly-detector '{"value": 9999}' --raw
```

### `primusdb ai analyze`

Analyse data patterns in a table:

```bash
primusdb ai analyze sales_data
primusdb ai analyze sales_data --columns region,revenue --analysis-type correlation
primusdb ai analyze user_data --analysis-type distribution
```

Analysis types: `summary`, `correlation`, `distribution`, `outliers`, `trend`.

### `primusdb ai anomalies`

Scan a table for anomalous records:

```bash
primusdb ai anomalies metrics
primusdb ai anomalies sensor_data --sensitivity 0.01 --algorithm isolation_forest
```

Algorithms: `zscore`, `isolation_forest`, `mad`, `iqr`.

---

## Architecture

```
AIEngine
├── Model Registry      — in-memory HashMap of trained models
├── Training Pipeline   — fits parameters (weights, bias) from table data
├── Inference Engine    — applies model to input JSON
├── Analytics Engine    — pattern analysis, anomaly detection, forecasting
└── Metrics             — Prometheus counters (predictions, training, anomalies)
```

Training reads data from the specified table via the storage engine, extracts
feature columns, and fits a model.  The current implementation stores models
only in memory — they are not persisted to disk across server restarts.

---

## REST API

| Method | Endpoint                                    | Description          |
|--------|---------------------------------------------|----------------------|
| POST   | `/api/v1/advanced/predict/{storage}/{table}`| Make predictions     |
| POST   | `/api/v1/advanced/analyze/{storage}/{table}`| Analyse data patterns |
| POST   | `/api/v1/advanced/cluster/{storage}/{table}`| Cluster data         |

---

## Comented-Out Dependencies

The `Cargo.toml` has commented-out references to `candle-core` and `candle-nn`
(line 115–116):

```toml
# AI/ML dependencies (lightweight)
# candle-core = "0.3"
# candle-nn = "0.3"
```

These were considered for a future rewrite that would replace the hand-rolled
linear algebra with a proper deep-learning runtime.  They are **not** currently
used — all ML operations are implemented with pure Rust (`ndarray` for vector
math, custom statistics for anomaly detection).

---

## Alpha Limitations

- **Models are not persisted** — after a server restart all trained models are
  lost.
- **No GPU acceleration** — all computation is CPU-only.
- **No hyperparameter tuning** — `--params` is accepted but ignored for most
  model types.
- **No online learning** — models are trained once; incremental updates are not
  supported.
- **No model versioning** — training a new model with the same name overwrites
  the previous one.
